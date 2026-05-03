use anchor_lang::prelude::*;

#[error_code]
pub enum NyxbidError {
    #[msg("intent is not open")]
    IntentNotOpen,
    #[msg("intent is not resolved")]
    IntentNotResolved,
    #[msg("reveal deadline has not passed")]
    RevealDeadlineNotReached,
    #[msg("reveal deadline has passed")]
    RevealDeadlinePassed,
    #[msg("resolve deadline has not passed")]
    ResolveDeadlineNotReached,
    #[msg("resolve deadline has passed")]
    ResolveDeadlinePassed,
    #[msg("quote commitment does not match revealed price")]
    CommitmentMismatch,
    #[msg("quote already revealed")]
    AlreadyRevealed,
    #[msg("price breaches taker limit")]
    LimitBreached,
    #[msg("insufficient escrow deposit")]
    InsufficientDeposit,
    #[msg("escrow already settled")]
    AlreadySettled,
    #[msg("unauthorized")]
    Unauthorized,
    #[msg("invalid side discriminator")]
    InvalidSide,
    #[msg("math overflow")]
    MathOverflow,
    #[msg("zero amount")]
    ZeroAmount,
    #[msg("wrong mint for the locked leg")]
    WrongLockMint,
    #[msg("reveal deadline must precede resolve deadline")]
    BadDeadlines,
    #[msg("revealed size does not match intent size")]
    SizeMismatch,
    #[msg("maker has not funded escrow")]
    MakerNotFunded,
    #[msg("maker already funded")]
    MakerAlreadyFunded,
    #[msg("caller is not the winning maker")]
    NotWinningMaker,
    #[msg("quote has not been revealed")]
    NotRevealed,
    #[msg("settle deadline has passed")]
    SettleDeadlinePassed,
    #[msg("settle deadline has not passed")]
    SettleDeadlineNotReached,
    #[msg("fund amount does not match revealed notional")]
    WrongFundAmount,
    #[msg("expire_no_maker requires winning_quote and reputation accounts when a winner exists")]
    MissingWinnerAccounts,
    #[msg("winner accounts must not be passed when no winning quote was selected")]
    UnexpectedWinnerAccounts,
    #[msg("reveal deadline must be in the future and leave a minimum submit window")]
    SubmitWindowTooShort,
}
