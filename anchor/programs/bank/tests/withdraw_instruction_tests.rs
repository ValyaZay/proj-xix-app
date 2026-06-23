use anchor_litesvm::{self, AssertionHelpers, EventHelpers, Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use utils::bank::{
    client::{accounts, args},
    accounts::{User, Bank},
    //events::DepositEvent, //import from idl modules
};

use ::bank::{constants::MIN_USDC_DEPOSIT, convert_assets_to_shares, convert_shares_to_assets, events::WithdrawEvent};//import from external crate

mod utils;
use utils::*;

mod invariants_tests;
use invariants_tests::*;

#[test]
fn withdraw_all_should_update_bank_and_user_states_and_token_accounts_and_emit() {
    // deposit MIN_USDC_DEPOSIT amount -> withdraw MIN_USDC_DEPOSIT amount -> close user acc
    let amount_to_deposit_and_withdraw = MIN_USDC_DEPOSIT;

    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange - depositor
    
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    // Arrange user
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit_and_withdraw).unwrap();

    // Deposit
    let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit_and_withdraw);

    ctx
    .execute_instruction(deposit_inx, &[&depositor])
    .unwrap();

    // state before withdraw
    // ---> bank state
    let before_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(before_withdraw_bank_state.total_deposits, amount_to_deposit_and_withdraw);
    assert_eq!(before_withdraw_bank_state.total_deposit_shares, amount_to_deposit_and_withdraw);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit_and_withdraw);

    // ---> user state
    let before_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(before_withdraw_user_state.deposit_usdc_shares, amount_to_deposit_and_withdraw);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, 0);


    // Withdraw
    let withdraw_accounts = accounts::Withdraw {
        user: depositor.pubkey(),
        user_state: user_state_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: user_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let withdraw_inx = ctx
        .program()
        .accounts(withdraw_accounts)
        .args(args::Withdraw {assets_amount_to_withdraw: amount_to_deposit_and_withdraw})
        .instruction()
        .unwrap();
    let withdraw_result = ctx
        .execute_instruction(withdraw_inx, &[&depositor])
        .unwrap();

    // shares to burn
    let shares_to_burn = convert_assets_to_shares(
        amount_to_deposit_and_withdraw, 
        before_withdraw_bank_state.total_deposit_shares,
        before_withdraw_bank_state.total_deposits,
        true);
    
    // state after withdraw
    // ---> bank state
    let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(after_withdraw_bank_state.total_deposits, before_withdraw_bank_state.total_deposits - amount_to_deposit_and_withdraw);
    assert_eq!(after_withdraw_bank_state.total_deposit_shares, before_withdraw_bank_state.total_deposit_shares - shares_to_burn);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, 0);

    // ---> user state - is closed
    ctx.svm.assert_account_closed(&user_state_pda);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, amount_to_deposit_and_withdraw);

    // Assert - WithdrawEvent
    withdraw_result.assert_event_emitted::<WithdrawEvent>();
    let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
    assert_eq!(withdraw_event.user, depositor.pubkey());
    assert_eq!(withdraw_event.amount, amount_to_deposit_and_withdraw);
    assert_eq!(withdraw_event.shares, shares_to_burn);

    // invariants check
    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, after_withdraw_bank_state.total_deposits);

    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(0, after_withdraw_bank_state.total_deposit_shares);

}


