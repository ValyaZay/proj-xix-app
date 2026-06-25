use anchor_lang::solana_program::clock::Clock;
use anchor_litesvm::{ AccountError, EventHelpers, Signer, TestHelpers, AssertionHelpers};
use anchor_spl::{token::Token, token_interface::TokenAccount};
use ::bank::{//import from external crate (not from idl modules)
    events::{DepositEvent, WithdrawEvent},
    constants::{ MIN_USDC_DEPOSIT, MAX_USDC_DEPOSIT },
    shares_math::{convert_shares_to_assets, convert_assets_to_shares},
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use rand::RngExt;

mod utils;
use utils::*;

mod invariants_tests;
use invariants_tests::*;

use utils::bank::{
    client::{accounts, args},
    accounts::{User, Bank},
    //events::DepositEvent, //import from idl modules
};

//TODO randomize amount to deposit, add user checks

#[test]
fn deposits_in_raw_should_update_state() {
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
    
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, u64::MAX).unwrap();

    // Arrange - INIT STATE FOR THE BANK
    let init_bank_state:Bank = ctx.get_account(&bank_pda).unwrap();
    let mut init_total_assets = init_bank_state.total_deposits;
    let mut init_total_shares = init_bank_state.total_deposit_shares;

    let init_bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    let mut init_bank_token_account_balance = init_bank_token_account.amount;

    // Arrange - INIT STATE FOR THE USER
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
    let mut init_user_deposit_usdc_shares = init_user_state.deposit_usdc_shares;

    let user_ata_account: TokenAccount = ctx.get_account(&user_ata).unwrap();
    let mut init_user_ata_balance = user_ata_account.amount;

    let mut num = 100;
    let mut rng = rand::rng();
    let mut clock: Clock = ctx.svm.get_sysvar();
    // Act
    // 1. deposit -> RECORD EVENT -> state -> invariants check -> roll slot    
    while num != 0 {
        let amount_to_deposit: u64 = rng.random_range(MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT);

        let inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

        let slot = ctx.svm.get_sysvar::<Clock>().slot;
        println!("slot {}", slot );
        println!("blockhash {}", ctx.latest_blockhash());

        println!("starter init_total_assets {}, num {}", init_total_assets, num);
        println!("starter init_total_shares {}, num {}", init_total_shares, num);
        
        // shares
         let shares_to_be_added_from_amount = convert_assets_to_shares(amount_to_deposit, init_total_shares, init_total_assets, false);
        
        // 1. DEPOSIT
        let result = ctx
        .execute_instruction(inx.clone(), &[&depositor])
        .unwrap();
        println!("result {:?}", result);

        // 2. RECORD EVENT
        result.assert_event_emitted::<DepositEvent>();
        let deposit_event: DepositEvent = result.parse_event().unwrap();
        record_deposit_event(&deposit_event);

        // 3. CHECK BANK STATE
        // bank state
        let updated_bank_state:Bank = ctx.get_account(&bank_pda).unwrap();
        println!("updated_bank_state.total_deposits {}, num {}", updated_bank_state.total_deposits, num);
        assert_eq!(updated_bank_state.total_deposits, amount_to_deposit + init_total_assets);
        assert_eq!(updated_bank_state.total_deposit_shares, shares_to_be_added_from_amount + init_total_shares);
        init_total_assets = updated_bank_state.total_deposits;
        init_total_shares = updated_bank_state.total_deposit_shares;
        println!("init_total_assets {}, num {}", init_total_assets, num);
        println!("init_total_shares {}, num {}", init_total_shares, num);

        // bank token account
        let updated_bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
        assert_eq!(updated_bank_token_account.amount, amount_to_deposit + init_bank_token_account_balance);
        init_bank_token_account_balance = updated_bank_token_account.amount;

        // 4. CHECK USER STATE
        // check user
        let updated_user_state: User = ctx.get_account(&user_state_pda).unwrap();
        assert_eq!(updated_user_state.deposit_usdc_shares, init_user_deposit_usdc_shares + shares_to_be_added_from_amount);
        init_user_deposit_usdc_shares = updated_user_state.deposit_usdc_shares;

        // check user ata
        let updated_user_ata_account_updated: TokenAccount = ctx.get_account(&user_ata).unwrap();
        assert_eq!(updated_user_ata_account_updated.amount, init_user_ata_balance - amount_to_deposit);
        init_user_ata_balance = updated_user_ata_account_updated.amount;

        // 5. CHECK INVARIANTS
        bank_token_account_balance_not_less_than_bank_total_deposits(updated_bank_token_account.amount, updated_bank_state.total_deposits);

        sum_of_users_deposit_shares_equals_bank_total_deposit_shares(updated_user_state.deposit_usdc_shares, updated_bank_state.total_deposit_shares);

        // 6. ROLL SLOT AND EXPIRE BLOCKHASH
        ctx.svm.advance_slot(500);
        ctx.svm.expire_blockhash();

        // set timestamp for event record
        clock.unix_timestamp = clock.unix_timestamp + 500 * 400 / 1000;
        ctx.svm.set_sysvar(&clock);
        
        num -= 1;
    }
}

