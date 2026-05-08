//! Program error code → human-readable name/message mapping.
//!
//! Mirrors `chain/programs/nyxbid/src/error.rs`. Anchor adds an offset
//! of 6000 to every `#[error_code]` enum variant, so the on-chain code
//! that surfaces in `TransactionError::InstructionError(_, Custom(N))`
//! is `6000 + variant_index`.

/// Anchor-side base offset for custom errors.
pub const CUSTOM_ERROR_BASE: u32 = 6000;

/// Strongly-typed program errors. Variant order matches `NyxbidError` in
/// the chain crate; the discriminant is the on-chain code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[repr(u32)]
pub enum NyxbidError {
    #[error("intent is not open")]
    IntentNotOpen = 6000,
    #[error("intent is not resolved")]
    IntentNotResolved = 6001,
    #[error("reveal deadline has not passed")]
    RevealDeadlineNotReached = 6002,
    #[error("reveal deadline has passed")]
    RevealDeadlinePassed = 6003,
    #[error("resolve deadline has not passed")]
    ResolveDeadlineNotReached = 6004,
    #[error("resolve deadline has passed")]
    ResolveDeadlinePassed = 6005,
    #[error("quote commitment does not match revealed price")]
    CommitmentMismatch = 6006,
    #[error("quote already revealed")]
    AlreadyRevealed = 6007,
    #[error("price breaches taker limit")]
    LimitBreached = 6008,
    #[error("insufficient escrow deposit")]
    InsufficientDeposit = 6009,
    #[error("escrow already settled")]
    AlreadySettled = 6010,
    #[error("unauthorized")]
    Unauthorized = 6011,
    #[error("invalid side discriminator")]
    InvalidSide = 6012,
    #[error("math overflow")]
    MathOverflow = 6013,
    #[error("zero amount")]
    ZeroAmount = 6014,
    #[error("wrong mint for the locked leg")]
    WrongLockMint = 6015,
    #[error("reveal deadline must precede resolve deadline")]
    BadDeadlines = 6016,
    #[error("revealed size does not match intent size")]
    SizeMismatch = 6017,
    #[error("maker has not funded escrow")]
    MakerNotFunded = 6018,
    #[error("maker already funded")]
    MakerAlreadyFunded = 6019,
    #[error("caller is not the winning maker")]
    NotWinningMaker = 6020,
    #[error("quote has not been revealed")]
    NotRevealed = 6021,
    #[error("settle deadline has passed")]
    SettleDeadlinePassed = 6022,
    #[error("settle deadline has not passed")]
    SettleDeadlineNotReached = 6023,
    #[error("fund amount does not match revealed notional")]
    WrongFundAmount = 6024,
    #[error("expire_no_maker requires winning_quote and reputation accounts when a winner exists")]
    MissingWinnerAccounts = 6025,
    #[error("winner accounts must not be passed when no winning quote was selected")]
    UnexpectedWinnerAccounts = 6026,
    #[error("reveal deadline must be in the future and leave a minimum submit window")]
    SubmitWindowTooShort = 6027,
    #[error("buy-side settle requires taker_refund_destination when filled_price < limit_price")]
    MissingRefundDestination = 6028,
}

impl NyxbidError {
    /// Resolve a raw on-chain code (`6000 + variant_index`) to a typed
    /// error. Returns `None` for codes outside the program's range.
    pub fn from_code(code: u32) -> Option<Self> {
        use NyxbidError::*;
        Some(match code {
            6000 => IntentNotOpen,
            6001 => IntentNotResolved,
            6002 => RevealDeadlineNotReached,
            6003 => RevealDeadlinePassed,
            6004 => ResolveDeadlineNotReached,
            6005 => ResolveDeadlinePassed,
            6006 => CommitmentMismatch,
            6007 => AlreadyRevealed,
            6008 => LimitBreached,
            6009 => InsufficientDeposit,
            6010 => AlreadySettled,
            6011 => Unauthorized,
            6012 => InvalidSide,
            6013 => MathOverflow,
            6014 => ZeroAmount,
            6015 => WrongLockMint,
            6016 => BadDeadlines,
            6017 => SizeMismatch,
            6018 => MakerNotFunded,
            6019 => MakerAlreadyFunded,
            6020 => NotWinningMaker,
            6021 => NotRevealed,
            6022 => SettleDeadlinePassed,
            6023 => SettleDeadlineNotReached,
            6024 => WrongFundAmount,
            6025 => MissingWinnerAccounts,
            6026 => UnexpectedWinnerAccounts,
            6027 => SubmitWindowTooShort,
            6028 => MissingRefundDestination,
            _ => return None,
        })
    }

    /// Numeric code as it appears in `Custom(...)`.
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// Errors raised by [`crate::AnchorAccount::try_decode`] and
/// [`crate::AnchorEvent::try_decode`].
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("buffer too short: got {got} bytes, want at least {want}")]
    TooShort { got: usize, want: usize },
    #[error("unexpected discriminator")]
    WrongDiscriminator,
    #[error("borsh decode error: {0}")]
    Borsh(#[from] std::io::Error),
}
