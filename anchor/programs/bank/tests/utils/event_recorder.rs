use std::fs::OpenOptions;
use std::io::Write;
use anchor_lang::prelude::*;
use bank::events::{ BankEvent, DepositEvent, EventType, WithdrawEvent, BankSnapshot };

pub fn record_bank_event<T: BankEvent>(event: &T, step: u8, date: &str, test_name: &str, seed: u64) {
    let mut event_json_model = event.to_json_model();
    event_json_model.step = step;
    event_json_model.seed = seed;
    let event_string = serde_json::to_string(&event_json_model).unwrap();
    
    let path = format!("test_runs/{test_name}_{date}.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_str()).unwrap();

    writeln!(file, "{}", event_string).unwrap();
}


