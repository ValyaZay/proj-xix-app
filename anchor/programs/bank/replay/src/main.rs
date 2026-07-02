use std::fs::{self, File};
use std::io::{BufRead, BufReader};

use bank::constants::*;
use bank::events::EventJsonModel;

fn main() {
    // try using a constant from a bank    
    //println!("MAX_USDC_DEPOSIT = {}", MAX_USDC_DEPOSIT);
    read_events_source();
}

fn read_events_source() {
    let file = File::open("programs/bank/bank_events_stream.jsonl").unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        let event_json_model: EventJsonModel = serde_json::from_str(&line).unwrap();
        println!("{:?}", event_json_model.event_type);
    }

    // 
}


