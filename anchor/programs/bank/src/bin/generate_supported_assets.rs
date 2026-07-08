use std::fs::OpenOptions;
use std::io::Write;
use anchor_lang::prelude::*;
use bank::{ MINT_USDC_MAINNET, MINT_WRAPPED_BTC_MAINNET };
use serde::{Deserialize, Serialize};

pub fn main() {
    set_supported_assets(Pubkey::from_str_const(MINT_USDC_MAINNET), String::from("USDC"));
    set_supported_assets(Pubkey::from_str_const(MINT_WRAPPED_BTC_MAINNET), String::from("BTC"));
}

fn set_supported_assets(mint: Pubkey, symbol: String) {
    println!("set config: mint {}, symbol {}", mint, symbol);
    
    let asset = SupportedAsset {
        mint: mint,
        symbol: symbol,
    };
    let asset_string = serde_json::to_string(&asset).unwrap();

    let path = format!("programs/bank/src/bin/supported_assets.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_str()).unwrap();

    writeln!(file, "{}", asset_string).unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SupportedAsset {
    pub mint: Pubkey,
    pub symbol: String,
}