use std::fs::OpenOptions;
use std::io::Write;
use anchor_lang::prelude::*;
use serde::Serialize;
//use bank::events::{ DepositEventJson };


pub fn record_deposit_event(event_json_model: &DepositEventJson) {
    let event_string = serde_json::to_string(event_json_model).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("deposit_event_records.jsonl").unwrap();

    writeln!(file, "{}", event_string).unwrap();
}

#[derive(Serialize)]
pub struct DepositEventJson {
    pub user: String,
    pub amount: u64,
    pub shares: u64,
    pub timestamp: i64,
}