#[test]
fn deposit_withdraw_should_update_state() {
    // diagnostic variables:
    let mut attempts_to_withdraw_more_than_deposit = 0;
    let mut attempts_to_withdraw_all = 0;
    let mut attempts_to_withdraw_amount = 0;

    // Arrange
    let mut ctx = init_anchor_ctx();
    let mut num = 100;
    let mut rng = rand::rng();
    let mut clock: Clock = ctx.svm.get_sysvar();
    
    // Arrange mint
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    println!("Bank address {}", bank_pda);
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    println!("Bank token acc {}", bank_token_account_pda);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    let mut final_bank_total_deposits = 0;
    let mut final_bank_total_shares= 0;
    let mut final_bank_balance= 0;
    let mut sum_of_user_shares = 0;

     // Act
    /*
        1 bank -> 100 users
        deposit randX = (MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT) amount -> withdraw randY = rand(0..MAX_USDC_DEPOSIT) amount ->
                    ---> if user's assets < randY + MIN_USDC_DEPOSIT -> withdraw all -> close user acc
                    ---> else -> withdraw randY -> user acc exists
    */
    while num != 0 {
        let amount_to_deposit: u64 = rng.random_range(MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT);
        let amount_to_withdraw: u64 = rng.random_range(0..=MAX_USDC_DEPOSIT);// don't limit amount to withdraw by amount to deposit - the cases what the user wants more than has should be included too!!!
        
        println!("-----------------------------");
        println!("num: {}, amount_to_deposit: {}, amount_to_withdraw: {}", num, amount_to_deposit, amount_to_withdraw);
        if amount_to_deposit < amount_to_withdraw {
            attempts_to_withdraw_more_than_deposit += 1;
        }

        // Arrange - depositor
        let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
        println!("User address {}", depositor.pubkey());

        // Arrange user
        let user_state_pda = get_user_account_pda(depositor.pubkey());
        let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
        println!("user ata {}", user_ata);
        ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit).unwrap();

        // Deposit
        let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

        ctx
        .execute_instruction(deposit_inx, &[&depositor])
        .unwrap();

        // state before withdraw
        // ---> bank state
        let before_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
        assert_eq!(before_withdraw_bank_state.total_deposits, final_bank_total_deposits + amount_to_deposit);
        assert_eq!(before_withdraw_bank_state.total_deposit_shares, final_bank_total_shares + amount_to_deposit);

        // ---> bank token account
        let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
        let bank_token_balance = bank_token_account.amount;
        ctx.svm.assert_token_balance(&bank_token_account_pda, bank_token_balance);

        // ---> user state
        let before_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
        let actual_assets_user_has = convert_shares_to_assets(
            before_withdraw_user_state.deposit_usdc_shares,
            before_withdraw_bank_state.total_deposit_shares,
            before_withdraw_bank_state.total_deposits
        );
        assert_eq!(before_withdraw_user_state.deposit_usdc_shares, amount_to_deposit);

        // ---> user ata state
        ctx.svm.assert_token_balance(&user_ata, 0);

        // ROLL SLOT AND EXPIRE BLOCKHASH
        ctx.svm.advance_slot(500);
        ctx.svm.expire_blockhash();
        // set timestamp for event record
        clock.unix_timestamp = clock.unix_timestamp + 500 * 400 / 1000;
        ctx.svm.set_sysvar(&clock);

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
        if actual_assets_user_has < (amount_to_withdraw + MIN_USDC_DEPOSIT) {
            // withdraw all
            attempts_to_withdraw_all += 1;
            println!("Withdraw All! withdraw - actual: {}", amount_to_withdraw - actual_assets_user_has);
            let shares_to_burn = before_withdraw_user_state.deposit_usdc_shares;

            // ---> user state - is closed
            ctx.svm.assert_account_closed(&user_state_pda);

            // ---> user ata state
            ctx.svm.assert_token_balance(&user_ata, actual_assets_user_has);

            // --->bank state
            let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
            assert_eq!(after_withdraw_bank_state.total_deposits, before_withdraw_bank_state.total_deposits - actual_assets_user_has);
            assert_eq!(after_withdraw_bank_state.total_deposit_shares, before_withdraw_bank_state.total_deposit_shares - shares_to_burn);

            // ---> bank token account
            ctx.svm.assert_token_balance(&bank_token_account_pda, before_withdraw_bank_state.total_deposits - actual_assets_user_has);
            final_bank_balance = before_withdraw_bank_state.total_deposits - actual_assets_user_has;

            // final bank and user state after iteration
            final_bank_total_deposits = after_withdraw_bank_state.total_deposits;
            final_bank_total_shares = after_withdraw_bank_state.total_deposit_shares;

            // Assert - WithdrawEvent
            withdraw_result.assert_event_emitted::<WithdrawEvent>();
            let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
            assert_eq!(withdraw_event.user, depositor.pubkey());
            assert_eq!(withdraw_event.amount, actual_assets_user_has);
            assert_eq!(withdraw_event.shares, shares_to_burn);

            // invariants check
            let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
            bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, final_bank_balance);

            sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_user_shares, after_withdraw_bank_state.total_deposit_shares);
        } else {
            // more than MIN_USDC_DEPOSIT should be left as deposited
            attempts_to_withdraw_amount +=1;
            println!("Withdraw an amount! remainer is not a dust: {}", actual_assets_user_has - amount_to_withdraw);
            let shares_to_burn = convert_assets_to_shares(amount_to_withdraw, before_withdraw_bank_state.total_deposit_shares, before_withdraw_bank_state.total_deposits, true);

            // ---> user state - account exists
            ctx.svm.assert_account_exists(&user_state_pda);

            let after_withdraw_user_state: User = ctx.get_account(&user_state_pda).unwrap();
            assert_eq!(after_withdraw_user_state.deposit_usdc_shares, before_withdraw_user_state.deposit_usdc_shares - shares_to_burn);

            // ---> user ata state
            ctx.svm.assert_token_balance(&user_ata, amount_to_withdraw);

            // --->bank state
            let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
            assert_eq!(after_withdraw_bank_state.total_deposits, before_withdraw_bank_state.total_deposits - amount_to_withdraw);
            assert_eq!(after_withdraw_bank_state.total_deposit_shares, before_withdraw_bank_state.total_deposit_shares - shares_to_burn);

            // ---> bank token account
            ctx.svm.assert_token_balance(&bank_token_account_pda, before_withdraw_bank_state.total_deposits - amount_to_withdraw);
            final_bank_balance = before_withdraw_bank_state.total_deposits - amount_to_withdraw;

            // final bank and user state after iteration
            final_bank_total_deposits = after_withdraw_bank_state.total_deposits;
            final_bank_total_shares = after_withdraw_bank_state.total_deposit_shares;
            sum_of_user_shares += after_withdraw_user_state.deposit_usdc_shares;

            // Assert - WithdrawEvent
            withdraw_result.assert_event_emitted::<WithdrawEvent>();
            let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
            assert_eq!(withdraw_event.user, depositor.pubkey());
            assert_eq!(withdraw_event.amount, amount_to_withdraw);
            assert_eq!(withdraw_event.shares, shares_to_burn);

            // invariants check
            let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
            bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, final_bank_balance);

            sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_user_shares, after_withdraw_bank_state.total_deposit_shares);
        }

        // ROLL SLOT AND EXPIRE BLOCKHASH
        ctx.svm.advance_slot(500);
        ctx.svm.expire_blockhash();

        // set timestamp for event record
        clock.unix_timestamp = clock.unix_timestamp + 500 * 400 / 1000;
        ctx.svm.set_sysvar(&clock);
        
        num -= 1;
    }

    // print diagnostic variables
    println!("------- print diagnostic variables ----------");
    println!("attempts_to_withdraw_more_than_deposit: {}", attempts_to_withdraw_more_than_deposit);
    println!("attempts_to_withdraw_amount: {}", attempts_to_withdraw_amount);
    println!("attempts_to_withdraw_all: {}", attempts_to_withdraw_all);


}

