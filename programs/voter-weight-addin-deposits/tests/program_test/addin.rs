use std::sync::Arc;

use solana_sdk::pubkey::Pubkey;
use solana_sdk::transport::TransportError;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
};
use voter_weight_addin_deposits as addin;

use crate::*;

#[derive(Clone)]
pub struct AddinCookie {
    pub solana: Arc<solana::SolanaCookie>,
    pub program_id: Pubkey,
}

pub struct RegistrarCookie {
    pub address: Pubkey,
    pub mint: MintCookie,
    pub vault: Pubkey,
}

pub struct VoterCookie {
    pub address: Pubkey,
}

impl AddinCookie {
    pub async fn create_registrar(
        &self,
        realm: &GovernanceRealmCookie,
        payer: &Keypair,
    ) -> RegistrarCookie {
        let (registrar, registrar_bump) =
            Pubkey::find_program_address(&[&realm.realm.to_bytes()], &self.program_id);

        let community_token_mint = realm.community_token_mint.pubkey.unwrap();
        let vault = spl_associated_token_account::get_associated_token_address(
            &registrar,
            &community_token_mint,
        );

        let data = anchor_lang::InstructionData::data(&addin::instruction::CreateRegistrar {
            registrar_bump,
        });

        let accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &addin::accounts::CreateRegistrar {
                registrar,
                governance_program_id: realm.governance.program_id,
                realm: realm.realm,
                realm_community_mint: community_token_mint,
                authority: realm.authority,
                vault,
                payer: payer.pubkey(),
                system_program: solana_sdk::system_program::id(),
                token_program: spl_token::id(),
                associated_token_program: spl_associated_token_account::id(),
                rent: solana_program::sysvar::rent::id(),
            },
            None,
        );

        let instructions = vec![Instruction {
            program_id: self.program_id,
            accounts,
            data,
        }];

        // clone the user secret
        let signer = Keypair::from_base58_string(&payer.to_base58_string());

        self.solana
            .process_transaction(&instructions, Some(&[&signer]))
            .await
            .unwrap();

        RegistrarCookie {
            address: registrar,
            mint: realm.community_token_mint,
            vault,
        }
    }

    pub async fn create_voter(
        &self,
        registrar: &RegistrarCookie,
        authority: &Keypair,
        payer: &Keypair,
    ) -> VoterCookie {
        let (voter, voter_bump) = Pubkey::find_program_address(
            &[
                &registrar.address.to_bytes(),
                &authority.pubkey().to_bytes(),
            ],
            &self.program_id,
        );
        let (voter_weight_record, voter_weight_record_bump) = Pubkey::find_program_address(
            &[
                b"voter-weight-record".as_ref(),
                &registrar.address.to_bytes(),
                &authority.pubkey().to_bytes(),
            ],
            &self.program_id,
        );

        let data = anchor_lang::InstructionData::data(&addin::instruction::CreateVoter {
            voter_bump,
            voter_weight_record_bump,
        });

        let accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &addin::accounts::CreateVoter {
                voter,
                voter_weight_record,
                registrar: registrar.address,
                authority: authority.pubkey(),
                payer: payer.pubkey(),
                token_program: spl_token::id(),
                associated_token_program: spl_associated_token_account::id(),
                system_program: solana_sdk::system_program::id(),
                rent: solana_program::sysvar::rent::id(),
                instructions: solana_program::sysvar::instructions::id(),
            },
            None,
        );

        let instructions = vec![Instruction {
            program_id: self.program_id,
            accounts,
            data,
        }];

        // clone the secrets
        let signer1 = Keypair::from_base58_string(&payer.to_base58_string());
        let signer2 = Keypair::from_base58_string(&authority.to_base58_string());

        self.solana
            .process_transaction(&instructions, Some(&[&signer1, &signer2]))
            .await
            .unwrap();

        VoterCookie { address: voter }
    }

    pub async fn deposit(
        &self,
        registrar: &RegistrarCookie,
        voter: &VoterCookie,
        authority: &Keypair,
        token_address: Pubkey,
        amount: u64,
    ) -> std::result::Result<(), TransportError> {
        let data = anchor_lang::InstructionData::data(&addin::instruction::Deposit { amount });

        let accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &addin::accounts::Deposit {
                registrar: registrar.address,
                voter: voter.address,
                vault: registrar.vault,
                deposit_mint: registrar.mint.pubkey.unwrap(),
                deposit_token: token_address,
                authority: authority.pubkey(),
                token_program: spl_token::id(),
            },
            None,
        );

        let instructions = vec![Instruction {
            program_id: self.program_id,
            accounts,
            data,
        }];

        // clone the secrets
        let signer = Keypair::from_base58_string(&authority.to_base58_string());

        self.solana
            .process_transaction(&instructions, Some(&[&signer]))
            .await
    }

    pub async fn withdraw(
        &self,
        registrar: &RegistrarCookie,
        voter: &VoterCookie,
        token_owner_record: &TokenOwnerRecordCookie,
        authority: &Keypair,
        token_address: Pubkey,
        amount: u64,
    ) -> std::result::Result<(), TransportError> {
        let data = anchor_lang::InstructionData::data(&addin::instruction::Withdraw { amount });

        let accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &addin::accounts::Withdraw {
                registrar: registrar.address,
                voter: voter.address,
                token_owner_record: token_owner_record.address,
                vault: registrar.vault,
                withdraw_mint: registrar.mint.pubkey.unwrap(),
                destination: token_address,
                authority: authority.pubkey(),
                token_program: spl_token::id(),
            },
            None,
        );

        let instructions = vec![Instruction {
            program_id: self.program_id,
            accounts,
            data,
        }];

        // clone the secrets
        let signer = Keypair::from_base58_string(&authority.to_base58_string());

        self.solana
            .process_transaction(&instructions, Some(&[&signer]))
            .await
    }
}

impl RegistrarCookie {
    pub async fn vault_balance(&self, solana: &SolanaCookie) -> u64 {
        solana
        .get_account::<TokenAccount>(self.vault)
        .await.amount
    }
}

impl VoterCookie {
    pub async fn deposit_amount(&self, solana: &SolanaCookie) -> u64 {
        solana
        .get_account::<addin::account::Voter>(self.address)
        .await.amount_deposited
    }
}