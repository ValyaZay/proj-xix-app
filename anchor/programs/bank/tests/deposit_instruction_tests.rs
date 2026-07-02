
use anchor_litesvm::{ AccountError, EventHelpers, Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use ::bank::{//import from external crate (not from idl modules)
    events::{DepositEvent},
    constants::{ MIN_USDC_DEPOSIT, MAX_USDC_DEPOSIT},
    shares_math::convert_assets_to_shares,
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;

mod utils;
use utils::*;

mod invariants_tests;
use invariants_tests::*;

use utils::bank::{
    accounts::{User, Bank},
    //events::DepositEvent, //import from idl modules
};


#[test]
fn deposit_should_revert_if_amount_is_less_than_allowed() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange depositor
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MIN_USDC_DEPOSIT).unwrap();

    let amount_to_deposit = MIN_USDC_DEPOSIT - 1;
    let inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

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
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange depositor
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MAX_USDC_DEPOSIT).unwrap();

    let amount_to_deposit = MAX_USDC_DEPOSIT + 1;
    let inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    // Act / Assert
    ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap()
    .assert_failure()
    .assert_anchor_error("TooBigAmountToDeposit");
}

#[test]
fn deposit_should_update_bank_and_user_states_and_token_accounts_and_emit() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);
    let init_bank_state:Bank = ctx.get_account(&bank_pda).unwrap();
    let init_total_assets = init_bank_state.total_deposits;
    let init_total_shares = init_bank_state.total_deposit_shares;

    let init_bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    let init_bank_token_account_balance = init_bank_token_account.amount;

    // Arrange - depositor
    let amount_to_deposit = MIN_USDC_DEPOSIT;
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    //let depositor_sol_account_balance_init = ctx.svm.get_balance(&depositor.pubkey()).unwrap();

    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit * 2).unwrap();
    let user_ata_account: TokenAccount = ctx.get_account(&user_ata).unwrap();
    let init_user_ata_balance = user_ata_account.amount;
    assert_eq!(init_user_ata_balance, amount_to_deposit * 2);

    let depositor_sol_account_balance_init = ctx.svm.get_balance(&depositor.pubkey()).unwrap();
    println!("depositor_sol_account_balance_init {}", depositor_sol_account_balance_init);
    
    let init_user_state = match ctx.get_account::<User>(&user_state_pda) {
        Ok(account) => account,
        Err(error) => {
            match error {
                AccountError::AccountNotFound(_) => {
                    User::default()
                }
                _ => panic!("Some problem when getting user account!")
            }
        }
    };
    let init_user_deposit_usdc_shares = init_user_state.deposit_usdc_shares;

    // Act
    let inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    let result = ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap();

    // Assert - DepositEvent
    let shares_to_be_added_from_amount = convert_assets_to_shares(amount_to_deposit, init_total_shares, init_total_assets, false);
    result.assert_event_emitted::<DepositEvent>();
    let deposit_event: DepositEvent = result.parse_event().unwrap();
    assert_eq!(deposit_event.user, depositor.pubkey());
    assert_eq!(deposit_event.amount, amount_to_deposit);
    assert_eq!(deposit_event.shares, shares_to_be_added_from_amount);

    // Assert - BankState
    let bank_state_updated:Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(bank_state_updated.total_deposits, amount_to_deposit + init_total_assets);
    assert_eq!(bank_state_updated.total_deposit_shares, shares_to_be_added_from_amount + init_total_shares);
    
    // Assert - UserState
    let user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(user_state.deposit_usdc_shares, init_user_deposit_usdc_shares + shares_to_be_added_from_amount);

    // Assert - BankTokenAccount
    let bank_token_account_updated: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    assert_eq!(bank_token_account_updated.amount, init_bank_token_account_balance + amount_to_deposit);

    // Assert - User ATA
    let user_ata_account_updated: TokenAccount = ctx.get_account(&user_ata).unwrap();
    assert_eq!(user_ata_account_updated.amount, init_user_ata_balance - amount_to_deposit);

    // Assert - fees are paid by the depositor
    let tx_fee_to_validator = &result.inner().fee;
    let sol_user_state_balance = ctx.svm.get_balance(&user_state_pda).unwrap(); // user state account was created during deposit inx
    let depositor_sol_account_balance_updated = ctx.svm.get_balance(&depositor.pubkey()).unwrap();
    assert_eq!(depositor_sol_account_balance_init, depositor_sol_account_balance_updated + tx_fee_to_validator + sol_user_state_balance);

    // invariant check
    bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account_updated.amount, bank_state_updated.total_deposits);
    sum_of_users_deposit_shares_equals_bank_total_deposit_shares(user_state.deposit_usdc_shares, bank_state_updated.total_deposit_shares);
}


#[test]
fn deposit_should_revert_if_user_is_not_user_state_owner() {
    // it should not be possible to sign tx for other user state because there is a contstrain in seeds - user is involved, which is a signer as well
    
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    let bank_token_account_pda = get_bank_token_account_pda(mint);

    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    // Arrange - depositor
    let amount_to_deposit = MIN_USDC_DEPOSIT;
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit * 2).unwrap();
    
    // Arrange - strange depositor
    let strange_depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let strange_depositor_ata = ctx.svm.create_associated_token_account(&mint, &strange_depositor).unwrap();
    ctx.svm.mint_to(&mint, &strange_depositor_ata, &mint_authority, amount_to_deposit * 2).unwrap();

    // Act
    // 1. deposit and create acc for the depositor
    let inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap();

    // 2. deposit for another depositor's account - a signer is different
    let inx = get_deposit_inx(&mut ctx, &user_state_pda, &strange_depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &strange_depositor_ata, amount_to_deposit);

    ctx
    .execute_instruction(inx, &[&strange_depositor])
    .unwrap()
    .assert_failure()
    .assert_anchor_error("ConstraintSeeds");
}
