use anchor_lang::prelude::*;
use serde::{Deserialize, Serialize};

#[event]
#[derive(Debug)]
pub struct DepositEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

#[event]
#[derive(Debug)]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}

impl From<&DepositEvent> for EventJsonModel {
    fn from(value: &DepositEvent) -> Self {
        let data = borsh::to_vec(&value).unwrap();
        EventJsonModel { 
            step: 0,
            seed: 0,
            event_type: EventType::Deposit,
            tx_id: String::from(""),
            timestamp: value.timestamp,
            user: value.user.to_string(), 
            data: data,
        }
    }
}

impl From<&WithdrawEvent> for EventJsonModel {
    fn from(value: &WithdrawEvent) -> Self {
        let data = borsh::to_vec(&value).unwrap();
        EventJsonModel { 
            step: 0,
            seed: 0,
            event_type: EventType::Withdraw,
            tx_id: String::from(""),
            timestamp: value.timestamp,
            user: value.user.to_string(), 
            data: data,
        }
    }
}

impl From<&BankSnapshot> for EventJsonModel {
    fn from(value: &BankSnapshot) -> Self {
        let data = borsh::to_vec(&value).unwrap();
        EventJsonModel { 
            step: 0,
            seed: 0,
            event_type: EventType::BankSnapshot,
            tx_id: String::from(""),
            timestamp: value.timestamp,
            user: value.user.to_string(), 
            data: data,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EventJsonModel {
    pub step: u8,
    pub seed: u64, // for randomized test
    pub event_type: EventType,
    pub tx_id: String,
    pub timestamp: i64,
    pub user: String,
    pub data: Vec<u8>,
}

pub trait BankEvent {
    fn to_json_model(&self) -> EventJsonModel;
}

impl BankEvent for DepositEvent {
    fn to_json_model(&self) -> EventJsonModel { //pass step here
        self.into()
    }
}

impl BankEvent for WithdrawEvent {
    fn to_json_model(&self) -> EventJsonModel { //pass step here
        self.into()
    }
}

impl BankEvent for BankSnapshot {
    fn to_json_model(&self) -> EventJsonModel { //pass step here
        self.into()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EventType {
    Deposit,
    Withdraw,
    BankSnapshot
}


// shapshots
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct BankSnapshot {
    pub user: Pubkey,
    pub total_deposits: u64,
    pub total_deposit_shares: u64,
    pub timestamp: i64,
}

