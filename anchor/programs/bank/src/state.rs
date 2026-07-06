use anchor_lang::prelude::*;

use crate::{ DISCR_USER, DISCR_BANK };

#[account(discriminator = &DISCR_USER)]
#[derive(InitSpace)]
pub struct User {
    pub user: Pubkey,
    pub deposit_usdc_shares: u64,
}

#[account(discriminator = &DISCR_BANK)]
#[derive(InitSpace)]
pub struct Bank {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_deposits: u64,
    pub total_deposit_shares: u64,
}

