use anchor_lang::prelude::*;
use anchor_spl:: {
    associated_token::AssociatedToken,
    token_interface::{TokenAccount, Mint, TokenInterface, TransferChecked, self}
};

pub mod tests;

pub mod state;
pub use state::*;

pub mod events;
use events::*;

pub mod errors;
pub use errors::*;

pub mod constants;
pub use constants::*;

pub mod shares_math;
pub use shares_math::*;

pub mod transfer_helpers;
pub use transfer_helpers::*;

declare_id!("cDNe9N78wv8rfaFHNKwmarce5JhJ1HmETRtkYazsm2v");

#[program]
pub mod bank {
    use super::*;

    pub fn init_bank(ctx: Context<InitBank>) -> Result<()> {
        let bank = &mut ctx.accounts.bank_state;
        bank.set_inner(Bank { 
            authority: ctx.accounts.authority.key(),
            mint: ctx.accounts.mint.key(),
            total_deposits: 0, 
            total_deposit_shares: 0,
        });
        // init bank token account 
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount >= MIN_USDC_DEPOSIT, BankErrors::NotEnoughAmountToDeposit);
        require!(amount <= MAX_USDC_DEPOSIT, BankErrors::TooBigAmountToDeposit);

        let bank_state = &mut ctx.accounts.bank_state;
        
        // invariant check
        require!(ctx.accounts.bank_token_account.amount >= bank_state.total_deposits, BankErrors::BankUnderfunded);

        let user_state = &mut ctx.accounts.user_state;
        if !user_state.is_initialized {
            user_state.set_inner(User { 
                user: ctx.accounts.user.key(), 
                deposit_usdc_shares: 0,
                is_initialized: true
            });
        }

        // safety invariant
        require_keys_eq!(user_state.user, ctx.accounts.user.key(), BankErrors::UnauthorizedAccess);

        let received = transfer_from_ata_to_token_account(
            &mut ctx.accounts.bank_token_account,
            ctx.accounts.user_associated_token_account.to_account_info(),
            ctx.accounts.user.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.mint.decimals,
            amount
        )?;
        
        let amount = received;

        // calculate shares
        let shares = convert_assets_to_shares(amount, bank_state.total_deposit_shares, bank_state.total_deposits, false);

        require!(shares > 0, BankErrors::ZeroSharesFromDeposit);

        // update bank and user state
        bank_state.total_deposit_shares = bank_state.total_deposit_shares.checked_add(shares).ok_or(BankErrors::Overflow)?;
        bank_state.total_deposits = bank_state.total_deposits.checked_add(amount).ok_or(BankErrors::Overflow)?;
        user_state.deposit_usdc_shares = user_state.deposit_usdc_shares.checked_add(shares).ok_or(BankErrors::Overflow)?;

