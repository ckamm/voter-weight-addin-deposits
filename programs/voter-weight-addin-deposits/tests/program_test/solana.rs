use std::cell::RefCell;

use anchor_lang::AccountDeserialize;
use solana_program::{program_pack::Pack, rent::*, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::ReadableAccount,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use spl_token::*;

pub struct SolanaCookie {
    pub context: RefCell<ProgramTestContext>,
    pub rent: Rent,
}

impl SolanaCookie {
    #[allow(dead_code)]
    pub async fn process_transaction(
        &self,
        instructions: &[Instruction],
        signers: Option<&[&Keypair]>,
    ) -> Result<(), TransportError> {
        let mut context = self.context.borrow_mut();

        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&context.payer.pubkey()));

        let mut all_signers = vec![&context.payer];

        if let Some(signers) = signers {
            all_signers.extend_from_slice(signers);
        }

        // This fails when warping is involved - https://gitmemory.com/issue/solana-labs/solana/18201/868325078
        // let recent_blockhash = self.context.banks_client.get_recent_blockhash().await.unwrap();

        transaction.sign(&all_signers, context.last_blockhash);

        context
            .banks_client
            .process_transaction_with_commitment(
                transaction,
                solana_sdk::commitment_config::CommitmentLevel::Processed,
            )
            .await
    }

    #[allow(dead_code)]
    pub async fn create_token_account(&self, owner: &Pubkey, mint: Pubkey) -> Pubkey {
        let keypair = Keypair::new();
        let rent = self.rent.minimum_balance(spl_token::state::Account::LEN);

        let instructions = [
            system_instruction::create_account(
                &self.context.borrow().payer.pubkey(),
                &keypair.pubkey(),
                rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &keypair.pubkey(),
                &mint,
                owner,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[&keypair]))
            .await
            .unwrap();
        return keypair.pubkey();
    }

    pub async fn get_account_data(&self, address: Pubkey) -> Vec<u8> {
        self.context
            .borrow_mut()
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap()
            .data()
            .to_vec()
    }

    pub async fn get_account<T: AccountDeserialize>(&self, address: Pubkey) -> T {
        let data = self.get_account_data(address).await;
        let mut data_slice: &[u8] = &data;
        AccountDeserialize::try_deserialize(&mut data_slice).unwrap()
    }
}