#[test]
fn deposit_withdraw_withdraw_should_update_state() {
    // diagnostic variables:
    let mut attempts_to_withdraw_more_than_has = 0;
    let mut attempts_to_withdraw_all_1st_pass = 0;
    let mut attempts_to_withdraw_amount_1st_pass = 0;
    let mut attempts_to_withdraw_all_2nd_pass = 0;
    let mut attempts_to_withdraw_amount_2nd_pass = 0;

    // Arrange
    let mut ctx = init_anchor_ctx();
    let mut num = 100;
    let ref_num = num;
    let mut rng = rand::rng();
    let mut clock: Clock = ctx.svm.get_sysvar();
    
    // Arrange mint
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(mint, bank_authority.pubkey());
    println!("Bank address {}", bank_pda);
    let bank_token_account_pda = get_bank_token_account_pda(mint);
    println!("Bank token acc {}", bank_token_account_pda);
    init_bank_helper(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    let mut final_bank_total_deposits = 0;
    let mut final_bank_total_shares= 0;
    let mut final_bank_balance= 0;
    let mut sum_of_user_shares = 0;

     // Act
    /*
        1 bank -> 100 users
        deposit randX = (MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT) amount -> withdraw randY = rand(0..MAX_USDC_DEPOSIT) amount ->
                    ---> if user's assets < randY + MIN_USDC_DEPOSIT -> withdraw all -> close user acc
                    ---> else -> withdraw randY -> user acc exists
    */
    while num != 0 {
        let amount_to_deposit: u64 = rng.random_range(MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT);
        let amount_to_withdraw_1: u64 = rng.random_range(0..=MAX_USDC_DEPOSIT);// don't limit amount to withdraw by amount to deposit - the cases what the user wants more than has should be included too!!!
        
        println!("-----------------------------");
        println!("num: {}, amount_to_deposit: {}, amount_to_withdraw: {}", num, amount_to_deposit, amount_to_withdraw_1);
        if amount_to_deposit < amount_to_withdraw_1 {
            attempts_to_withdraw_more_than_has += 1;
        }

        // Arrange - depositor
        let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
        println!("User address {}", depositor.pubkey());

        // Arrange user
        let user_state_pda = get_user_account_pda(depositor.pubkey());
        let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
        println!("user ata {}", user_ata);
        ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit).unwrap();

        // Deposit
        let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

        ctx
        .execute_instruction(deposit_inx, &[&depositor])
        .unwrap();

        // state before withdraw 1
        // ---> bank state
        let before_withdraw_1_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
        assert_eq!(before_withdraw_1_bank_state.total_deposits, final_bank_total_deposits + amount_to_deposit);
        assert_eq!(before_withdraw_1_bank_state.total_deposit_shares, final_bank_total_shares + amount_to_deposit);

        // ---> bank token account
        let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
        let bank_token_balance = bank_token_account.amount;
        ctx.svm.assert_token_balance(&bank_token_account_pda, bank_token_balance);

        // ---> user state
        let before_withdraw_1_user_state: User = ctx.get_account(&user_state_pda).unwrap();
        let actual_assets_user_has_1 = convert_shares_to_assets(
            before_withdraw_1_user_state.deposit_usdc_shares,
            before_withdraw_1_bank_state.total_deposit_shares,
            before_withdraw_1_bank_state.total_deposits
        );
        assert_eq!(before_withdraw_1_user_state.deposit_usdc_shares, amount_to_deposit);

        // ---> user ata state
        ctx.svm.assert_token_balance(&user_ata, 0);

        // ROLL SLOT AND EXPIRE BLOCKHASH
        ctx.svm.advance_slot(500);
        ctx.svm.expire_blockhash();
        // set timestamp for event record
        clock.unix_timestamp = clock.unix_timestamp + 500 * 400 / 1000;
        ctx.svm.set_sysvar(&clock);

        // ----------- WITHDRAW 1 ---------------
        let withdraw_1_accounts = accounts::Withdraw {
            user: depositor.pubkey(),
            user_state: user_state_pda,
            bank_state: bank_pda,
            mint: mint,
            user_associated_token_account: user_ata,
            bank_token_account: bank_token_account_pda,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::system_program::ID,
        };
        let withdraw_1_inx = ctx
            .program()
            .accounts(withdraw_1_accounts)
            .args(args::Withdraw {assets_amount_to_withdraw: amount_to_withdraw_1})
            .instruction()
            .unwrap();
        let withdraw_1_result = ctx
            .execute_instruction(withdraw_1_inx, &[&depositor])
            .unwrap();

        // state after withdraw 1
        if actual_assets_user_has_1 < (amount_to_withdraw_1 + MIN_USDC_DEPOSIT) {
            // withdraw all
            attempts_to_withdraw_all_1st_pass += 1;
            println!("----> Withdraw 1 <------- All! withdraw - actual: {}", amount_to_withdraw_1 - actual_assets_user_has_1);
            let shares_to_burn_1 = before_withdraw_1_user_state.deposit_usdc_shares;

            // ---> user state - is closed
            ctx.svm.assert_account_closed(&user_state_pda);

            // ---> user ata state
            ctx.svm.assert_token_balance(&user_ata, actual_assets_user_has_1);

            // --->bank state
            let after_withdraw_1_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
            assert_eq!(after_withdraw_1_bank_state.total_deposits, before_withdraw_1_bank_state.total_deposits - actual_assets_user_has_1);
            assert_eq!(after_withdraw_1_bank_state.total_deposit_shares, before_withdraw_1_bank_state.total_deposit_shares - shares_to_burn_1);

            // ---> bank token account
            ctx.svm.assert_token_balance(&bank_token_account_pda, before_withdraw_1_bank_state.total_deposits - actual_assets_user_has_1);
            final_bank_balance = before_withdraw_1_bank_state.total_deposits - actual_assets_user_has_1;

            // final bank and user state after iteration
            final_bank_total_deposits = after_withdraw_1_bank_state.total_deposits;
            final_bank_total_shares = after_withdraw_1_bank_state.total_deposit_shares;

            // Assert - WithdrawEvent
            withdraw_1_result.assert_event_emitted::<WithdrawEvent>();
            let withdraw_event: WithdrawEvent = withdraw_1_result.parse_event().unwrap();
            assert_eq!(withdraw_event.user, depositor.pubkey());
            assert_eq!(withdraw_event.amount, actual_assets_user_has_1);
            assert_eq!(withdraw_event.shares, shares_to_burn_1);

            // invariants check
            let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
            bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, final_bank_balance);

            sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_user_shares, after_withdraw_1_bank_state.total_deposit_shares);
        } else {
            // more than MIN_USDC_DEPOSIT should be left as deposited
            attempts_to_withdraw_amount_1st_pass +=1;
            println!("-----> Withdraw 1 <------ an amount! remainer is not a dust: {}", actual_assets_user_has_1 - amount_to_withdraw_1);
            let shares_to_burn_1 = convert_assets_to_shares(amount_to_withdraw_1, before_withdraw_1_bank_state.total_deposit_shares, before_withdraw_1_bank_state.total_deposits, true);

            // ---> user state - account exists
            ctx.svm.assert_account_exists(&user_state_pda);

            let after_withdraw_1_user_state: User = ctx.get_account(&user_state_pda).unwrap();
            assert_eq!(after_withdraw_1_user_state.deposit_usdc_shares, before_withdraw_1_user_state.deposit_usdc_shares - shares_to_burn_1);

            // ---> user ata state
            ctx.svm.assert_token_balance(&user_ata, amount_to_withdraw_1);
            let after_withdraw_1_user_ata: TokenAccount = ctx.get_account(&user_ata).unwrap();


            // --->bank state
            let after_withdraw_1_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
            assert_eq!(after_withdraw_1_bank_state.total_deposits, before_withdraw_1_bank_state.total_deposits - amount_to_withdraw_1);
            assert_eq!(after_withdraw_1_bank_state.total_deposit_shares, before_withdraw_1_bank_state.total_deposit_shares - shares_to_burn_1);

            // ---> bank token account
            ctx.svm.assert_token_balance(&bank_token_account_pda, before_withdraw_1_bank_state.total_deposits - amount_to_withdraw_1);
            final_bank_balance = before_withdraw_1_bank_state.total_deposits - amount_to_withdraw_1;

            // final bank and user state after iteration
            final_bank_total_deposits = after_withdraw_1_bank_state.total_deposits;
            final_bank_total_shares = after_withdraw_1_bank_state.total_deposit_shares;
            sum_of_user_shares += after_withdraw_1_user_state.deposit_usdc_shares;

            // Assert - WithdrawEvent
            withdraw_1_result.assert_event_emitted::<WithdrawEvent>();
            let withdraw_event: WithdrawEvent = withdraw_1_result.parse_event().unwrap();
            assert_eq!(withdraw_event.user, depositor.pubkey());
            assert_eq!(withdraw_event.amount, amount_to_withdraw_1);
            assert_eq!(withdraw_event.shares, shares_to_burn_1);

            // invariants check
            let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
            bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, final_bank_balance);

            sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_user_shares, after_withdraw_1_bank_state.total_deposit_shares);

            // ----------- WITHDRAW 2 ---------------
            // ROLL SLOT AND EXPIRE BLOCKHASH
            ctx.svm.advance_slot(500);
            ctx.svm.expire_blockhash();
            // set timestamp for event record
            clock.unix_timestamp = clock.unix_timestamp + 500 * 400 / 1000;
            ctx.svm.set_sysvar(&clock);

            // ARRANGE
            let amount_to_withdraw_2: u64 = rng.random_range(0..=MAX_USDC_DEPOSIT);// don't limit amount to withdraw by amount to deposit - the cases what the user wants more than has should be included too!!!

            let actual_assets_user_has_2 = convert_shares_to_assets(
                after_withdraw_1_user_state.deposit_usdc_shares,
                after_withdraw_1_bank_state.total_deposit_shares,
                after_withdraw_1_bank_state.total_deposits
            );
            
            println!("-----------------------------");
            println!("num: {}, actual_assets_user_has_2: {}, amount_to_withdraw: {}", num, actual_assets_user_has_2, amount_to_withdraw_2);
            if actual_assets_user_has_2 < amount_to_withdraw_2 {
                attempts_to_withdraw_more_than_has += 1;
            }

            // Withdraw 2
            let withdraw_2_accounts = accounts::Withdraw {
                user: depositor.pubkey(),
                user_state: user_state_pda,
                bank_state: bank_pda,
                mint: mint,
                user_associated_token_account: user_ata,
                bank_token_account: bank_token_account_pda,
                token_program: anchor_spl::token::ID,
                system_program: anchor_lang::system_program::ID,
            };
            let withdraw_2_inx = ctx
                .program()
                .accounts(withdraw_2_accounts)
                .args(args::Withdraw {assets_amount_to_withdraw: amount_to_withdraw_2})
                .instruction()
                .unwrap();
            let withdraw_2_result = ctx
                .execute_instruction(withdraw_2_inx, &[&depositor])
                .unwrap();

            // state after withdraw 2
            if actual_assets_user_has_2 < (amount_to_withdraw_2 + MIN_USDC_DEPOSIT) {
                // withdraw all
                attempts_to_withdraw_all_2nd_pass += 1;
                println!("------> Withdraw 2 <----- All! withdraw - actual: {}", amount_to_withdraw_2 - actual_assets_user_has_2);
                let shares_to_burn_2 = after_withdraw_1_user_state.deposit_usdc_shares;

                // ---> user state - is closed
                ctx.svm.assert_account_closed(&user_state_pda);

                // ---> user ata state
                ctx.svm.assert_token_balance(&user_ata, after_withdraw_1_user_ata.amount + actual_assets_user_has_2);

                // --->bank state
                let after_withdraw_2_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
                assert_eq!(after_withdraw_2_bank_state.total_deposits, after_withdraw_1_bank_state.total_deposits - actual_assets_user_has_2);
                assert_eq!(after_withdraw_2_bank_state.total_deposit_shares, after_withdraw_1_bank_state.total_deposit_shares - shares_to_burn_2);

                // ---> bank token account
                ctx.svm.assert_token_balance(&bank_token_account_pda, after_withdraw_1_bank_state.total_deposits - actual_assets_user_has_2);
                final_bank_balance = after_withdraw_1_bank_state.total_deposits - actual_assets_user_has_2;

                // final bank and user state after iteration
                final_bank_total_deposits = after_withdraw_2_bank_state.total_deposits;
                final_bank_total_shares = after_withdraw_2_bank_state.total_deposit_shares;
                sum_of_user_shares -= shares_to_burn_2;

                // Assert - WithdrawEvent
                withdraw_2_result.assert_event_emitted::<WithdrawEvent>();
                let withdraw_event: WithdrawEvent = withdraw_2_result.parse_event().unwrap();
                assert_eq!(withdraw_event.user, depositor.pubkey());
                assert_eq!(withdraw_event.amount, actual_assets_user_has_2);
                assert_eq!(withdraw_event.shares, shares_to_burn_2);

                // invariants check
                let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
                bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, final_bank_balance);

                sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_user_shares, after_withdraw_2_bank_state.total_deposit_shares);
            } else {
                // more than MIN_USDC_DEPOSIT should be left as deposited
                attempts_to_withdraw_amount_2nd_pass +=1;
                println!("----> Withdraw 2 <----- an amount! remainer is not a dust: {}", actual_assets_user_has_2 - amount_to_withdraw_2);
                let shares_to_burn_2 = convert_assets_to_shares(amount_to_withdraw_2, after_withdraw_1_bank_state.total_deposit_shares, after_withdraw_1_bank_state.total_deposits, true);

                // ---> user state - account exists
                ctx.svm.assert_account_exists(&user_state_pda);

                let after_withdraw_2_user_state: User = ctx.get_account(&user_state_pda).unwrap();
                assert_eq!(after_withdraw_2_user_state.deposit_usdc_shares, after_withdraw_1_user_state.deposit_usdc_shares - shares_to_burn_2);

                // ---> user ata state
                ctx.svm.assert_token_balance(&user_ata, after_withdraw_1_user_ata.amount + amount_to_withdraw_2);

                // --->bank state
                let after_withdraw_2_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
                assert_eq!(after_withdraw_2_bank_state.total_deposits, after_withdraw_1_bank_state.total_deposits - amount_to_withdraw_2);
                assert_eq!(after_withdraw_2_bank_state.total_deposit_shares, after_withdraw_1_bank_state.total_deposit_shares - shares_to_burn_2);

                // ---> bank token account
                ctx.svm.assert_token_balance(&bank_token_account_pda, after_withdraw_1_bank_state.total_deposits - amount_to_withdraw_2);
                final_bank_balance = after_withdraw_1_bank_state.total_deposits - amount_to_withdraw_2;

                // final bank and user state after iteration
                final_bank_total_deposits = after_withdraw_2_bank_state.total_deposits;
                final_bank_total_shares = after_withdraw_2_bank_state.total_deposit_shares;
                sum_of_user_shares -= shares_to_burn_2;

                // Assert - WithdrawEvent
                withdraw_2_result.assert_event_emitted::<WithdrawEvent>();
                let withdraw_event: WithdrawEvent = withdraw_2_result.parse_event().unwrap();
                assert_eq!(withdraw_event.user, depositor.pubkey());
                assert_eq!(withdraw_event.amount, amount_to_withdraw_2);
                assert_eq!(withdraw_event.shares, shares_to_burn_2);

                // invariants check
                let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
                bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, final_bank_balance);

                sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_user_shares, after_withdraw_2_bank_state.total_deposit_shares);
            }
        }

        // ROLL SLOT AND EXPIRE BLOCKHASH
        ctx.svm.advance_slot(500);
        ctx.svm.expire_blockhash();

        // set timestamp for event record
        clock.unix_timestamp = clock.unix_timestamp + 500 * 400 / 1000;
        ctx.svm.set_sysvar(&clock);
        
        num -= 1;
    }

    // print diagnostic variables
    assert_eq!(ref_num, attempts_to_withdraw_all_1st_pass + attempts_to_withdraw_amount_1st_pass);
    assert_eq!(attempts_to_withdraw_amount_1st_pass, attempts_to_withdraw_all_2nd_pass + attempts_to_withdraw_amount_2nd_pass);
    println!("------- print diagnostic variables ----------");
    println!("attempts_to_withdraw_more_than_deposit: {}", attempts_to_withdraw_more_than_has);
    println!("attempts_to_withdraw_amount_1st_pass: {}", attempts_to_withdraw_amount_1st_pass);
    println!("--------> attempts_to_withdraw_amount_2nd_pass: {}", attempts_to_withdraw_amount_2nd_pass);
    println!("--------> attempts_to_withdraw_all_2nd_pass: {}", attempts_to_withdraw_all_2nd_pass);
    println!("attempts_to_withdraw_all_1st_pass: {}", attempts_to_withdraw_all_1st_pass);
    


}