        // emit event
        emit!(DepositEvent {
            user: user_state.user,
            amount: amount,
            shares: shares,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, assets_amount_to_withdraw: u64) -> Result<()> {
        require!(assets_amount_to_withdraw > 0, BankErrors::ZeroAmountToWithdraw);
        let user_state = &mut ctx.accounts.user_state;
        let bank_state = &mut ctx.accounts.bank_state;
        
        // invariant check
        require!(ctx.accounts.bank_token_account.amount >= bank_state.total_deposits, BankErrors::BankUnderfunded);
        require!(bank_state.total_deposit_shares >= user_state.deposit_usdc_shares, BankErrors::InvalidBankState);

        // how many assets does the user have?
        let actual_assets_user_has = convert_shares_to_assets(user_state.deposit_usdc_shares, bank_state.total_deposit_shares, bank_state.total_deposits);
        
        // if assets_amount_to_withdraw + MIN_DEPOSIT_AMOUNT > user has => withdraw all
        // if assets_amount_to_withdraw + MIN_DEPOSIT_AMOUNT <= user has => withdraw assets_amount_to_withdraw
        let (actual_assets_amount_to_withdraw, actual_shares_amount_to_burn) = 
            if actual_assets_user_has < assets_amount_to_withdraw.checked_add(MIN_USDC_DEPOSIT).ok_or(BankErrors::Overflow)?
            {
                (actual_assets_user_has, user_state.deposit_usdc_shares)
            } else {
                (assets_amount_to_withdraw, convert_assets_to_shares(assets_amount_to_withdraw, bank_state.total_deposit_shares, bank_state.total_deposits, true))
            };
        require!(actual_shares_amount_to_burn > 0, BankErrors::ZeroSharesToBurn);
        require!(user_state.deposit_usdc_shares >= actual_shares_amount_to_burn, BankErrors::InsufficientUserShares);
        require!(bank_state.total_deposits >= actual_assets_amount_to_withdraw, BankErrors::BankUnderfunded);

        // transfer from bank token account PDA to user ata
        let transfer_cpi_accounts = TransferChecked {
            from: ctx.accounts.bank_token_account.to_account_info(),
            to: ctx.accounts.user_associated_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            authority: ctx.accounts.bank_token_account.to_account_info(),
        };

        let mint_key = ctx.accounts.mint.key();

        let signer_seeds: &[&[&[u8]]] = &[
            &[
                SEED_BANK_TOKEN_ACCOUNT, mint_key.as_ref(),
                &[
                    ctx.bumps.bank_token_account,
                ]
            ]
        ];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            transfer_cpi_accounts,
            signer_seeds);
        
        token_interface::transfer_checked(
            cpi_ctx, 
            actual_assets_amount_to_withdraw, 
            ctx.accounts.mint.decimals)?;

        // update bank_state
        bank_state.total_deposits = bank_state.total_deposits.checked_sub(actual_assets_amount_to_withdraw).ok_or(BankErrors::Overflow)?;
        bank_state.total_deposit_shares = bank_state.total_deposit_shares.checked_sub(actual_shares_amount_to_burn).ok_or(BankErrors::Overflow)?;

        // update user_state
        user_state.deposit_usdc_shares = user_state.deposit_usdc_shares.checked_sub(actual_shares_amount_to_burn).ok_or(BankErrors::Overflow)?;

        // emit event
        emit!(WithdrawEvent {
            user: user_state.user,
            amount: actual_assets_amount_to_withdraw,
            shares: actual_shares_amount_to_burn, 
            timestamp: Clock::get()?.unix_timestamp,
        });

        // close user state if shares == 0
        if user_state.deposit_usdc_shares == 0 {
            user_state.close(ctx.accounts.user.to_account_info())?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitBank<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = Bank::DISCRIMINATOR.len() + Bank::INIT_SPACE,
        seeds = [SEED_BANK_STATE, mint.key().as_ref(), authority.key().as_ref()],
        bump
    )]
    pub bank_state: Account<'info, Bank>,

    #[account(
        init,
        payer = authority, 
        token::mint = mint,
        token::authority = bank_token_account,
        seeds = [SEED_BANK_TOKEN_ACCOUNT, mint.key().as_ref()],
        bump
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = User::DISCRIMINATOR.len() + User::INIT_SPACE,
        seeds = [SEED_USER_STATE, user.key().as_ref()],
        bump,
    )]
    pub user_state: Account<'info, User>,

    #[account(
        mut,
        has_one = mint @ BankErrors::MintForBankIsWrong,
        constraint = bank_state.mint.key() == user_associated_token_account.mint.key() @ BankErrors::UserAtaForBankIsWrong,
        seeds = [SEED_BANK_STATE, mint.key().as_ref(), bank_state.authority.as_ref()],
        bump,
    )]
    pub bank_state: Account<'info, Bank>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_associated_token_account: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [SEED_BANK_TOKEN_ACCOUNT, mint.key().as_ref()],
        token::authority = bank_token_account,
        token::mint = mint,
        bump,
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>, 
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        has_one = user @ BankErrors::UnauthorizedAccess,
        seeds = [SEED_USER_STATE, user.key().as_ref()],
        bump,
    )]
    pub user_state: Account<'info, User>,

    #[account(
        mut,
        has_one = mint @ BankErrors::MintForBankIsWrong,
        constraint = bank_state.mint.key() == user_associated_token_account.mint.key() @ BankErrors::UserAtaForBankIsWrong,
        seeds = [SEED_BANK_STATE, mint.key().as_ref(), bank_state.authority.as_ref()],
        bump,
    )]
    pub bank_state: Account<'info, Bank>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_associated_token_account: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [SEED_BANK_TOKEN_ACCOUNT, mint.key().as_ref()],
        token::authority = bank_token_account,
        token::mint = mint,
        bump,
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>, 
}