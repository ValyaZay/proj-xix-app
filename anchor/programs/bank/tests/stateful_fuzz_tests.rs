use anchor_lang::solana_program::clock::Clock;
use anchor_litesvm::{ AccountError, EventHelpers, Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use ::bank::{//import from external crate (not from idl modules)
    events::{DepositEvent},
    constants::{ MIN_USDC_DEPOSIT, MAX_USDC_DEPOSIT },
    shares_math::convert_assets_to_shares,
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use rand::RngExt;

mod utils;
use utils::*;

mod invariants_tests;
use invariants_tests::*;

use utils::bank::{
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
         let shares_to_be_added_from_amount = convert_assets_to_shares(amount_to_deposit, init_total_shares, init_total_assets);
        
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