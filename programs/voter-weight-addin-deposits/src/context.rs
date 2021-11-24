use crate::account::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use std::mem::size_of;

pub const VOTER_WEIGHT_RECORD: [u8; 19] = *b"voter-weight-record";

#[derive(Accounts)]
#[instruction(registrar_bump: u8)]
pub struct CreateRegistrar<'info> {
    #[account(
        init,
        seeds = [realm.key().as_ref()],
        bump = registrar_bump,
        payer = payer,
        space = 8 + size_of::<Registrar>()
    )]
    pub registrar: AccountLoader<'info, Registrar>,
    pub governance_program_id: AccountInfo<'info>,
    // Unsafe and untrusted. This instruction needs to be invoked immediatley
    // after the realm is created.
    pub realm: UncheckedAccount<'info>,
    pub realm_community_mint: Account<'info, Mint>,
    pub authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        associated_token::authority = registrar,
        associated_token::mint = deposit_mint,
    )]
    pub exchange_vault: Account<'info, TokenAccount>,
    pub deposit_mint: Account<'info, Mint>,

    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(voter_bump: u8, voter_weight_record_bump: u8)]
pub struct CreateVoter<'info> {
    #[account(
        init,
        seeds = [registrar.key().as_ref(), authority.key().as_ref()],
        bump = voter_bump,
        payer = authority,
        space = 8 + size_of::<Voter>(),
    )]
    pub voter: AccountLoader<'info, Voter>,

    #[account(
        init,
        seeds = [VOTER_WEIGHT_RECORD.as_ref(), registrar.key().as_ref(), authority.key().as_ref()],
        bump = voter_weight_record_bump,
        payer = payer,
        space = 150,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    pub registrar: AccountLoader<'info, Registrar>,
    pub authority: Signer<'info>,
    pub payer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    #[account(mut, has_one = authority, has_one = registrar)]
    pub voter: AccountLoader<'info, Voter>,

    #[account(
        mut,
        associated_token::authority = registrar,
        associated_token::mint = deposit_mint,
    )]
    pub exchange_vault: Account<'info, TokenAccount>,
    pub deposit_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = deposit_token.mint == deposit_mint.key(),
    )]
    pub deposit_token: Account<'info, TokenAccount>,

    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Deposit<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.deposit_token.to_account_info(),
            to: self.exchange_vault.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    #[account(mut, has_one = registrar, has_one = authority)]
    pub voter: AccountLoader<'info, Voter>,

    pub token_owner_record: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::authority = registrar,
        associated_token::mint = withdraw_mint,
    )]
    pub exchange_vault: Account<'info, TokenAccount>,
    pub withdraw_mint: Account<'info, Mint>,

    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.exchange_vault.to_account_info(),
            to: self.destination.to_account_info(),
            authority: self.registrar.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

#[derive(Accounts)]
pub struct UpdateVoterWeightRecord<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    #[account(
        has_one = registrar,
        has_one = authority,
    )]
    pub voter: AccountLoader<'info, Voter>,

    #[account(
        mut,
        seeds = [VOTER_WEIGHT_RECORD.as_ref(), registrar.key().as_ref(), authority.key().as_ref()],
        bump = voter.load()?.voter_weight_record_bump,
        constraint = voter_weight_record.realm == registrar.load()?.realm,
        constraint = voter_weight_record.governing_token_owner == voter.load()?.authority,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseVoter<'info> {
    #[account(mut, has_one = authority, close = sol_destination)]
    pub voter: AccountLoader<'info, Voter>,
    pub authority: Signer<'info>,
    pub sol_destination: UncheckedAccount<'info>,
}
