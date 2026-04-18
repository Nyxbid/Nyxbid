use anchor_lang::prelude::*;

#[error_code]
pub enum NyxbidError {
    #[msg("intent is not open")]
    IntentNotOpen,
    #[msg("intent is not resolved")]
    IntentNotResolved,
    #[msg("reveal deadline has not passed")]
    RevealDeadlineNotReached,
    #[msg("resolve deadline has not passed")]
    ResolveDeadlineNotReached,
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
}
