use anchor_lang::prelude::*;
use anchor_spl:: {
    token_interface::{TokenAccount, TransferChecked, self}
};

use crate::{BankError, MIN_USDC_DEPOSIT, MathError};

// NOT TESTED YET!!!
pub fn transfer_from_ata_to_token_account<'info>(
    bank_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    user_ata_account_info: AccountInfo<'info>,
    user_account_info: AccountInfo<'info>,
    mint_account_info: AccountInfo<'info>,
    token_program_account_info: AccountInfo<'info>,
    decimals: u8,
    amount: u64,
) -> Result<u64> {
    let bank_balance_before_transfer = bank_token_account.amount;

    let transfer_cpi_accounts = TransferChecked {
    from: user_ata_account_info,
    to: bank_token_account.to_account_info(),
    mint: mint_account_info,
    authority: user_account_info,
    };

    let cpi_context = CpiContext::new(
        token_program_account_info,
        transfer_cpi_accounts,
    );

    token_interface::transfer_checked(cpi_context, amount, decimals)?;

    bank_token_account.reload()?;
    let bank_balance_after_transfer = bank_token_account.amount;

    let received = bank_balance_after_transfer.checked_sub(bank_balance_before_transfer).ok_or(MathError::Overflow)?;

    require!(received >= MIN_USDC_DEPOSIT, BankError::NotEnoughTokensTransferred);

    Ok(received)
}