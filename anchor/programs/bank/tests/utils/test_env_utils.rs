use anchor_lang::{self, error};
use anchor_lang::declare_program;
use anchor_litesvm::{ AccountError, AnchorContext, AnchorLiteSVM, AssertionHelpers, EventHelpers, Instruction, Pubkey, Signer, TestHelpers, TransactionResult};
use anchor_spl::{ token_interface::TokenAccount};
use ::bank::{convert_assets_to_shares, convert_shares_to_assets};
use solana_keypair::Keypair;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use ::bank::{//import from external crate (not from idl modules)
    events::{DepositEvent, WithdrawEvent, BankSnapshot},
    constants::{ MIN_USDC_DEPOSIT, MAX_USDC_DEPOSIT},
    errors::BankErrors,
};

use crate::utils::event_recorder::record_bank_event;

declare_program!(bank);

use crate::bank::{
    client::{accounts, args},
    accounts::{Bank, UserShares}
    //events::DepositEvent, //import from idl modules
};

const PROGRAM_BYTES: &[u8] = include_bytes!("../../../../target/deploy/bank.so");

pub fn init_anchor_ctx() -> anchor_litesvm::AnchorContext {
    let ctx = AnchorLiteSVM::build_with_program(self::bank::ID, PROGRAM_BYTES);
    ctx
}

// TODO remove
pub fn get_deposit_inx(ctx: &mut AnchorContext, user_state_pda: &Pubkey, depositor: &Pubkey, bank_pda: &Pubkey, mint: &Pubkey, bank_token_account_pda: &Pubkey, user_ata: &Pubkey, amount: u64) -> Instruction {
    let deposit_accounts = accounts::Deposit {
        user: *depositor,
        user_shares: *user_state_pda,
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

pub fn get_bank_account_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_BANK_STATE", mint.as_ref()], &self::bank::ID).0
}

pub fn get_bank_token_account_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_BANK_TOKEN_ACCOUNT", mint.as_ref()], &self::bank::ID).0
}

pub fn get_user_shares_pda(user: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"SEED_USER_SHARES", mint.as_ref(), user.as_ref()], &self::bank::ID).0
}

pub fn init_bank_and_assert(
    ctx: &mut AnchorContext, 
    mint: &Pubkey, 
    bank_authority: &Keypair
) {
    let bank_pda = get_bank_account_pda(mint);
    let bank_token_account_pda = get_bank_token_account_pda(mint);

    let ix = ctx
        .program()
        .accounts(accounts::InitBank {
            authority: bank_authority.pubkey(),
            mint: *mint,
            bank_state: bank_pda,
            bank_token_account: bank_token_account_pda,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::system_program::ID,
        })
        .args(args::InitBank {})
        .instruction()
        .unwrap();

    let result = ctx.execute_instruction(ix, &[&bank_authority]).unwrap();
    result.assert_success();
    
    ctx.svm.assert_account_exists(&bank_pda);
    ctx.svm.assert_account_exists(&bank_token_account_pda);
}

pub fn init_user_shares_and_assert(
    ctx: &mut AnchorContext,
    depositor: &Keypair,
    mint: &Pubkey,
) {
    let user_shares_pda = get_user_shares_pda(&depositor.pubkey(), mint);

    let init_user_accounts = accounts::InitUserShares {
        user: depositor.pubkey(),
        user_shares: user_shares_pda,
        mint: *mint,
        system_program: anchor_lang::system_program::ID,
    };

    let inx = ctx
        .program()
        .accounts(init_user_accounts)
        .args(args::InitUserShares {})
        .instruction()
        .unwrap();

    let transaction_result = ctx.execute_instruction(inx, &[&depositor]).unwrap();
    transaction_result.assert_success();

    ctx.svm.assert_account_exists(&user_shares_pda);
}

