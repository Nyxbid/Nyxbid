use anchor_lang::prelude::*;

use crate::error::NyxbidError;
use crate::events::QuoteSubmitted;
use crate::state::{Intent, IntentStatus, Quote, QUOTE_SEED};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SubmitQuoteParams {
    pub commitment: [u8; 32],
    pub nonce: [u8; 16],
}

#[derive(Accounts)]
#[instruction(params: SubmitQuoteParams)]
pub struct SubmitQuote<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Account<'info, Intent>,

    #[account(
        init,
        payer = maker,
        space = 8 + Quote::INIT_SPACE,
        seeds = [QUOTE_SEED, intent.key().as_ref(), maker.key().as_ref(), &params.nonce],
        bump
    )]
    pub quote: Account<'info, Quote>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<SubmitQuote>, params: SubmitQuoteParams) -> Result<()> {
    let quote = &mut ctx.accounts.quote;
    quote.intent = ctx.accounts.intent.key();
    quote.maker = ctx.accounts.maker.key();
    quote.commitment = params.commitment;
    quote.revealed_price = 0;
    quote.revealed_size = 0;
    quote.nonce = [0u8; 32];
    quote.revealed = false;
    quote.maker_funded = false;
    quote.bump = ctx.bumps.quote;

    emit!(QuoteSubmitted {
        intent: quote.intent,
        quote: quote.key(),
        maker: quote.maker,
    });

    Ok(())
}
