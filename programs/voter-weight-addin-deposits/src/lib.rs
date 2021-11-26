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
    pub fn create_registrar(ctx: Context<CreateRegistrar>, registrar_bump: u8) -> Result<()> {
        let registrar = &mut ctx.accounts.registrar.load_init()?;
        registrar.bump = registrar_bump;
        registrar.governance_program_id = ctx.accounts.governance_program_id.key();
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
        // Forbid creating voter accounts from CPI. The goal is to make automation
        // impossible that weakens some of the limitations intentionally imposed on
        // locked tokens.
        {
            use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
            let ixns = ctx.accounts.instructions.to_account_info();
            let current_index = tx_instructions::load_current_index_checked(&ixns)? as usize;
            let current_ixn = tx_instructions::load_instruction_at_checked(current_index, &ixns)?;
            require!(
                current_ixn.program_id == *ctx.program_id,
                ErrorCode::ForbiddenCpi
            );
        }

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
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        // Load accounts.
        let voter = &mut ctx.accounts.voter.load_mut()?;

        voter.amount_deposited += amount;
        voter.last_deposit_slot = Clock::get()?.slot;

        // Deposit tokens into the registrar.
        token::transfer(ctx.accounts.transfer_ctx(), amount)?;

        Ok(())
    }

    /// Withdraws tokens from a deposit entry.
    ///
    /// `amount` is in units of the native currency being withdrawn.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // Load the accounts.
        let registrar = &ctx.accounts.registrar.load()?;
        let voter = &mut ctx.accounts.voter.load_mut()?;

        // Governance may forbid withdraws, for example when engaged in a vote.
        let token_owner = ctx.accounts.authority.key();
        use spl_governance::state::token_owner_record;
        let token_owner_record_address_seeds =
            token_owner_record::get_token_owner_record_address_seeds(
                &registrar.realm,
                &registrar.realm_community_mint,
                &token_owner,
            );
        let token_owner_record_data = token_owner_record::get_token_owner_record_data_for_seeds(
            &registrar.governance_program_id,
            &ctx.accounts.token_owner_record.to_account_info(),
            &token_owner_record_address_seeds,
        )?;
        token_owner_record_data.assert_can_withdraw_governing_tokens()?;

        // Must not withdraw in the same slot as depositing, to prevent people
        // depositing, having the vote weight updated, withdrawing and then
        // voting.
        require!(
            voter.last_deposit_slot < Clock::get()?.slot,
            ErrorCode::InvalidToDepositAndWithdrawInOneSlot
        );

        require!(
            amount <= voter.amount_deposited,
            ErrorCode::InsufficientVestedTokens
        );

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
