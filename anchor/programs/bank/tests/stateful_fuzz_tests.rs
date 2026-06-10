use anchor_lang::solana_program::clock::Clock;
use anchor_litesvm::{ AccountError, EventHelpers, Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use ::bank::{//import from external crate (not from idl modules)
    events::{DepositEvent},
    constants::MIN_USDC_DEPOSIT,
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
    let amount_to_deposit = MIN_USDC_DEPOSIT;
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit * 100).unwrap();

    // Arrange - bank state
    let init_bank_state:Bank = ctx.get_account(&bank_pda).unwrap();
    let mut init_total_assets = init_bank_state.total_deposits;
    let mut init_total_shares = init_bank_state.total_deposit_shares;

    let init_bank_token_account: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    let mut init_bank_token_account_balance = init_bank_token_account.amount;

    let mut num = 100;
    // Act
    // 1. deposit -> state -> invariants check -> roll slot
    let inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);
    
    while num != 0 {
        let slot = ctx.svm.get_sysvar::<Clock>().slot;
        println!("slot {}", slot );
        println!("blockhash {}", ctx.latest_blockhash());

        println!("starter init_total_assets {}, num {}", init_total_assets, num);
        println!("starter init_total_shares {}, num {}", init_total_shares, num);
        let result = ctx
        .execute_instruction(inx.clone(), &[&depositor])
        .unwrap();
        println!("result {:?}", result);

        // shares
         let shares_to_be_added_from_amount = convert_assets_to_shares(amount_to_deposit, init_total_shares, init_total_assets);

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

        num -= 1;

        // invariant check
        bank_token_account_balance_not_less_than_bank_total_deposits(updated_bank_token_account.amount, updated_bank_state.total_deposits);

        // roll slot
        ctx.svm.advance_slot(500);
        ctx.svm.expire_blockhash();
    }
    

    
}