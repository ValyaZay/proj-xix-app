use anchor_lang::{self};
use anchor_lang::declare_program;
use anchor_litesvm::{ AnchorContext, AnchorLiteSVM, AssertionHelpers, Instruction, Pubkey, Signer, TestHelpers, TransactionResult};
use solana_keypair::Keypair;
use solana_sdk::native_token::LAMPORTS_PER_SOL;

declare_program!(bank);

use self::bank::{
    client::{accounts, args},
    //events::DepositEvent, //import from idl modules
};

const PROGRAM_BYTES: &[u8] = include_bytes!("../../../../target/deploy/bank.so");

pub fn init_anchor_ctx() -> anchor_litesvm::AnchorContext {
    let ctx = AnchorLiteSVM::build_with_program(self::bank::ID, PROGRAM_BYTES);
    ctx
}

pub fn init_bank_helper(ctx: &mut AnchorContext, mint: &Pubkey, bank_pda: &Pubkey, bank_token_account_pda: &Pubkey, bank_authority: &Keypair) {
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
    
pub fn get_deposit_inx(ctx: &mut AnchorContext, user_state_pda: &Pubkey, depositor: &Pubkey, bank_pda: &Pubkey, mint: &Pubkey, bank_token_account_pda: &Pubkey, user_ata: &Pubkey, amount: u64) -> Instruction {
    let deposit_accounts = accounts::Deposit {
        user: *depositor,
        user_state: *user_state_pda,
        bank_state: *bank_pda,
        mint: *mint,
        user_associated_token_account: *user_ata,
        bank_token_account: *bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };

    ctx
        .program()
        .accounts(deposit_accounts)
        .args(args::Deposit { amount: amount })
        .instruction()
        .unwrap()
}

pub fn get_mint_pubkey_and_authority(ctx: &mut AnchorContext) -> (Pubkey, Keypair) {
    let mint_authority = ctx.svm.create_funded_account(10 * LAMPORTS_PER_SOL).unwrap();
    let mint = ctx.svm.create_token_mint(&mint_authority, 6).unwrap();
    ctx.svm.assert_account_exists(&mint.pubkey());
    (mint.pubkey(), mint_authority)
}

pub fn get_bank_account_pda(mint: Pubkey, authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_BANK_STATE", mint.as_ref(), authority.as_ref()], &self::bank::ID).0
}

pub fn get_bank_token_account_pda(mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_BANK_TOKEN_ACCOUNT", mint.as_ref()], &self::bank::ID).0
}

pub fn get_user_account_pda(user: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_USER_STATE", user.as_ref()], &self::bank::ID).0
}

pub fn deposit(
    ctx: &mut AnchorContext, 
    user_state_pda: &Pubkey, 
    depositor: &Keypair, 
    bank_pda: &Pubkey, 
    mint: &Pubkey, 
    bank_token_account_pda: &Pubkey, 
    user_ata: &Pubkey, 
    amount: u64) -> TransactionResult {
    let deposit_accounts = accounts::Deposit {
        user: depositor.pubkey(),
        user_state: *user_state_pda,
        bank_state: *bank_pda,
        mint: *mint,
        user_associated_token_account: *user_ata,
        bank_token_account: *bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };

    let inx = ctx
        .program()
        .accounts(deposit_accounts)
        .args(args::Deposit { amount: amount })
        .instruction()
        .unwrap();

    ctx
        .execute_instruction(inx, &[&depositor])
        .unwrap()
}