use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use borsh::de::BorshDeserialize;

use bank::constants::*;
use bank::events::{BankSnapshot, DepositEvent, EventJsonModel, EventType};

fn main() {
    // try using a constant from a bank    
    //println!("MAX_USDC_DEPOSIT = {}", MAX_USDC_DEPOSIT);
    let mut bank = ReplayBank {
        total_deposits: 0,
        total_deposit_shares: 0
    };
    state_replay_events_from_source(&mut bank);
}

fn state_replay_events_from_source(bank: &mut ReplayBank) {
    //println!("Replay events for user {}...", user); //add a user to params
    let file = File::open("programs/bank/bank_events_stream.jsonl").unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        let event_json_model: EventJsonModel = serde_json::from_str(&line).unwrap();
        
        
        // here replay for one user or all and print intermediate result
        match event_json_model.event_type {
            EventType::Deposit => {
                println!("Replaying step {}...", event_json_model.step);
                let event = DepositEvent::try_from_slice(&event_json_model.data).unwrap();
                println!("Deposit {}", event.amount);
                
                bank.total_deposits = bank.total_deposits + event.amount;
                bank.total_deposit_shares = bank.total_deposit_shares + event.shares;
                println!("{:?}", bank);
                println!();
            },
            EventType::Withdraw => {
                println!("Replaying step {}...", event_json_model.step);
                let event = DepositEvent::try_from_slice(&event_json_model.data).unwrap();
                println!("Withdraw {}", event.amount);
               
                bank.total_deposits = bank.total_deposits - event.amount;
                bank.total_deposit_shares = bank.total_deposit_shares - event.shares; 
                println!("{:?}", bank);
                println!();
            },
            EventType::BankSnapshot => {
                println!("Final assert");
                let event = BankSnapshot::try_from_slice(&event_json_model.data).unwrap();
                println!("{:?}", event);
                println!("{:?}", bank);
                assert!(bank.total_deposits == event.total_deposits);
                assert!(bank.total_deposit_shares == event.total_deposit_shares);
            },
        }
    }
}

#[derive(Debug)]
pub struct ReplayBank {
    pub total_deposits: u64,
    pub total_deposit_shares: u64,
}


