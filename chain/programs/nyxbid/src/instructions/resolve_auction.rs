use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

use crate::error::NyxbidError;
use crate::events::AuctionResolved;
use crate::state::{Intent, IntentStatus, Quote, Side};

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
    pub intent: Account<'info, Intent>,

    #[account(
        mut,
        constraint = winning_quote.intent == intent.key(),
        constraint = !winning_quote.revealed @ NyxbidError::AlreadyRevealed,
    )]
    pub winning_quote: Account<'info, Quote>,
}

pub(crate) fn handler(ctx: Context<ResolveAuction>, params: ResolveAuctionParams) -> Result<()> {
    let clock = Clock::get()?;
    let intent = &mut ctx.accounts.intent;
    let quote = &mut ctx.accounts.winning_quote;

    require!(
        clock.unix_timestamp >= intent.reveal_deadline,
        NyxbidError::RevealDeadlineNotReached
    );

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

    let clears = match intent.side {
        s if s == Side::Buy as u8 => params.revealed_price <= intent.limit_price,
        s if s == Side::Sell as u8 => params.revealed_price >= intent.limit_price,
        _ => false,
    };
    require!(clears, NyxbidError::LimitBreached);

    quote.revealed_price = params.revealed_price;
    quote.revealed_size = params.revealed_size;
    quote.nonce = params.nonce;
    quote.revealed = true;

    intent.status = IntentStatus::Resolved as u8;
    intent.winning_quote = quote.key();

    emit!(AuctionResolved {
        intent: intent.key(),
        winning_quote: quote.key(),
        clearing_price: quote.revealed_price,
        filled_size: quote.revealed_size,
    });

    Ok(())
}
