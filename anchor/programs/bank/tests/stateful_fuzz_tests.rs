use anchor_lang::solana_program::clock::{Clock};
use anchor_litesvm::{ AccountError, AnchorContext, AssertionHelpers, EventHelpers, Pubkey, Signer, TestHelpers};
use anchor_spl::{ token_interface::TokenAccount};
use anchor_spl::associated_token::get_associated_token_address;
use ::bank::{//import from external crate (not from idl modules)
    Bank,
    UserShares,
    events::{DepositEvent, WithdrawEvent, BankSnapshot},
    constants::{ MIN_USDC_DEPOSIT, MAX_USDC_DEPOSIT, SECONDS_PER_WEEK },
    shares_math::{convert_shares_to_assets, convert_assets_to_shares},
};
use solana_keypair::Keypair;
use solana_sdk::{native_token::LAMPORTS_PER_SOL};
use rand::RngExt;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

use bank_test_utils::*;
use bank_client::client::{ accounts, args };

mod invariants_tests;
use invariants_tests::*;

// use test_env_utils::bank::{
//     client::{accounts, args},
//     accounts::{UserShares, Bank},
//     //events::DepositEvent, //import from idl modules
// };

use chrono::{Utc};

#[test]
fn deposits_in_raw_should_update_state() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(&mint);
    let bank_token_account_pda = get_bank_token_account_pda(&mint);

    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

    // Arrange - depositor    
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    init_user_shares_and_assert(&mut ctx, &depositor, &mint);
    
    let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), &mint);
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, u64::MAX).unwrap();

    // Arrange - INIT STATE FOR THE BANK
    let init_bank_state:Bank = ctx.get_account(&bank_pda).unwrap();
    let mut init_total_assets = init_bank_state.total_deposits;
    let mut init_total_shares = init_bank_state.total_deposit_shares;

    let init_bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    let mut init_bank_token_account_balance = init_bank_token_account.amount;

    // Arrange - INIT STATE FOR THE USER
    let init_user_shares: UserShares = ctx.get_account(&user_shares_pda).unwrap(); 
    let mut init_user_deposit_shares = init_user_shares.deposit_shares;

    let user_ata_account: TokenAccount = ctx.get_account(&user_ata).unwrap();
    let mut init_user_ata_balance = user_ata_account.amount;

    let mut num = 100;
    let mut rng = rand::rng();
    let mut clock: Clock = ctx.svm.get_sysvar();
    // Act
    // 1. deposit -> RECORD EVENT -> state -> invariants check -> roll slot    
    while num != 0 {
        let amount_to_deposit: u64 = rng.random_range(MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT);

        let inx = get_deposit_inx(&mut ctx, &depositor.pubkey(), &mint, &user_ata, amount_to_deposit);

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
        let updated_user_shares: UserShares = ctx.get_account(&user_shares_pda).unwrap();
        assert_eq!(updated_user_shares.deposit_shares, init_user_deposit_shares + shares_to_be_added_from_amount);
        init_user_deposit_shares = updated_user_shares.deposit_shares;

        // check user ata
        let updated_user_ata_account_updated: TokenAccount = ctx.get_account(&user_ata).unwrap();
        assert_eq!(updated_user_ata_account_updated.amount, init_user_ata_balance - amount_to_deposit);
        init_user_ata_balance = updated_user_ata_account_updated.amount;

        // 5. CHECK INVARIANTS
        bank_token_account_balance_not_less_than_bank_total_deposits(updated_bank_token_account.amount, updated_bank_state.total_deposits);

        sum_of_users_deposit_shares_equals_bank_total_deposit_shares(updated_user_shares.deposit_shares, updated_bank_state.total_deposit_shares);

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
    let bank_pda = get_bank_account_pda(&mint);
    println!("Bank address {}", bank_pda);
    let bank_token_account_pda = get_bank_token_account_pda(&mint);
    println!("Bank token acc {}", bank_token_account_pda);
    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

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
        
        init_user_shares_and_assert(&mut ctx, &depositor, &mint);

        // Arrange user
        let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), &mint);
        let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
        println!("user ata {}", user_ata);
        ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit).unwrap();

        // Deposit
        let deposit_inx = get_deposit_inx(&mut ctx, &depositor.pubkey(), &mint, &user_ata, amount_to_deposit);

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
        let before_withdraw_user_shares: UserShares = ctx.get_account(&user_shares_pda).unwrap();
        let actual_assets_user_has = convert_shares_to_assets(
            before_withdraw_user_shares.deposit_shares,
            before_withdraw_bank_state.total_deposit_shares,
            before_withdraw_bank_state.total_deposits
        );
        assert_eq!(before_withdraw_user_shares.deposit_shares, amount_to_deposit);

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
            user_shares: user_shares_pda,
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
            let shares_to_burn = before_withdraw_user_shares.deposit_shares;

            // ---> user state - is closed
            ctx.svm.assert_account_closed(&user_shares_pda);

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
            ctx.svm.assert_account_exists(&user_shares_pda);

            let after_withdraw_user_shares: UserShares = ctx.get_account(&user_shares_pda).unwrap();
            assert_eq!(after_withdraw_user_shares.deposit_shares, before_withdraw_user_shares.deposit_shares - shares_to_burn);

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
            sum_of_user_shares += after_withdraw_user_shares.deposit_shares;

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
    let utc_now = Utc::now().to_string();
    let test_name = "deposit_withdraw_withdraw_should_update_state";

    let mut ctx = init_anchor_ctx();
    let mut num = 100;
    let mut rng = rand::rng();
    let mut sum_of_users_shares = 0;

    // Arrange mint
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(&mint);
    let bank_token_account_pda = get_bank_token_account_pda(&mint);

    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

     // Act
    /*
        1 bank -> 100 users
        deposit randX = (MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT) amount -> withdraw randY = rand(0..MAX_USDC_DEPOSIT) amount ->
                    ---> if user's assets < randY + MIN_USDC_DEPOSIT -> withdraw all -> close user acc
                    ---> else -> withdraw randY -> user acc exists
    */
    while num != 0 {
        let mut step: u8 = 1;
        
        // Arrange - depositor
        let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
        init_user_shares_and_assert(&mut ctx, &depositor, &mint);
        let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), &mint);
        let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
        ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MAX_USDC_DEPOSIT).unwrap();

        // Deposit
        let amount_to_deposit: u64 = rng.random_range(MIN_USDC_DEPOSIT..=MAX_USDC_DEPOSIT);
        let (deposit_result, actual_deposited_amount, shares_to_mint) = match process_deposit_and_assert_states(&mut ctx, &user_shares_pda, &depositor, mint, &user_ata, amount_to_deposit) {
                    Ok(t) => t,
                    Err(_) => {
                        continue;
                    },
                };

        let _ = assert_deposit_event(&deposit_result, &depositor.pubkey(), actual_deposited_amount, shares_to_mint);

        // invariants check
        let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
        let bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
        bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, bank_state.total_deposits);

        sum_of_users_shares += shares_to_mint;
        sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_users_shares, bank_state.total_deposit_shares);
        
        time_travel(&mut ctx);

        // roll step
        step += 1;
        
        // Withdraw 1
        let amount_to_withdraw_1: u64 = rng.random_range(0..=MAX_USDC_DEPOSIT);// don't limit amount to withdraw by amount to deposit - the cases what the user wants more than has should be included too!!!

        let (withdraw_1_result, actually_withdrawn_assets_1, shares_to_burn_1, user_is_closed) = match process_withdraw_and_assert_states(&mut ctx, &user_shares_pda, &depositor, &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_withdraw_1)  {
                    Ok(t) => t,
                    Err(error) => continue,
                };

        assert_withdraw_event(&withdraw_1_result, &depositor.pubkey(), actually_withdrawn_assets_1, shares_to_burn_1);

        // invariants check
        let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
        let bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
        bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, bank_state.total_deposits);

        sum_of_users_shares -= shares_to_burn_1;
        sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_users_shares, bank_state.total_deposit_shares);

        time_travel(&mut ctx);

        // roll step
        step += 1;

        // check if user exists, so he has any shares are left for the user
        if !user_is_closed {
            let user_shares: UserShares = ctx.get_account(&user_shares_pda).unwrap();
            if user_shares.deposit_shares > 0 {
                // Withdraw 2
                let amount_to_withdraw_2: u64 = rng.random_range(0..=MAX_USDC_DEPOSIT);// don't limit amount to withdraw by amount to deposit - the cases what the user wants more than has should be included too!!!
                let (withdraw_2_result, actually_withdrawn_assets_2, shares_to_burn_2, _) = match process_withdraw_and_assert_states(&mut ctx, &user_shares_pda, &depositor, &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_withdraw_2)  {
                    Ok(t) => t,
                    Err(error) => continue,
                };

                assert_withdraw_event(&withdraw_2_result, &depositor.pubkey(), actually_withdrawn_assets_2, shares_to_burn_2);

                // invariants check
                let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
                let bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
                bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, bank_state.total_deposits);

                sum_of_users_shares -= shares_to_burn_2;
                sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_users_shares, bank_state.total_deposit_shares);

                time_travel(&mut ctx);
            }
        }

        num -= 1;
    }
}

