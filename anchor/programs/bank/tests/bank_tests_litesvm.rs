
use anchor_lang;
use anchor_lang::declare_program;
use anchor_litesvm::{ AnchorContext, AnchorLiteSVM, AssertionHelpers, Pubkey, Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use solana_keypair::Keypair;
use solana_sdk::native_token::LAMPORTS_PER_SOL;

declare_program!(bank);

use crate::bank::client::accounts::Deposit;

use self::bank::client::{accounts, args};
use self::bank::accounts::{User, Bank};

const PROGRAM_BYTES: &[u8] = include_bytes!("../../../target/deploy/bank.so");

fn init_anchor_ctx() -> anchor_litesvm::AnchorContext {
    let ctx = AnchorLiteSVM::build_with_program(self::bank::ID, PROGRAM_BYTES);
    ctx
}

fn init_bank_accounts(ctx: &mut AnchorContext, mint: &Keypair, bank_pda: &Pubkey, bank_token_account_pda: &Pubkey, bank_authority: &Keypair) -> (Bank, TokenAccount) {
    println!("incoming mint key {} ", mint.pubkey());
    ctx.svm.assert_account_exists(&mint.pubkey());

    let ix = ctx
        .program()
        .accounts(accounts::InitBank {
            authority: bank_authority.pubkey(),
            mint: mint.pubkey(),
            bank_state: *bank_pda,
            bank_token_account: *bank_token_account_pda,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::system_program::ID,
        })
        .args(args::InitBank {})
        .instruction()
        .unwrap();

    let result = ctx.execute_instruction(ix, &[&bank_authority]).unwrap();

    println!("TransactionResult {:?}", result);

    result.assert_success();

    println!("bank_pda {}", bank_pda);
    println!("bank_token_account_pda {}", bank_token_account_pda);
    
    ctx.svm.assert_account_exists(bank_pda);
    ctx.svm.assert_account_exists(bank_token_account_pda);

    let bank_account: Bank = ctx.get_account(bank_pda).unwrap();
    let bank_token_account: TokenAccount = ctx.get_account(bank_token_account_pda).unwrap();
    
    (bank_account, bank_token_account)
}
    
fn get_deposit_inx_accounts(user_state_pda: &Pubkey, depositor: &Pubkey, bank_pda: &Pubkey, mint: &Pubkey, bank_token_account_pda: &Pubkey, user_ata: &Pubkey,) -> accounts::Deposit {
    accounts::Deposit {
        user: *depositor,
        user_state: *user_state_pda,
        bank_state: *bank_pda,
        mint: *mint,
        user_associated_token_account: *user_ata,
        bank_token_account: *bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    }
}

#[test]
fn should_init_bank() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let mint_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let mint = ctx.svm.create_token_mint(&mint_authority, 6).unwrap();
    println!("created mint key {:?}", mint.pubkey());
    ctx.svm.assert_account_exists(&mint.pubkey());

    let bank_pda = Pubkey::find_program_address(&[b"SEED_BANK_STATE", mint.pubkey().as_ref()], &self::bank::ID).0;
    let bank_token_account_pda = Pubkey::find_program_address(&[b"SEED_BANK_TOKEN_ACCOUNT", mint.pubkey().as_ref()], &self::bank::ID).0;

     let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    let (bank_account, bank_token_account) = init_bank_accounts(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    assert_eq!(bank_account.is_initialized, true);
    assert_eq!(bank_account.authority, bank_authority.pubkey());
    assert_eq!(bank_account.mint, mint.pubkey());
    assert_eq!(bank_account.total_deposits, 0);
    assert_eq!(bank_account.total_deposit_shares, 0);

    assert_eq!(bank_token_account.mint, mint.pubkey());
    assert_eq!(bank_token_account.amount, 0);
}

#[test]
fn deposit_should_revert_if_amount_is_zero() {
    // Arrange
    let mut ctx = init_anchor_ctx();
    let mint_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let mint = ctx.svm.create_token_mint(&mint_authority, 6).unwrap();
    println!("created mint key {:?}", mint.pubkey());
    ctx.svm.assert_account_exists(&mint.pubkey());

    let bank_pda = Pubkey::find_program_address(&[b"SEED_BANK_STATE", mint.pubkey().as_ref()], &self::bank::ID).0;
    let bank_token_account_pda = Pubkey::find_program_address(&[b"SEED_BANK_TOKEN_ACCOUNT", mint.pubkey().as_ref()], &self::bank::ID).0;

     let bank_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    let (bank_account, bank_token_account) = init_bank_accounts(&mut ctx, &mint, &bank_pda, &bank_token_account_pda, &bank_authority);

    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();

    let user_state_pda = Pubkey::find_program_address(&[b"SEED_USER_STATE", depositor.pubkey().as_ref()], &self::bank::ID).0;

    let user_ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &depositor).unwrap();

    let deposit_inx_accounts = get_deposit_inx_accounts(&user_state_pda, &depositor.pubkey(), &bank_pda, &mint.pubkey(), &bank_token_account_pda, &user_ata,);
    println!("deposit_inx_accounts - mint {}", deposit_inx_accounts.mint);

    // Act / Assert
    let inx = ctx
        .program()
        .accounts(deposit_inx_accounts)
        .args(args::Deposit { amount: 0 })
        .instruction()
        .unwrap();

    ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap()
    .assert_failure()
    .assert_anchor_error("ZeroAmountToDeposit");
}



