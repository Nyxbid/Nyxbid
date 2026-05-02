use anchor_lang::prelude::*;

use crate::events::IntentCreated;
use crate::state::{Intent, IntentStatus, INTENT_SEED};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateIntentParams {
    pub side: u8,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: i64,
    pub resolve_deadline: i64,
    pub commitment_root: [u8; 32],
    pub nonce: [u8; 16],
}

#[derive(Accounts)]
#[instruction(params: CreateIntentParams)]
pub struct CreateIntent<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// CHECK: base asset mint, validated by the client
    pub base_mint: UncheckedAccount<'info>,

    /// CHECK: quote asset mint, validated by the client
    pub quote_mint: UncheckedAccount<'info>,

    #[account(
        init,
        payer = taker,
        space = 8 + Intent::INIT_SPACE,
        seeds = [INTENT_SEED, taker.key().as_ref(), &params.nonce],
        bump
    )]
    pub intent: Account<'info, Intent>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<CreateIntent>, params: CreateIntentParams) -> Result<()> {
    let intent = &mut ctx.accounts.intent;
    intent.taker = ctx.accounts.taker.key();
    intent.side = params.side;
    intent.base_mint = ctx.accounts.base_mint.key();
    intent.quote_mint = ctx.accounts.quote_mint.key();
    intent.size = params.size;
    intent.limit_price = params.limit_price;
    intent.reveal_deadline = params.reveal_deadline;
    intent.resolve_deadline = params.resolve_deadline;
    intent.commitment_root = params.commitment_root;
    intent.status = IntentStatus::Open as u8;
    intent.winning_quote = Pubkey::default();
    intent.bump = ctx.bumps.intent;
    intent.escrow_bump = 0;

    emit!(IntentCreated {
        intent: intent.key(),
        taker: intent.taker,
        side: intent.side,
        size: intent.size,
        limit_price: intent.limit_price,
        reveal_deadline: intent.reveal_deadline,
    });

    Ok(())
}
