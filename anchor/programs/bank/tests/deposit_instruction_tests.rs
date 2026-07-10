
use anchor_litesvm::{ EventHelpers, Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use ::bank::{//import from external crate (not from idl modules)
    events::{DepositEvent},
    constants::{ MIN_USDC_DEPOSIT, MAX_USDC_DEPOSIT},
    shares_math::convert_assets_to_shares,
    Bank,
    UserShares,
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;

use bank_test_utils::*;
use bank_client::client::{accounts, args};

mod invariants_tests;
use invariants_tests::*;

#[test]
fn deposit_should_revert_if_amount_is_less_than_allowed() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    
    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

    // Arrange depositor
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MIN_USDC_DEPOSIT).unwrap();

    init_user_shares_and_assert(&mut ctx, &depositor, &mint);

    let amount_to_deposit = MIN_USDC_DEPOSIT - 1;
    let inx = get_deposit_inx(&mut ctx, &depositor.pubkey(), &mint, &user_ata, amount_to_deposit);

    // Act / Assert
    ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap()
    .assert_failure()
    .assert_anchor_error("NotEnoughAmountToDeposit");
    
}

#[test]
fn deposit_should_revert_if_amount_is_more_than_allowed() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    
    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

    // Arrange depositor
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MAX_USDC_DEPOSIT).unwrap();

    init_user_shares_and_assert(&mut ctx, &depositor, &mint);

    let amount_to_deposit = MAX_USDC_DEPOSIT + 1;
    let inx = get_deposit_inx(&mut ctx, &depositor.pubkey(), &mint, &user_ata, amount_to_deposit);

    // Act / Assert
    ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap()
    .assert_failure()
    .assert_anchor_error("TooBigAmountToDeposit");
}

#[test]
fn deposit_should_update_bank_and_user_shares_and_token_accounts_and_emit() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

    // Arrange - depositor
    let amount_to_deposit = MIN_USDC_DEPOSIT;
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    init_user_shares_and_assert(&mut ctx, &depositor, &mint);

    let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), &mint);
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit * 2).unwrap();

    let depositor_sol_balance_before = ctx.svm.get_balance(&depositor.pubkey()).unwrap();

    let (transaction_result, deposited_amount, shares_to_mint) = process_deposit_and_assert_states(&mut ctx, &user_shares_pda, &depositor, mint, &user_ata, amount_to_deposit).unwrap();

    // Assert - DepositEvent - do not record!!!
    // let shares_to_be_added_from_amount = convert_assets_to_shares(amount_to_deposit, init_total_shares, init_total_assets, false);
    // result.assert_event_emitted::<DepositEvent>();
    // let deposit_event: DepositEvent = result.parse_event().unwrap();
    // assert_eq!(deposit_event.user, depositor.pubkey());
    // assert_eq!(deposit_event.amount, amount_to_deposit);
    // assert_eq!(deposit_event.shares, shares_to_be_added_from_amount);

    

    // Assert - fees are paid by the depositor
    let tx_fee_to_validator = &transaction_result.inner().fee;
    let depositor_sol_balance_after = ctx.svm.get_balance(&depositor.pubkey()).unwrap();
    assert_eq!(depositor_sol_balance_after, depositor_sol_balance_before - tx_fee_to_validator);

    // invariant check
    let bank_token_account_pda = get_bank_token_account_pda(&mint);
    let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    let bank_pda = get_bank_account_pda(&mint);
    let bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, bank_state.total_deposits);

    let user_shares: UserShares = ctx.get_account(&user_shares_pda).unwrap();
    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(user_shares.deposit_shares, bank_state.total_deposit_shares);
}


#[test]
fn deposit_should_revert_if_user_is_not_user_shares_owner() {
    // it should not be possible to sign tx for other user state because there is a contstrain in seeds - user is involved, which is a signer as well
    
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(&mint);
    let bank_token_account_pda = get_bank_token_account_pda(&mint);

    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

    // Arrange - depositor
    let amount_to_deposit = MIN_USDC_DEPOSIT;
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    
    let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), &mint);
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit * 2).unwrap();

    init_user_shares_and_assert(&mut ctx, &depositor, &mint);
    
    // Arrange - strange depositor
    let strange_depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let strange_depositor_ata = ctx.svm.create_associated_token_account(&mint, &strange_depositor).unwrap();
    ctx.svm.mint_to(&mint, &strange_depositor_ata, &mint_authority, amount_to_deposit * 2).unwrap();

    // Act
    // 1. deposit for the depositor
    let inx = get_deposit_inx(&mut ctx, &depositor.pubkey(), &mint, &user_ata, amount_to_deposit);

    ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap();

    // 2. deposit for another depositor's account - a signer is different
    let deposit_accounts = accounts::Deposit {
        user: strange_depositor.pubkey(),
        user_shares: user_shares_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: strange_depositor_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };

    let inx = ctx
        .program()
        .accounts(deposit_accounts)
        .args(args::Deposit { amount: amount_to_deposit })
        .instruction()
        .unwrap();

    ctx
    .execute_instruction(inx, &[&strange_depositor])
    .unwrap()
    .assert_failure()
    .assert_anchor_error("ConstraintSeeds");
}