#[test]
fn withdraw_no_dust_remains() {
    // deposit (2 * MIN_USDC_DEPOSIT) amount -> withdraw (MIN_USDC_DEPOSIT + 1) amount -> withdraws all -> close user acc
    let amount_to_deposit = 2 * MIN_USDC_DEPOSIT;
    let amount_to_withdraw = MIN_USDC_DEPOSIT + 1;

    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange - depositor
    
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    // Arrange user
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit).unwrap();

    // Deposit
    let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    ctx
    .execute_instruction(deposit_inx, &[&depositor])
    .unwrap();

    // state before withdraw
    // ---> bank state
    let before_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(before_withdraw_bank_state.total_deposits, amount_to_deposit);
    assert_eq!(before_withdraw_bank_state.total_deposit_shares, amount_to_deposit);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit);

    // ---> user state
    let before_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(before_withdraw_user_state.deposit_usdc_shares, amount_to_deposit);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, 0);


    // Withdraw
    let withdraw_accounts = accounts::Withdraw {
        user: depositor.pubkey(),
        user_state: user_state_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: user_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let withdraw_inx = ctx
        .program()
        .accounts(withdraw_accounts)
        .args(args::Withdraw {assets_amount_to_withdraw: amount_to_withdraw})
        .instruction()
        .unwrap();
    let withdraw_result = ctx
        .execute_instruction(withdraw_inx, &[&depositor])
        .unwrap();

    // state after withdraw
    // ---> bank state
    let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(after_withdraw_bank_state.total_deposits, 0);
    assert_eq!(after_withdraw_bank_state.total_deposit_shares, 0);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, 0);

    // ---> user state - is closed
    ctx.svm.assert_account_closed(&user_state_pda);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, amount_to_deposit);

    // Assert - WithdrawEvent
    withdraw_result.assert_event_emitted::<WithdrawEvent>();
    let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
    assert_eq!(withdraw_event.user, depositor.pubkey());
    assert_eq!(withdraw_event.amount, amount_to_deposit);
    assert_eq!(withdraw_event.shares, before_withdraw_user_state.deposit_usdc_shares);

    // invariants check
    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, after_withdraw_bank_state.total_deposits);

    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(0, after_withdraw_bank_state.total_deposit_shares);

}

#[test]
fn withdraw_but_leave_minimum() {
    // deposit (2 * MIN_USDC_DEPOSIT) amount -> withdraw (MIN_USDC_DEPOSIT - 1) amount -> user shares == MIN_USDC_DEPOSIT -> do not close user acc
    let amount_to_deposit= 2 * MIN_USDC_DEPOSIT;
    let amount_to_withdraw = MIN_USDC_DEPOSIT - 1;

    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange - depositor
    
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    // Arrange user
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit).unwrap();

    // Deposit
    let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    ctx
    .execute_instruction(deposit_inx, &[&depositor])
    .unwrap();

    // state before withdraw
    // ---> bank state
    let before_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(before_withdraw_bank_state.total_deposits, amount_to_deposit);
    assert_eq!(before_withdraw_bank_state.total_deposit_shares, amount_to_deposit);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit);

    // ---> user state
    let before_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(before_withdraw_user_state.deposit_usdc_shares, amount_to_deposit);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, 0);


    // Withdraw
    let withdraw_accounts = accounts::Withdraw {
        user: depositor.pubkey(),
        user_state: user_state_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: user_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let withdraw_inx = ctx
        .program()
        .accounts(withdraw_accounts)
        .args(args::Withdraw {assets_amount_to_withdraw: amount_to_withdraw})
        .instruction()
        .unwrap();
    let withdraw_result = ctx
        .execute_instruction(withdraw_inx, &[&depositor])
        .unwrap();

    // state after withdraw
    // ---> bank state
    let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(after_withdraw_bank_state.total_deposits, before_withdraw_bank_state.total_deposits - amount_to_withdraw);

    let shares_to_burn = convert_assets_to_shares(
        amount_to_withdraw, 
        before_withdraw_bank_state.total_deposit_shares, 
        before_withdraw_bank_state.total_deposits, 
        true);
    assert_eq!(after_withdraw_bank_state.total_deposit_shares, before_withdraw_bank_state.total_deposit_shares - shares_to_burn);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit - amount_to_withdraw);

    // ---> user state - is not closed
    ctx.svm.assert_account_exists(&user_state_pda);
    let after_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(after_withdraw_user_state.deposit_usdc_shares, before_withdraw_user_state.deposit_usdc_shares - shares_to_burn);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, amount_to_withdraw);

    // Assert - WithdrawEvent
    withdraw_result.assert_event_emitted::<WithdrawEvent>();
    let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
    assert_eq!(withdraw_event.user, depositor.pubkey());
    assert_eq!(withdraw_event.amount, amount_to_withdraw);
    assert_eq!(withdraw_event.shares, shares_to_burn);

    // invariants check
    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, after_withdraw_bank_state.total_deposits);

    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(after_withdraw_user_state.deposit_usdc_shares, after_withdraw_bank_state.total_deposit_shares);

}

