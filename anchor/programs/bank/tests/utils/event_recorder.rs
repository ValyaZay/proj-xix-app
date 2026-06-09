use std::fs::OpenOptions;
use std::io::Write;
use anchor_lang::prelude::*;
use bank::events::{ DepositEventJson, DepositEvent };


pub fn record_deposit_event(deposit_event: &DepositEvent) {
    let event_json_model: DepositEventJson = deposit_event.into();
    let event_string = serde_json::to_string(&event_json_model).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("deposit_event_records.jsonl").unwrap();

    writeln!(file, "{}", event_string).unwrap();
}


