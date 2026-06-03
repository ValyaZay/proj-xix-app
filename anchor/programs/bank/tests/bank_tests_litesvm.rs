
use anchor_lang::solana_program::pubkey;
use anchor_lang::{self, Key, pubkey};
use anchor_lang::declare_program;
use anchor_litesvm::{ AnchorContext, AnchorLiteSVM, AssertionHelpers, EventHelpers, Pubkey, Signer, TestHelpers};
use anchor_spl::token_interface::TokenAccount;
use solana_keypair::Keypair;
use solana_sdk::native_token::LAMPORTS_PER_SOL;

declare_program!(bank);

use self::bank::{
    client::{accounts, args},
    accounts::{User, Bank},
    events::DepositEvent,
};


const PROGRAM_BYTES: &[u8] = include_bytes!("../../../target/deploy/bank.so");
const MIN_USDC_DEPOSIT: u64 = 10_000_000; // 10 usdc

fn init_anchor_ctx() -> anchor_litesvm::AnchorContext {
    let ctx = AnchorLiteSVM::build_with_program(self::bank::ID, PROGRAM_BYTES);
    ctx
}

fn init_bank_helper(ctx: &mut AnchorContext, mint: &Pubkey, bank_pda: &Pubkey, bank_token_account_pda: &Pubkey, bank_authority: &Keypair) {
    let ix = ctx
        .program()
        .accounts(accounts::InitBank {
            authority: bank_authority.pubkey(),
            mint: *mint,
            bank_state: *bank_pda,
            bank_token_account: *bank_token_account_pda,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::system_program::ID,
        })
        .args(args::InitBank {})
        .instruction()
        .unwrap();

    let result = ctx.execute_instruction(ix, &[&bank_authority]).unwrap();
    result.assert_success();
    
    ctx.svm.assert_account_exists(bank_pda);
    ctx.svm.assert_account_exists(bank_token_account_pda);
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

fn get_mint_pubkey_and_authority(ctx: &mut AnchorContext) -> (Pubkey, Keypair) {
    let mint_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let mint = ctx.svm.create_token_mint(&mint_authority, 6).unwrap();
    println!("created mint key {:?}", mint.pubkey());
    ctx.svm.assert_account_exists(&mint.pubkey());
    (mint.pubkey(), mint_authority)
}

fn get_bank_account_pda(mint: Pubkey, authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_BANK_STATE", mint.as_ref(), authority.as_ref()], &self::bank::ID).0
}

fn get_bank_token_account_pda(mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_BANK_TOKEN_ACCOUNT", mint.as_ref()], &self::bank::ID).0
}

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
}

#[test]
fn deposit_should_revert_if_amount_is_zero() {
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
    let user_state_pda = Pubkey::find_program_address(&[b"SEED_USER_STATE", depositor.pubkey().as_ref()], &self::bank::ID).0;
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MIN_USDC_DEPOSIT).unwrap();

    let deposit_inx_accounts = get_deposit_inx_accounts(&user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata,);

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

    // Arrange - depositor
    let depositor = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let user_state_pda = Pubkey::find_program_address(&[b"SEED_USER_STATE", depositor.pubkey().as_ref()], &self::bank::ID).0;

    // create ata and fund it
    let user_ata = ctx.svm.create_associated_token_account(&mint, &depositor).unwrap();
    ctx.svm.mint_to(&mint, &user_ata, &mint_authority, MIN_USDC_DEPOSIT).unwrap();

    let deposit_inx_accounts = get_deposit_inx_accounts(&user_state_pda, &depositor.pubkey(), &bank_pda, &mint, &bank_token_account_pda, &user_ata,);

    // Act
    let inx = ctx
        .program()
        .accounts(deposit_inx_accounts)
        .args(args::Deposit { amount: MIN_USDC_DEPOSIT })
        .instruction()
        .unwrap();

    let result = ctx
    .execute_instruction(inx, &[&depositor])
    .unwrap();

    // Assert
    result.assert_event_emitted::<DepositEvent>();
    let deposit_event: DepositEvent = result.parse_event().unwrap();
    assert_eq!(deposit_event.user, depositor.pubkey());


    let bank_state_updated:Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(bank_state_updated.total_deposits, MIN_USDC_DEPOSIT);
    assert_eq!(bank_state_updated.total_deposit_shares, MIN_USDC_DEPOSIT);
    
    let user_state: User = ctx.get_account(&user_state_pda).unwrap();
    assert_eq!(user_state.deposit_usdc_shares, MIN_USDC_DEPOSIT);

    let bank_token_account_updated: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    assert_eq!(bank_token_account_updated.amount, MIN_USDC_DEPOSIT);
    println!("bank_token_account_updated.amount {}", bank_token_account_updated.amount);

    // check ata
}


#[test]
fn deposit_should_revert_if_user_is_not_user_state_owner() {

    //require!(user_state.user == ctx.accounts.user.key(), BankErrors::UserIsWrong); - should be redundant check, it should not be possible to sign tx for other user state because there is a contstrain in seeds - user is involved, which is a signer as well
}