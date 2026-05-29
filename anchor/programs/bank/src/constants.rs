// 1-byte discriminators
pub const DISCR_USER: [u8;1] = [1];
pub const DISCR_BANK: [u8;1] = [2];

// State
pub const SEED_USER_STATE: &[u8] = b"SEED_USER_STATE";
pub const SEED_BANK_STATE: &[u8] = b"SEED_BANK_STATE";
pub const SEED_BANK_TOKEN_ACCOUNT: &[u8] = b"SEED_BANK_TOKEN_ACCOUNT";

// 
pub const MIN_USDC_DEPOSIT: u64 = 10_000_000; // 10 usdc
