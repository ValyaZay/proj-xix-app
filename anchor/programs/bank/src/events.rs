use anchor_lang::prelude::*;
use serde::Serialize;

#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct DepositEventJson {
    pub user: String,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

impl From<&DepositEvent> for DepositEventJson {
    fn from(value: &DepositEvent) -> Self {
        DepositEventJson { 
            user: value.user.to_string(), 
            amount: value.amount,
            shares: value.shares, 
            timestamp: value.timestamp,
        }
    }
}