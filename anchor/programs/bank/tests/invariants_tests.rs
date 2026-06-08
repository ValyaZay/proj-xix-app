pub fn bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account_balance: u64, bank_total_deposits: u64) {
    assert!(bank_token_account_balance >= bank_total_deposits)
}

pub fn sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_users_deposit_shares: u64, bank_total_deposit_shares: u64) {
    assert_eq!(sum_of_users_deposit_shares, bank_total_deposit_shares)
}