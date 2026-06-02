use anchor_lang::prelude::*;
use anchor_spl:: {
    associated_token::AssociatedToken,
    token_interface::{TokenAccount, Mint, TokenInterface}
};

pub mod tests;

pub mod state;
pub use state::*;

pub mod events;
pub use events::*;

pub mod errors;
pub use errors::*;

pub mod constants;
pub use constants::*;

pub mod shares_math;
pub use shares_math::*;

pub mod transfer_helpers;
pub use transfer_helpers::*;

declare_id!("CTb99n5SgTTkjA2i6zAKvDxd7QbbmCgj4RbgFfpmX6FZ");

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
            is_initialized: true });
        // init bank token account 
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, BankErrors::ZeroAmountToDeposit);

        let bank_state = &mut ctx.accounts.bank_state;
        require!(bank_state.is_initialized, BankErrors::BankIsNotInitialized);

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

       // require!()

        // deposit - TODO - add test
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
        let shares = if bank_state.total_deposits == 0 {
            amount
        } else {
            convert_assets_to_shares(amount, bank_state.total_deposit_shares, bank_state.total_deposits)
        };

        require!(shares > 0, BankErrors::ZeroSharesFromAmount);

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
        seeds = [SEED_BANK_STATE, mint.key().as_ref()],
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
        seeds = [SEED_BANK_STATE, mint.key().as_ref()],
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
        constraint = bank_token_account.mint == mint.key() @ BankErrors::MintForBankIsWrong,
        constraint = bank_token_account.owner == bank_token_account.key() @ BankErrors::BankTokenAccountOwnerIsWrong,
        bump,
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>, 
}