pub fn process_deposit_and_assert_states(
    ctx: &mut AnchorContext, 
    bank_authority: &Keypair,
    user_shares_pda: &Pubkey, //derive
    depositor: &Keypair, 
    mint: Pubkey, 
    user_ata: &Pubkey, //derive
    amount: u64
) -> Result<(TransactionResult, u64, u64), BankErrors> {
    println!("deposit amount {} for user {}", amount, depositor.pubkey());
    if amount < MIN_USDC_DEPOSIT {
        println!("---> amount to deposit is less than min - {}", amount < MIN_USDC_DEPOSIT);
    } 
    if amount > MAX_USDC_DEPOSIT {
        println!("---> amount to deposit is more than max - {}", amount > MAX_USDC_DEPOSIT);
    }
    // derive pdas here, not pass???
    let bank_pda = get_bank_account_pda(&mint);
    let bank_token_account_pda = get_bank_token_account_pda(&mint);

    let bank_state_before: Bank = ctx.get_account(&bank_pda).unwrap();
    let bank_token_account_before: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();

    
    let user_shares_before: UserShares = match ctx.get_account(&user_shares_pda) { //remove match
        Ok(user_shares) => user_shares,
        Err(error) => {
            match error {
                AccountError::AccountNotFound(_) => {
                    UserShares::default()
                }
                _ => panic!("Cant get user state")
            }
        }
    };
    let user_ata_before: TokenAccount = ctx.get_account(&user_ata).unwrap();

    let shares_to_mint = convert_assets_to_shares(amount, bank_state_before.total_deposit_shares, bank_state_before.total_deposits, false);
    

    let deposit_accounts = accounts::Deposit {
        user: depositor.pubkey(),
        user_shares: *user_shares_pda,
        bank_state: bank_pda,
        mint: mint,
        user_associated_token_account: *user_ata,
        bank_token_account: bank_token_account_pda,
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

    let transaction_result = match ctx
        .execute_instruction(inx, &[&depositor]) {
            Ok(res) => match res.is_success() {
                true => res,
                false => {
                    println!("---> failed deposit for user {} with error message: {:#?}", depositor.pubkey(), res.find_log("Error Message").unwrap());
                    return Err(BankErrors::DepositError)
                },
            },
            _ => return Err(BankErrors::DepositError),
        };

    // Assert bank state
    let bank_state_after: Bank = ctx.get_account(&bank_pda).unwrap();
    let bank_token_account_after: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    assert_eq!(bank_state_after.total_deposits, bank_state_before.total_deposits + amount);
    assert_eq!(bank_state_after.total_deposit_shares, bank_state_before.total_deposit_shares + shares_to_mint);
    assert_eq!(bank_token_account_after.amount, bank_token_account_before.amount + amount);

    // Assert user state
    let user_shares_after: UserShares = ctx.get_account(&user_shares_pda).unwrap();
    let user_ata_after: TokenAccount = ctx.get_account(&user_ata).unwrap();
    assert_eq!(user_shares_after.deposit_shares, user_shares_before.deposit_shares + shares_to_mint);
    assert_eq!(user_ata_after.amount, user_ata_before.amount - amount);

    Ok((transaction_result, amount, shares_to_mint))
}



pub fn process_withdraw_and_assert_states(
    ctx: &mut AnchorContext, 
    user_shares_pda: &Pubkey, //derive
    depositor: &Keypair, 
    bank_pda: &Pubkey, //do not pass, derive
    mint: &Pubkey, 
    bank_token_account_pda: &Pubkey, //derive
    user_ata: &Pubkey, //derive
    amount: u64
) -> Result<(TransactionResult, u64, u64, bool), BankErrors> {
    println!("withdraw amount {} for user {}", amount, depositor.pubkey());
    let bank_state_before: Bank = ctx.get_account(&bank_pda).unwrap();
    let bank_token_account_before: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    let user_shares_before: UserShares = ctx.get_account(&user_shares_pda).unwrap();
    let user_ata_before: TokenAccount = ctx.get_account(&user_ata).unwrap();

    let actual_assets_user_has = convert_shares_to_assets(
        user_shares_before.deposit_shares,
        bank_state_before.total_deposit_shares,
        bank_state_before.total_deposits
    );

    let withdraw_accounts = accounts::Withdraw {
        user: depositor.pubkey(),
        user_shares: *user_shares_pda,
        bank_state: *bank_pda,
        mint: *mint,
        user_associated_token_account: *user_ata,
        bank_token_account: *bank_token_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::system_program::ID,
    };

    let inx = ctx
            .program()
            .accounts(withdraw_accounts)
            .args(args::Withdraw {assets_amount_to_withdraw: amount})
            .instruction()
            .unwrap();
        
    let transaction_result = match ctx
        .execute_instruction(inx, &[&depositor]) {
            Ok(res) => match res.is_success() {
                true => res,
                false => {
                    println!("---> failed withdraw for user {} with error message: {:#?}", depositor.pubkey(), res.find_log("Error Message").unwrap());
                    return Err(BankErrors::WithdrawError)
                },
            },
            _ => return Err(BankErrors::WithdrawError),
        };

    // assert on state after depending on 'actual_assets_user_has'
    let mut actually_withdrawn_assets: u64 = 0;
    let mut shares_to_burn: u64 = 0;
    let mut user_is_closed: bool = false;
    if actual_assets_user_has < (amount + MIN_USDC_DEPOSIT) {
        // withdraw all
        actually_withdrawn_assets = actual_assets_user_has;
        shares_to_burn = user_shares_before.deposit_shares;
        ctx.svm.assert_account_closed(&user_shares_pda);
        user_is_closed = true;
    } else {
        // withdraw only claimed amount
        actually_withdrawn_assets = amount;
        shares_to_burn = convert_assets_to_shares(amount, bank_state_before.total_deposit_shares, bank_state_before.total_deposits, true);

        let user_shares_after: UserShares = ctx.get_account(&user_shares_pda).unwrap();
        assert_eq!(user_shares_after.deposit_shares, user_shares_before.deposit_shares - shares_to_burn);
    }

    let bank_state_after: Bank = ctx.get_account(&bank_pda).unwrap();
    assert_eq!(bank_state_after.total_deposits, bank_state_before.total_deposits - actually_withdrawn_assets);
    assert_eq!(bank_state_after.total_deposit_shares, bank_state_before.total_deposit_shares - shares_to_burn);

    let bank_token_account_after: TokenAccount = ctx.get_account(&bank_token_account_pda).unwrap();
    assert_eq!(bank_token_account_after.amount, bank_token_account_before.amount - actually_withdrawn_assets);

    let user_ata_after: TokenAccount = ctx.get_account(&user_ata).unwrap();
    assert_eq!(user_ata_after.amount, user_ata_before.amount + actually_withdrawn_assets);
    

    Ok((transaction_result, actually_withdrawn_assets, shares_to_burn, user_is_closed))
}

pub fn assert_and_record_deposit_event_and_snapshot(
    ctx: &AnchorContext, 
    deposit_result: &TransactionResult,
    depositor: &Pubkey,
    actually_deposited_amount: u64,
    shares_to_mint: u64,
    bank_pda: &Pubkey,
    step: u8, 
    utc_now: &str, 
    test_name: &str, 
    seed: u64
) {
    deposit_result.assert_event_emitted::<DepositEvent>();
    let deposit_event: DepositEvent = deposit_result.parse_event().unwrap();
    record_bank_event(&deposit_event, step, &utc_now, test_name, seed);
    assert_eq!(deposit_event.user, *depositor);
    assert_eq!(deposit_event.amount, actually_deposited_amount);
    assert_eq!(deposit_event.shares, shares_to_mint);

    // record current bank state in BankSnapshot struct
    let after_deposit_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    let bank_snapshot = BankSnapshot {
        user: deposit_event.user,//use depositor, not event data
        total_deposits: after_deposit_bank_state.total_deposits,
        total_deposit_shares: after_deposit_bank_state.total_deposit_shares,
        timestamp: deposit_event.timestamp,
    };
    record_bank_event(&bank_snapshot, step, &utc_now, test_name, seed);
}

pub fn assert_and_record_withdraw_event_and_snapshot( //unify with fn for deposit event, use match
    ctx: &AnchorContext, 
    withdraw_result: &TransactionResult,
    depositor: &Pubkey,
    actually_withdrawn_amount: u64,
    shares_to_burn: u64,  
    bank_pda: &Pubkey,
    step: u8, 
    utc_now: &str, 
    test_name: &str,
    seed: u64,
) {
    withdraw_result.assert_event_emitted::<WithdrawEvent>();
    let withdraw_event: WithdrawEvent = withdraw_result.parse_event().unwrap();
    assert_eq!(withdraw_event.user, *depositor);
    assert_eq!(withdraw_event.amount, actually_withdrawn_amount);
    assert_eq!(withdraw_event.shares, shares_to_burn);
    record_bank_event(&withdraw_event, step, &utc_now, test_name, seed);

    // record current bank state in BankSnapshot struct
    let after_withdraw_bank_state: Bank = ctx.get_account(&bank_pda).unwrap();
    let bank_snapshot = BankSnapshot {
        user: withdraw_event.user,//user depositor, not event data
        total_deposits: after_withdraw_bank_state.total_deposits,
        total_deposit_shares: after_withdraw_bank_state.total_deposit_shares,
        timestamp: withdraw_event.timestamp,
    };
    record_bank_event(&bank_snapshot, step, &utc_now, test_name, seed);
}