#[test]
fn withdraw_should_revert_if_zero_amount_to_withdraw() {
    // deposit (2 * MIN_USDC_DEPOSIT) amount -> withdraw (MIN_USDC_DEPOSIT + 1) amount -> withdraws all -> close user acc
    let amount_to_deposit = 2 * MIN_USDC_DEPOSIT;
    let amount_to_withdraw = 0;

    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange - depositor
    
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    // Arrange user
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit).unwrap();

    // Deposit
    let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    ctx
    .execute_instruction(deposit_inx, &[&depositor])
    .unwrap();

    // state before withdraw
    // ---> bank state
    let before_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(before_withdraw_bank_state.total_deposits, amount_to_deposit);
    assert_eq!(before_withdraw_bank_state.total_deposit_shares, amount_to_deposit);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit);

    // ---> user state
    let before_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(before_withdraw_user_state.deposit_usdc_shares, amount_to_deposit);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, 0);


    // Withdraw
    let withdraw_accounts = accounts::Withdraw {
        user: depositor.pubkey(),
        user_state: user_state_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: user_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let withdraw_inx = ctx
        .program()
        .accounts(withdraw_accounts)
        .args(args::Withdraw {assets_amount_to_withdraw: amount_to_withdraw})
        .instruction()
        .unwrap();
    ctx
        .execute_instruction(withdraw_inx, &[&depositor])
        .unwrap()
        .assert_failure()
        .assert_anchor_error("ZeroAmountToWithdraw");

    // state after withdraw
    // ---> bank state
    let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(after_withdraw_bank_state.total_deposits, before_withdraw_bank_state.total_deposits);
    assert_eq!(after_withdraw_bank_state.total_deposit_shares, before_withdraw_bank_state.total_deposit_shares);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit);

    // ---> user state - exists
    ctx.svm.assert_account_exists(&user_state_pda);
    let after_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(after_withdraw_user_state.deposit_usdc_shares, amount_to_deposit);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, 0);

    // invariants check
    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, after_withdraw_bank_state.total_deposits);

    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(after_withdraw_user_state.deposit_usdc_shares, after_withdraw_bank_state.total_deposit_shares);

}