#[test]
fn randomized_test() {
    let utc_now = Utc::now();//gets timestamp from sysvar
    let seed: u64 = 8555; 
    let utc_not_str = utc_now.to_string();
    let test_name = format!("seed-{seed}-randomized_test_{utc_not_str}");

    // Arrange
    let mut ctx = init_anchor_ctx();
    
    // Arrange mint
    let (mint, mint_authority) = get_mint_pubkey_and_authority(&mut ctx);

    // Arrange bank
    let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let bank_pda = get_bank_account_pda(&mint);
    let bank_token_account_pda = get_bank_token_account_pda(&mint);
    init_bank_and_assert(&mut ctx, &mint, &bank_authority);

    //choose random inx
    let mut amount_of_init_users = 0;
    let mut amount_of_deposits = 0;
    let mut amount_of_withdraws = 0;

    let amount_of_inxs = 50;

    let mut depositors: Vec<Keypair> = Vec::new();
    let mut steps: HashMap<Pubkey, u8> = HashMap::new();
    let mut sum_of_users_shares = 0;

    let mut rng = StdRng::seed_from_u64(seed);
    for _ in 0..amount_of_inxs {
        let instruction = BankInstruction::random(&mut rng);

        match instruction {
            BankInstruction::InitUser => {
                amount_of_init_users += 1;
                let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
                init_user_shares_and_assert(&mut ctx, &depositor, &mint);
                let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
                ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MAX_USDC_DEPOSIT * amount_of_inxs).unwrap();
            
                // add step
                steps.insert(depositor.pubkey(), 0);

                // add user to a Vec<Keypair> to choose it for other inxs randomly
                depositors.push(depositor);
            },
            BankInstruction::Deposit => {
                println!("------------");
                amount_of_deposits += 1;

                // get depositor from Vec<Keypair> any random
                if depositors.len() == 0 {
                    println!("---> No depositors created yet!");
                    continue;
                }
                let random_depositor_index = rng.random_range(0..depositors.len());
                let depositor = depositors.get(random_depositor_index).unwrap();

                let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), &mint);
                let user_ata = get_associated_token_address(&depositor.pubkey(), &mint);

                // Deposit
                let amount_to_deposit: u64 = rng.random_range(0..=MAX_USDC_DEPOSIT * 2); // make the deposit inx check the min and max deposit amount

                let (deposit_result, actual_deposited_amount, shares_to_mint) = match process_deposit_and_assert_states(&mut ctx, &user_shares_pda, depositor, mint, &user_ata, amount_to_deposit) {
                    Ok(t) => t,
                    Err(_) => {
                        continue;
                    },
                };

                // step 
                let mut step = steps[&depositor.pubkey()];
                step += 1;

                // update step// add step
                steps.insert(depositor.pubkey(), step);

                // assert event
                let deposit_event = assert_deposit_event(&deposit_result, &depositor.pubkey(), actual_deposited_amount, shares_to_mint);
                record_bank_event(&deposit_event, step, &test_name, seed);

                // invariants check
                let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
                let bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
                bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, bank_state.total_deposits);
                sum_of_users_shares += shares_to_mint;
                sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_users_shares, bank_state.total_deposit_shares);

                record_bank_snapshot(&bank_state, &depositor.pubkey(), step, &test_name, seed);
                
                time_travel(&mut ctx);
            },
            BankInstruction::Withdraw => {
                println!("------------");
                amount_of_withdraws += 1;

                // get depositor from Vec<Keypair> any random
                if depositors.len() == 0 {
                    println!("---> No depositors created yet!");
                    continue;
                }
                let random_depositor_index = rng.random_range(0..depositors.len());
                let depositor = depositors.get(random_depositor_index).unwrap();

                let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), &mint);
                let user_ata = get_associated_token_address(&depositor.pubkey(), &mint);

                // withdraw
                let amount_to_withdraw = rng.random_range(0..=MAX_USDC_DEPOSIT);
                let (withdraw_result, actually_withdrawn_assets, shares_to_burn, user_is_closed) = match process_withdraw_and_assert_states(&mut ctx, &user_shares_pda, &depositor, &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_withdraw) {
                    Ok(t) => t,
                    Err(_) => {
                        continue;
                    },
                };

                // step 
                let mut step = steps[&depositor.pubkey()];
                step += 1;

                let withdraw_event = assert_withdraw_event(&withdraw_result, &depositor.pubkey(), actually_withdrawn_assets, shares_to_burn);
                record_bank_event(&withdraw_event, step, &test_name, seed);

                // invariants check
                let bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
                let bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
                bank_token_account_balance_not_less_than_bank_total_deposits(bank_token_account.amount, bank_state.total_deposits);

                sum_of_users_shares -= shares_to_burn;
                sum_of_users_deposit_shares_equals_bank_total_deposit_shares(sum_of_users_shares, bank_state.total_deposit_shares);

                record_bank_snapshot(&bank_state, &depositor.pubkey(), step, &test_name, seed);

                time_travel(&mut ctx);
                
                // if user is closed - remove her from depositors
                // update step if not closed, remove step if closed
                if user_is_closed {
                    steps.remove(&depositor.pubkey());
                    depositors.remove(random_depositor_index);
                    amount_of_init_users -=1;
                } else {
                    steps.insert(depositor.pubkey(), step);
                }
            },
            /*BankInstruction::Swap => {
                println!("swap an amount for a user - not implemented yet");
            }*/
        }
    }
    println!("---------------");
    
    println!("Depositors:");
    for k in depositors.iter() {
        println!("{}", k.pubkey());
    }

    println!("---------------");
    println!("amount_of_init_users {}, depositors amount is {}", amount_of_init_users, depositors.len());
    assert_eq!(amount_of_init_users, depositors.len());

    println!("amount_of_deposits {}", amount_of_deposits);
    println!("amount_of_withdraws {}", amount_of_withdraws);

    
}

pub enum BankInstruction {
    InitUser,
    Deposit,
    Withdraw,
    //Swap,
}

impl BankInstruction {
    fn random(rng: &mut StdRng) -> Self {
        match rng.random_range(0..3) {
            0 => {
                Self::InitUser
            },
            1 => {
                Self::Deposit
            },
            2 => {
                Self::Withdraw
            },
            /*3 => {
                Self::Swap
            }*/
            _ => unreachable!(),
        }
    }
}

fn time_travel(ctx: &mut AnchorContext) {
    let mut clock: Clock = ctx.svm.get_sysvar();
    let mut rng = rand::rng();
    
    // EXPIRE BLOCKHASH
    ctx.svm.expire_blockhash();
    
    // set timestamp
    let delta: i64 = rng.random_range(1..=SECONDS_PER_WEEK);
    clock.unix_timestamp += delta;
    ctx.svm.set_sysvar(&clock);
}
/*
Withdraw
Deposit
Swap
Swap
Withdraw
Withdraw
Withdraw
Deposit
Withdraw
Deposit
Withdraw

*/
