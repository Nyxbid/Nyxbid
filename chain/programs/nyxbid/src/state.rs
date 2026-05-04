use anchor_lang::prelude::*;

pub const INTENT_SEED: &[u8] = b"intent";
pub const QUOTE_SEED: &[u8] = b"quote";
pub const ESCROW_SEED: &[u8] = b"escrow";
pub const TAKER_VAULT_SEED: &[u8] = b"taker_vault";
pub const MAKER_VAULT_SEED: &[u8] = b"maker_vault";
pub const RECEIPT_SEED: &[u8] = b"receipt";
pub const REPUTATION_SEED: &[u8] = b"reputation";

/// Fixed-point scale for `limit_price` and `revealed_price`.
/// A price of 1.0 quote per base is encoded as `PRICE_SCALE`.
/// `quote_amount = size * price / PRICE_SCALE` (computed in u128).
pub const PRICE_SCALE: u64 = 1_000_000;

/// Minimum gap (in seconds) between `clock.unix_timestamp` and
/// `reveal_deadline` at intent creation time. Prevents a taker from
/// locking funds into an intent whose submit window is already closed
/// or absurdly short. 5 seconds gives makers a meaningful chance to
/// observe the intent and submit a sealed quote.
pub const MIN_SUBMIT_WINDOW_SECS: i64 = 5;

#[account]
#[derive(InitSpace)]
pub struct Intent {
    pub taker: Pubkey,
    pub side: u8,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub size: u64,
    pub limit_price: u64,
    /// Submit window: clients can submit sealed quotes while
    /// `clock < reveal_deadline`.
    pub reveal_deadline: i64,
    /// Reveal window: makers can reveal between `reveal_deadline` and
    /// `resolve_deadline`. Each valid reveal can replace the current
    /// winner if it improves the price.
    pub resolve_deadline: i64,
    /// Settle window: after `resolve_deadline` the winner is final and
    /// must fund + settle before `settle_deadline`. After that, the
    /// taker can expire and recover their funds (and the would-be
    /// winner takes a `failed_reveals` reputation hit).
    pub settle_deadline: i64,
    pub commitment_root: [u8; 32],
    pub status: u8,
    /// Best valid revealed quote so far. Default Pubkey if no maker has
    /// successfully revealed.
    pub winning_quote: Pubkey,
    /// Best price revealed so far, in PRICE_SCALE units. Cached on the
    /// Intent so reveal_quote can compare without deserializing the
    /// previous winning quote account.
    pub winning_price: u64,
    /// Cached PDA bumps for cheap signer-seeds derivation later.
    pub bump: u8,
    pub escrow_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Quote {
    pub intent: Pubkey,
    pub maker: Pubkey,
    pub commitment: [u8; 32],
    pub revealed_price: u64,
    pub revealed_size: u64,
    pub nonce: [u8; 32],
    pub revealed: bool,
    pub maker_funded: bool,
    pub bump: u8,
}

/// Per-intent escrow record. Owns two vault token accounts:
///   - taker_vault: holds the leg the taker locked at create_intent.
///   - maker_vault: holds the leg the winning maker locked before reveal.
/// Settle drains both vaults atomically and closes them.
#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub intent: Pubkey,
    pub taker_amount: u64,
    pub taker_mint: Pubkey,
    pub maker: Pubkey,
    pub maker_amount: u64,
    pub maker_mint: Pubkey,
    pub settled: bool,
    pub bump: u8,
    /// Cached PDA bump for the taker_vault token account so later
    /// instructions can pass `bump = escrow.taker_vault_bump` and skip
    /// the ~1500 CU find_program_address re-derivation.
    pub taker_vault_bump: u8,
    /// Same for maker_vault. Set during fund_maker_escrow; remains 0
    /// until the winning maker funds.
    pub maker_vault_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Receipt {
    pub intent: Pubkey,
    pub taker: Pubkey,
    pub maker: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub filled_size: u64,
    pub filled_price: u64,
    pub settled_at: i64,
    pub bump: u8,
}

/// Per-maker reputation. Phase 1 stores raw counters only.
/// Phase 3 will map A2A agent identities to this PDA and add scoring.
#[account]
#[derive(InitSpace)]
pub struct Reputation {
    pub maker: Pubkey,
    pub quotes_submitted: u64,
    pub quotes_won: u64,
    pub settled_count: u64,
    pub failed_reveals: u64,
    pub bump: u8,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IntentStatus {
    Open = 0,
    Resolved = 1,
    Settled = 2,
    Cancelled = 3,
    Expired = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

impl Side {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Side::Buy),
            1 => Some(Side::Sell),
            _ => None,
        }
    }
}

/// Compute the quote-side notional for a given size and price.
/// Returns `None` on overflow.
pub fn quote_notional(size: u64, price: u64) -> Option<u64> {
    let n = (size as u128).checked_mul(price as u128)?;
    let n = n.checked_div(PRICE_SCALE as u128)?;
    u64::try_from(n).ok()
}