#[test]
fn withdraw_all_via_two_attempts() {
    // deposit (2 * MIN_USDC_DEPOSIT) amount -> withdraw (MIN_USDC_DEPOSIT - 1) amount -> user shares == MIN_USDC_DEPOSIT -> do not close user acc -> withdraw (MIN_USDC_DEPOSIT - 1) amount -> withdraw all -> close user acc
    let amount_to_deposit= 2 * MIN_USDC_DEPOSIT;
    let amount_to_withdraw = MIN_USDC_DEPOSIT - 1;

    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange - depositor
    
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    // Arrange user
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit).unwrap();

    // Deposit
    let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    ctx
    .execute_instruction(deposit_inx, &[&depositor])
    .unwrap();

    // state before withdraw
    // ---> bank state
    let before_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(before_withdraw_bank_state.total_deposits, amount_to_deposit);
    assert_eq!(before_withdraw_bank_state.total_deposit_shares, amount_to_deposit);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit);

    // ---> user state
    let before_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(before_withdraw_user_state.deposit_usdc_shares, amount_to_deposit);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, 0);


    // Withdraw
    let withdraw_accounts = accounts::Withdraw {
        user: depositor.pubkey(),
        user_state: user_state_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: user_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let withdraw_inx = ctx
        .program()
        .accounts(withdraw_accounts)
        .args(args::Withdraw {assets_amount_to_withdraw: amount_to_withdraw})
        .instruction()
        .unwrap();
    let withdraw_result = ctx
        .execute_instruction(withdraw_inx, &[&depositor])
        .unwrap();

    // state after withdraw
    // ---> bank state
    let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(after_withdraw_bank_state.total_deposits, before_withdraw_bank_state.total_deposits - amount_to_withdraw);

    let shares_to_burn = convert_assets_to_shares(
        amount_to_withdraw, 
        before_withdraw_bank_state.total_deposit_shares, 
        before_withdraw_bank_state.total_deposits, 
        true);
    assert_eq!(after_withdraw_bank_state.total_deposit_shares, before_withdraw_bank_state.total_deposit_shares - shares_to_burn);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, amount_to_deposit - amount_to_withdraw);

    // ---> user state - is not closed
    ctx.svm.assert_account_exists(&user_state_pda);
    let after_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(after_withdraw_user_state.deposit_usdc_shares, before_withdraw_user_state.deposit_usdc_shares - shares_to_burn);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, amount_to_withdraw);

    // Assert - WithdrawEvent
    withdraw_result.assert_event_emitted::<WithdrawEvent>();
    let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
    assert_eq!(withdraw_event.user, depositor.pubkey());
    assert_eq!(withdraw_event.amount, amount_to_withdraw);
    assert_eq!(withdraw_event.shares, shares_to_burn);

    // invariants check
    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, after_withdraw_bank_state.total_deposits);

    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(after_withdraw_user_state.deposit_usdc_shares, after_withdraw_bank_state.total_deposit_shares);
    
    // ROLL SLOT AND EXPIRE BLOCKHASH
    ctx.svm.advance_slot(500);
    ctx.svm.expire_blockhash();
    
    // Withdraw 2
    let withdraw_accounts = accounts::Withdraw {
        user: depositor.pubkey(),
        user_state: user_state_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: user_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let withdraw_inx = ctx
        .program()
        .accounts(withdraw_accounts)
        .args(args::Withdraw {assets_amount_to_withdraw: amount_to_withdraw})
        .instruction()
        .unwrap();
    let withdraw_result = ctx
        .execute_instruction(withdraw_inx, &[&depositor])
        .unwrap();

    let actual_assets_to_withdraw = convert_shares_to_assets(
        after_withdraw_user_state.deposit_usdc_shares,
        after_withdraw_bank_state.total_deposit_shares,
        after_withdraw_bank_state.total_deposits);

    // state after withdraw
    // ---> bank state
    let after_withdraw_2_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(after_withdraw_2_bank_state.total_deposits, after_withdraw_bank_state.total_deposits - actual_assets_to_withdraw);
    assert_eq!(after_withdraw_2_bank_state.total_deposit_shares, after_withdraw_bank_state.total_deposit_shares - after_withdraw_user_state.deposit_usdc_shares);

    // ---> bank token account
    ctx.svm.assert_token_balance(&bank_token_account_pda, 0);

    // ---> user state - is closed
    ctx.svm.assert_account_closed(&user_state_pda);

    // ---> user ata state
    ctx.svm.assert_token_balance(&user_ata, amount_to_deposit);

    // Assert - WithdrawEvent
    withdraw_result.assert_event_emitted::<WithdrawEvent>();
    let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
    assert_eq!(withdraw_event.user, depositor.pubkey());
    assert_eq!(withdraw_event.amount, actual_assets_to_withdraw);
    assert_eq!(withdraw_event.shares, after_withdraw_user_state.deposit_usdc_shares);

    // invariants check
    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, after_withdraw_2_bank_state.total_deposits);

    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(0, after_withdraw_2_bank_state.total_deposit_shares);
}