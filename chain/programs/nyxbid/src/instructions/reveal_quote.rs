use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

use crate::error::NyxbidError;
use crate::events::QuoteRevealed;
use crate::state::{Intent, IntentStatus, Quote, Side};

/// One maker reveals their sealed quote during the reveal window.
///
/// Lifecycle:
///   - allowed only while clock is in [reveal_deadline, resolve_deadline),
///   - caller must be the quote's maker,
///   - quote must not already be revealed,
///   - commitment hash must match (price, size, nonce),
///   - revealed_size must equal intent.size (no partial fills in Phase 1),
///   - revealed_price must clear the taker's limit.
///
/// If the revealed quote improves on intent.winning_price (lower for buy,
/// higher for sell, or first valid reveal), the program updates
/// intent.winning_quote and intent.winning_price. Otherwise the reveal
/// is still recorded but the winner is unchanged.
///
/// No funds move here. No reputation counter changes here either - the
/// quotes_won bump fires when the actual winner first calls
/// fund_maker_escrow after resolve_deadline.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RevealQuoteParams {
    pub revealed_price: u64,
    pub revealed_size: u64,
    pub nonce: [u8; 32],
}

#[derive(Accounts)]
pub struct RevealQuote<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Box<Account<'info, Intent>>,

    #[account(
        mut,
        constraint = quote.intent == intent.key(),
        constraint = quote.maker == maker.key() @ NyxbidError::Unauthorized,
        constraint = !quote.revealed @ NyxbidError::AlreadyRevealed,
    )]
    pub quote: Box<Account<'info, Quote>>,
}

pub(crate) fn handler(ctx: Context<RevealQuote>, params: RevealQuoteParams) -> Result<()> {
    let clock = Clock::get()?;
    let intent = &mut ctx.accounts.intent;
    let quote = &mut ctx.accounts.quote;

    require!(
        clock.unix_timestamp >= intent.reveal_deadline,
        NyxbidError::RevealDeadlineNotReached
    );
    require!(
        clock.unix_timestamp < intent.resolve_deadline,
        NyxbidError::ResolveDeadlinePassed
    );

    // Verify the commitment.
    let computed = hashv(&[
        &params.revealed_price.to_le_bytes(),
        &params.revealed_size.to_le_bytes(),
        &params.nonce,
    ])
    .to_bytes();
    require!(
        computed == quote.commitment,
        NyxbidError::CommitmentMismatch
    );

    // Phase 1: enforce full size only. Partial fills are a future feature.
    require!(
        params.revealed_size == intent.size,
        NyxbidError::SizeMismatch
    );

    // Verify the revealed price clears the taker's limit.
    let side = Side::from_u8(intent.side).ok_or(NyxbidError::InvalidSide)?;
    let clears = match side {
        Side::Buy => params.revealed_price <= intent.limit_price,
        Side::Sell => params.revealed_price >= intent.limit_price,
    };
    require!(clears, NyxbidError::LimitBreached);

    quote.revealed_price = params.revealed_price;
    quote.revealed_size = params.revealed_size;
    quote.nonce = params.nonce;
    quote.revealed = true;

    // Best-bid replacement. First valid reveal becomes the winner; later
    // reveals replace only if strictly better (lower price for buy, higher
    // for sell). Ties keep the earlier winner so makers can't grief by
    // reposting the same price.
    let no_winner_yet = intent.winning_quote == Pubkey::default();
    let improves = match side {
        Side::Buy => params.revealed_price < intent.winning_price,
        Side::Sell => params.revealed_price > intent.winning_price,
    };
    if no_winner_yet || improves {
        intent.winning_quote = quote.key();
        intent.winning_price = params.revealed_price;
    }

    emit!(QuoteRevealed {
        intent: intent.key(),
        quote: quote.key(),
        maker: quote.maker,
        revealed_price: params.revealed_price,
        revealed_size: params.revealed_size,
        is_best: intent.winning_quote == quote.key(),
    });

    Ok(())
}
