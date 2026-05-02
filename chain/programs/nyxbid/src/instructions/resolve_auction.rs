use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

use crate::error::NyxbidError;
use crate::events::AuctionResolved;
use crate::state::{
    quote_notional, Escrow, Intent, IntentStatus, Quote, Reputation, Side, ESCROW_SEED,
    REPUTATION_SEED,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ResolveAuctionParams {
    pub revealed_price: u64,
    pub revealed_size: u64,
    pub nonce: [u8; 32],
}

#[derive(Accounts)]
pub struct ResolveAuction<'info> {
    #[account(mut)]
    pub resolver: Signer<'info>,

    #[account(
        mut,
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Box<Account<'info, Intent>>,

    #[account(
        mut,
        constraint = winning_quote.intent == intent.key(),
        constraint = !winning_quote.revealed @ NyxbidError::AlreadyRevealed,
        constraint = winning_quote.maker_funded @ NyxbidError::MakerNotFunded,
    )]
    pub winning_quote: Box<Account<'info, Quote>>,

    #[account(
        seeds = [ESCROW_SEED, intent.key().as_ref()],
        bump = intent.escrow_bump,
        constraint = !escrow.settled @ NyxbidError::AlreadySettled,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        seeds = [REPUTATION_SEED, winning_quote.maker.as_ref()],
        bump = reputation.bump,
        constraint = reputation.maker == winning_quote.maker @ NyxbidError::Unauthorized,
    )]
    pub reputation: Box<Account<'info, Reputation>>,
}

pub(crate) fn handler(ctx: Context<ResolveAuction>, params: ResolveAuctionParams) -> Result<()> {
    let clock = Clock::get()?;
    let intent = &mut ctx.accounts.intent;
    let quote = &mut ctx.accounts.winning_quote;
    let escrow = &ctx.accounts.escrow;

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

    // Verify the maker's escrow holds the right amount of the right mint.
    let (expected_maker_amount, expected_maker_mint) = match side {
        // Buy: maker delivers base_mint, sized by revealed_size.
        Side::Buy => (params.revealed_size, intent.base_mint),
        // Sell: maker delivers quote_mint, sized by revealed_size * revealed_price.
        Side::Sell => (
            quote_notional(params.revealed_size, params.revealed_price)
                .ok_or(NyxbidError::MathOverflow)?,
            intent.quote_mint,
        ),
    };
    require_keys_eq!(
        escrow.maker_mint,
        expected_maker_mint,
        NyxbidError::WrongLockMint
    );
    require!(
        escrow.maker_amount == expected_maker_amount,
        NyxbidError::InsufficientDeposit
    );

    quote.revealed_price = params.revealed_price;
    quote.revealed_size = params.revealed_size;
    quote.nonce = params.nonce;
    quote.revealed = true;

    intent.status = IntentStatus::Resolved as u8;
    intent.winning_quote = quote.key();

    let rep = &mut ctx.accounts.reputation;
    rep.quotes_won = rep.quotes_won.saturating_add(1);

    emit!(AuctionResolved {
        intent: intent.key(),
        winning_quote: quote.key(),
        clearing_price: quote.revealed_price,
        filled_size: quote.revealed_size,
    });

    Ok(())
}
