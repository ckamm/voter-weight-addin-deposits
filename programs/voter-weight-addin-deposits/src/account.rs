use crate::error::*;
use anchor_lang::prelude::*;
use anchor_spl::vote_weight_record;

// Generate a VoteWeightRecord Anchor wrapper, owned by the current program.
// VoteWeightRecords are unique in that they are defined by the SPL governance
// program, but they are actaully owned by this program.
vote_weight_record!(crate::ID);

/// Instance of a voting rights distributor.
#[account(zero_copy)]
pub struct Registrar {
    pub authority: Pubkey,
    pub realm: Pubkey,
    pub realm_community_mint: Pubkey,
    pub bump: u8,
}

/// User account for minting voting rights.
#[account(zero_copy)]
pub struct Voter {
    pub authority: Pubkey,
    pub registrar: Pubkey,
    pub voter_bump: u8,
    pub voter_weight_record_bump: u8,
    pub amount_deposited: u64,
}

impl Voter {
    pub fn weight(&self) -> Result<u64> {
        Ok(self.amount_deposited)
    }
}

