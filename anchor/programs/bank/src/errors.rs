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

    #[msg("Not Enough Amount To Deposit")]
    NotEnoughAmountToDeposit,

    #[msg("Too Big Amount To Deposit")]
    TooBigAmountToDeposit,

    #[msg("Bank Underfunded")]
    BankUnderfunded,

    #[msg("Overflow")]
    Overflow,

    #[msg("Division By Zero")]
    DivisionByZero,

    #[msg("Unauthorized Access")]
    UnauthorizedAccess,
}