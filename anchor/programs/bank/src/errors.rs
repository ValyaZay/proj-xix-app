use anchor_lang::prelude::*;

#[error_code]
pub enum BankErrors {
    #[msg("Bank Is Not Initialized")]
    BankIsNotInitialized,

    #[msg("Mint For Bank Is Wrong")]
    MintForBankIsWrong,

    #[msg("User Ata For Bank Is Wrong")]
    UserAtaForBankIsWrong,

    #[msg("Not Enough Tokens Transferred")]
    NotEnoughTokensTransferred,

    #[msg("Zero Shares From Amount")]
    ZeroSharesFromAmount,

    #[msg("Bank Token Account Owner Is Wrong")]
    BankTokenAccountOwnerIsWrong,

    #[msg("Zero Amount To Deposit")]
    ZeroAmountToDeposit,

    #[msg("Bank Underfunded")]
    BankUnderfunded,

    #[msg("Overflow")]
    Overflow,

    #[msg("Division By Zero")]
    DivisionByZero
}