use std::fs::OpenOptions;
use std::io::Write;
use anchor_lang::prelude::*;
use bank::events::{ BankEvent, DepositEvent, EventType, WithdrawEvent };


pub fn record_bank_event<T: BankEvent>(event: &T, step: u8) {
    let mut event_json_model = event.to_json_model();
    event_json_model.step = step;
    let event_string = serde_json::to_string(&event_json_model).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("bank_events_stream.jsonl").unwrap();

    writeln!(file, "{}", event_string).unwrap();

    // test deserialize
    match event_json_model.event_type { //make type an enum?
        EventType::Deposit => {
            println!("decerialise deposit event");
            println!("{:?}", DepositEvent::try_from_slice(&event_json_model.data).unwrap());
        }
        EventType::Withdraw => {
            println!("decerialise withdraw event");
            println!("{:?}", WithdrawEvent::try_from_slice(&event_json_model.data).unwrap());
        }
    };

    // test print
    println!("{:?}", event_json_model);
}


