use anchor_lang::prelude::*;

use crate::error::NyxbidError;
use crate::events::Settled;
use crate::state::{Intent, IntentStatus, Quote, Receipt, RECEIPT_SEED};

#[derive(Accounts)]
pub struct Settle<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        constraint = intent.status == IntentStatus::Resolved as u8 @ NyxbidError::IntentNotResolved,
    )]
    pub intent: Account<'info, Intent>,

    #[account(
        constraint = winning_quote.key() == intent.winning_quote,
        constraint = winning_quote.revealed @ NyxbidError::AlreadyRevealed,
    )]
    pub winning_quote: Account<'info, Quote>,

    #[account(
        init,
        payer = payer,
        space = Receipt::LEN,
        seeds = [RECEIPT_SEED, intent.key().as_ref()],
        bump
    )]
    pub receipt: Account<'info, Receipt>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Settle>) -> Result<()> {
    let clock = Clock::get()?;
    let intent = &mut ctx.accounts.intent;
    let quote = &ctx.accounts.winning_quote;
    let receipt = &mut ctx.accounts.receipt;

    receipt.intent = intent.key();
    receipt.taker = intent.taker;
    receipt.maker = quote.maker;
    receipt.base_mint = intent.base_mint;
    receipt.quote_mint = intent.quote_mint;
    receipt.filled_size = quote.revealed_size;
    receipt.filled_price = quote.revealed_price;
    receipt.settled_at = clock.unix_timestamp;
    receipt.bump = ctx.bumps.receipt;

    intent.status = IntentStatus::Settled as u8;

    emit!(Settled {
        intent: intent.key(),
        receipt: receipt.key(),
        maker: receipt.maker,
        taker: receipt.taker,
        filled_price: receipt.filled_price,
        filled_size: receipt.filled_size,
    });

    Ok(())
}
