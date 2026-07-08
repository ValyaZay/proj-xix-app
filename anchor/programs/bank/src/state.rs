use anchor_lang::prelude::*;

use crate::{ DISCR_USER_SHARES, DISCR_BANK };

#[account(discriminator = &DISCR_BANK)]
#[derive(InitSpace)]
pub struct Bank {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_deposits: u64,
    pub total_deposit_shares: u64,
}

#[account(discriminator = &DISCR_USER_SHARES)] // PDA contains mint
#[derive(InitSpace)]
pub struct UserShares {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub deposit_shares: u64,
    // borrow_shares - future implementation
}
