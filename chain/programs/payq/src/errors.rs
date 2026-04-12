use anchor_lang::prelude::*;

#[error_code]
pub enum PayqError {
    #[msg("Label must be 32 characters or fewer")]
    LabelTooLong,
    #[msg("Field exceeds maximum length")]
    FieldTooLong,
    #[msg("Amount exceeds per-transaction limit")]
    ExceedsPerTxLimit,
    #[msg("Cumulative spend exceeds daily limit")]
    ExceedsDailyLimit,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Vault is paused")]
    VaultPaused,
}
