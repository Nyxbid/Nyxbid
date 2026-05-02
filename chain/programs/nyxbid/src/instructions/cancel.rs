use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::Cancelled;
use crate::state::{
    Escrow, Intent, IntentStatus, ESCROW_SEED, TAKER_VAULT_SEED,
};

/// Taker-initiated cancel. Refunds the locked taker leg and closes the
/// taker vault. Only allowed if:
///   - intent is still Open (no winning quote locked in),
///   - no maker has funded the opposite leg (would be unfair to that maker),
///   - resolve_deadline has not passed (after that the expire path is used,
///     which has slightly different semantics for already-funded auctions).
#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(
        mut,
        constraint = taker.key() == intent.taker @ NyxbidError::Unauthorized,
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Box<Account<'info, Intent>>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, intent.key().as_ref()],
        bump = intent.escrow_bump,
        constraint = !escrow.settled @ NyxbidError::AlreadySettled,
        constraint = escrow.maker_amount == 0 @ NyxbidError::MakerAlreadyFunded,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        seeds = [TAKER_VAULT_SEED, intent.key().as_ref()],
        bump
    )]
    pub taker_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = taker_destination.mint == escrow.taker_mint @ NyxbidError::WrongLockMint,
        constraint = taker_destination.owner == intent.taker @ NyxbidError::Unauthorized,
    )]
    pub taker_destination: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub(crate) fn handler(ctx: Context<Cancel>) -> Result<()> {
    let clock = Clock::get()?;
    let intent_key = ctx.accounts.intent.key();
    let escrow_bump = ctx.accounts.intent.escrow_bump;

    require!(
        clock.unix_timestamp < ctx.accounts.intent.resolve_deadline,
        NyxbidError::ResolveDeadlinePassed
    );

    let signer_seeds: &[&[u8]] = &[ESCROW_SEED, intent_key.as_ref(), &[escrow_bump]];
    let signer = &[signer_seeds];

    // Refund taker_vault -> taker_destination.
    let cpi_transfer = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.taker_vault.to_account_info(),
            to: ctx.accounts.taker_destination.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi_transfer, ctx.accounts.escrow.taker_amount)?;

    // Close vault, rent back to taker.
    let cpi_close = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.taker_vault.to_account_info(),
            destination: ctx.accounts.taker.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::close_account(cpi_close)?;

    let escrow = &mut ctx.accounts.escrow;
    escrow.settled = true;
    escrow.taker_amount = 0;

    let intent = &mut ctx.accounts.intent;
    intent.status = IntentStatus::Cancelled as u8;

    emit!(Cancelled {
        intent: intent.key(),
        reason: 0,
    });

    Ok(())
}
