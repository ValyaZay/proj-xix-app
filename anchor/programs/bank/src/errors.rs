use anchor_lang::prelude::*;

#[error_code]
pub enum BankError {
    #[msg("Bank Is Not Initialized")]
    BankIsNotInitialized,

    #[msg("Mint For Bank Is Wrong")]
    MintForBankIsWrong,

    #[msg("NotEnoughTokensTransferred")]
    NotEnoughTokensTransferred,
}

#[error_code]
pub enum MathError {
    #[msg("Overflow")]
    Overflow,

    #[msg("Division By Zero")]
    DivisionByZero
}