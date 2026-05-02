use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::IntentCreated;
use crate::state::{
    quote_notional, Escrow, Intent, IntentStatus, Side, ESCROW_SEED, INTENT_SEED, TAKER_VAULT_SEED,
};

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

    pub base_mint: Box<Account<'info, Mint>>,
    pub quote_mint: Box<Account<'info, Mint>>,

    /// Source ATA the taker is locking from. Must hold the leg dictated by `side`.
    #[account(mut, token::authority = taker)]
    pub taker_source: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = taker,
        space = 8 + Intent::INIT_SPACE,
        seeds = [INTENT_SEED, taker.key().as_ref(), &params.nonce],
        bump
    )]
    pub intent: Box<Account<'info, Intent>>,

    #[account(
        init,
        payer = taker,
        space = 8 + Escrow::INIT_SPACE,
        seeds = [ESCROW_SEED, intent.key().as_ref()],
        bump
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    /// PDA-owned vault holding the taker's locked tokens until settle/refund.
    #[account(
        init,
        payer = taker,
        token::mint = taker_lock_mint,
        token::authority = escrow,
        seeds = [TAKER_VAULT_SEED, intent.key().as_ref()],
        bump
    )]
    pub taker_vault: Box<Account<'info, TokenAccount>>,

    /// The mint actually being locked. Must match base_mint for sells,
    /// quote_mint for buys. Verified in handler.
    pub taker_lock_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub(crate) fn handler(ctx: Context<CreateIntent>, params: CreateIntentParams) -> Result<()> {
    let side = Side::from_u8(params.side).ok_or(NyxbidError::InvalidSide)?;

    // Determine which mint and amount the taker must lock.
    let (expected_mint, lock_amount) = match side {
        Side::Buy => {
            // Buy: lock quote_mint up to size * limit_price.
            let amt = quote_notional(params.size, params.limit_price)
                .ok_or(NyxbidError::MathOverflow)?;
            (ctx.accounts.quote_mint.key(), amt)
        }
        Side::Sell => {
            // Sell: lock size of base_mint.
            (ctx.accounts.base_mint.key(), params.size)
        }
    };

    require_keys_eq!(
        ctx.accounts.taker_lock_mint.key(),
        expected_mint,
        NyxbidError::WrongLockMint
    );
    require_keys_eq!(
        ctx.accounts.taker_source.mint,
        expected_mint,
        NyxbidError::WrongLockMint
    );
    require!(
        ctx.accounts.taker_source.amount >= lock_amount,
        NyxbidError::InsufficientDeposit
    );
    require!(lock_amount > 0, NyxbidError::ZeroAmount);
    require!(
        params.resolve_deadline > params.reveal_deadline,
        NyxbidError::BadDeadlines
    );

    // CPI: taker_source -> taker_vault (PDA-owned).
    let cpi = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.taker_source.to_account_info(),
            to: ctx.accounts.taker_vault.to_account_info(),
            authority: ctx.accounts.taker.to_account_info(),
        },
    );
    token::transfer(cpi, lock_amount)?;

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
    intent.escrow_bump = ctx.bumps.escrow;

    let escrow = &mut ctx.accounts.escrow;
    escrow.intent = intent.key();
    escrow.taker_amount = lock_amount;
    escrow.taker_mint = expected_mint;
    escrow.maker = Pubkey::default();
    escrow.maker_amount = 0;
    escrow.maker_mint = Pubkey::default();
    escrow.settled = false;
    escrow.bump = ctx.bumps.escrow;

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
