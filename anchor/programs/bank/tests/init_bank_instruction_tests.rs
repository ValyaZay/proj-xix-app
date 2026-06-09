
use anchor_litesvm::{ Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use solana_sdk::native_token::LAMPORTS_PER_SOL;

mod utils;
use utils::*;

mod invariants_tests;
use invariants_tests::*;

use test_env_utils::bank::{
    accounts::{Bank},
    //events::DepositEvent, //import from idl modules
};


#[test]
fn should_init_bank() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, _) = get_mint_pubkey_and_authority(&mut ctx);

    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);

    // Act
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Assert
    let bank_account: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(bank_account.authority, bank_authority.pubkey());
    assert_eq!(bank_account.mint, mint);
    assert_eq!(bank_account.total_deposits, 0);
    assert_eq!(bank_account.total_deposit_shares, 0);

    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    assert_eq!(bank_token_account.mint, mint);
    assert_eq!(bank_token_account.amount, 0);

    // invariant check
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, bank_account.total_deposits);
}