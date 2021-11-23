use anchor_lang::prelude::*;
use anchor_spl::token;
use context::*;
use error::*;
use spl_governance::addins::voter_weight::VoterWeightAccountType;

mod account;
mod context;
mod error;

// The program address.
declare_id!("HoVX43xherfXV6RUoLmcoLA1XSyd9SbF4V1Edpi2QbLY");

/// # Introduction
///
/// The voter-weight-addin-deposits is an "addin" to the SPL governance program
/// that allows one to deposit tokens and to vote with them. The behavior is
/// intended to be exactly identical to the spl-governance without an addin.
///
/// The flow for voting with this program is as follows:
///
/// - Create a SPL governance realm.
/// - Create a governance registry account.
/// - Create a voter account.
/// - Deposit tokens into this program.
/// - Vote.
///
/// Upon voting with SPL governance, a client is expected to call
/// `update_voter_weight_record` to get an up to date measurement of a given
/// `Voter`'s voting power for the given slot. If this is not done, then the
/// transaction will fail (since the SPL governance program will require the
/// measurement to be active for the current slot).
///
/// # Interacting with SPL Governance
///
/// This program does not directly interact with SPL governance via CPI.
/// Instead, it simply writes a `VoterWeightRecord` account with a well defined
/// format, which is then used by SPL governance as the voting power measurement
/// for a given user.
///
#[program]
pub mod voter_weight_addin_deposits {
    use super::*;

    /// Creates a new voting registrar. There can only be a single regsitrar
    /// per governance realm.
    pub fn create_registrar(
        ctx: Context<CreateRegistrar>,
        registrar_bump: u8,
    ) -> Result<()> {
        let registrar = &mut ctx.accounts.registrar.load_init()?;
        registrar.bump = registrar_bump;
        registrar.realm = ctx.accounts.realm.key();
        registrar.realm_community_mint = ctx.accounts.realm_community_mint.key();
        registrar.authority = ctx.accounts.authority.key();

        Ok(())
    }

    /// Creates a new voter account. There can only be a single voter per
    /// user wallet.
    pub fn create_voter(
        ctx: Context<CreateVoter>,
        voter_bump: u8,
        voter_weight_record_bump: u8,
    ) -> Result<()> {
        // Load accounts.
        let registrar = &ctx.accounts.registrar.load()?;
        let voter = &mut ctx.accounts.voter.load_init()?;
        let voter_weight_record = &mut ctx.accounts.voter_weight_record;

        // Init the voter.
        voter.voter_bump = voter_bump;
        voter.voter_weight_record_bump = voter_weight_record_bump;
        voter.authority = ctx.accounts.authority.key();
        voter.registrar = ctx.accounts.registrar.key();

        // Init the voter weight record.
        voter_weight_record.account_type = VoterWeightAccountType::VoterWeightRecord;
        voter_weight_record.realm = registrar.realm;
        voter_weight_record.governing_token_mint = registrar.realm_community_mint;
        voter_weight_record.governing_token_owner = ctx.accounts.authority.key();

        Ok(())
    }

    /// Creates a new deposit entry and updates it by transferring in tokens.
    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
    ) -> Result<()> {
        // Load accounts.
        let voter = &mut ctx.accounts.voter.load_mut()?;

        voter.amount_deposited += amount;

        // Deposit tokens into the registrar.
        token::transfer(ctx.accounts.transfer_ctx(), amount)?;

        Ok(())
    }

    /// Withdraws tokens from a deposit entry.
    ///
    /// `amount` is in units of the native currency being withdrawn.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // TODO:
        // 1. Do not allow withdraws while the voter has voted on something!
        // 2. Do not allow withdraws in the same slot as deposits!

        // Load the accounts.
        let registrar = &ctx.accounts.registrar.load()?;
        let voter = &mut ctx.accounts.voter.load_mut()?;

        require!(amount <= voter.amount_deposited, ErrorCode::InsufficientVestedTokens);

        // Update deposit book keeping.
        voter.amount_deposited -= amount;

        // Transfer the tokens to withdraw.
        token::transfer(
            ctx.accounts
                .transfer_ctx()
                .with_signer(&[&[registrar.realm.as_ref(), &[registrar.bump]]]),
            amount,
        )?;

        Ok(())
    }

    /// Calculates the voting power for the given voter (exactly the number
    /// of deposited tokens) and writes it into a `VoteWeightRecord` account
    /// to be used by the SPL governance program.
    ///
    /// This "revise" instruction should be called in the same transaction,
    /// immediately before voting.
    pub fn update_voter_weight_record(ctx: Context<UpdateVoterWeightRecord>) -> Result<()> {
        let voter = ctx.accounts.voter.load()?;
        let record = &mut ctx.accounts.voter_weight_record;
        record.voter_weight = voter.weight()?;
        record.voter_weight_expiry = Some(Clock::get()?.slot);

        Ok(())
    }

    /// Closes the voter account, allowing one to retrieve rent exemption SOL.
    /// Only accounts with no remaining deposits can be closed.
    pub fn close_voter(ctx: Context<CloseVoter>) -> Result<()> {
        let voter = &ctx.accounts.voter.load()?;
        require!(voter.amount_deposited == 0, VotingTokenNonZero);
        Ok(())
    }
}
