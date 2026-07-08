use anchor_lang::prelude::*;

#[error_code]
pub enum BankErrors {
    #[msg("Bank Is Not Initialized")]
    BankIsNotInitialized,

    #[msg("Mint For Bank Is Wrong")]
    MintForBankIsWrong,

    #[msg("Mint For User Shares Is Wrong")]
    MintForUserSharesIsWrong,

    #[msg("User Ata For Bank Is Wrong")]
    UserAtaForBankIsWrong,

    #[msg("Not Enough Tokens Transferred")]
    NotEnoughTokensTransferred,

    #[msg("Zero Shares From Deposit")]
    ZeroSharesFromDeposit,

    #[msg("Zero Shares To Burn")]
    ZeroSharesToBurn,

    #[msg("Not Enough Amount To Deposit")]
    NotEnoughAmountToDeposit,

    #[msg("Too Big Amount To Deposit")]
    TooBigAmountToDeposit,

    #[msg("Bank Underfunded")]
    BankUnderfunded,

    #[msg("Invalid Bank State")]
    InvalidBankState,

    #[msg("Overflow")]
    Overflow,

    #[msg("Division By Zero")]
    DivisionByZero,

    #[msg("Unauthorized Access")]
    UnauthorizedAccess,

    #[msg("Insufficient User Shares")]
    InsufficientUserShares,

    #[msg("Zero Amount To Withdraw")]
    ZeroAmountToWithdraw,

    #[msg("Withdraw Error")]
    WithdrawError,

    #[msg("Deposit Error")]
    DepositError,

    #[msg("User Has No Shares")]
    UserHasNoShares,
}