

use utils::bank::{
    client::{accounts::{Withdraw}},
    //accounts::{User, Bank},
    //events::DepositEvent, //import from idl modules
};

use ::bank::constants::MIN_USDC_DEPOSIT;//import from external crate



mod utils;
use utils::*;

fn withdraw_should_update_bank_and_user_states_and_token_accounts_and_emit() {
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

    // Arrange user
    let user_state_pda = get_user_account_pda(depositor.pubkey());
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, amount_to_deposit * 2).unwrap();

    // Deposit
    let deposit_inx = get_deposit_inx(&mut ctx, &user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata, amount_to_deposit);

    let result = ctx
    .execute_instruction(deposit_inx, &[&depositor])
    .unwrap();

    // Withdraw
    let withdraw_accounts = Withdraw {
        user: depositor.pubkey(),
        user_state: user_state_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: user_ata,
        bank_token_account: bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };
    let withdraw_inx = 
}
    