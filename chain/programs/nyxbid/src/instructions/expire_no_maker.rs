use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::Cancelled;
use crate::state::{
    Escrow, Intent, IntentStatus, ESCROW_SEED, TAKER_VAULT_SEED,
};

/// Recovery path when no maker ever funded the maker_vault by the
/// settle_deadline. Permissionless: anyone can trigger after the
/// deadline. Refunds the taker's leg only and closes taker_vault.
///
/// This closes the P0 hole where, if no maker ever revealed and funded
/// in time, the taker's locked funds had no recovery path:
///   - cancel was blocked by clock >= reveal_deadline,
///   - expire_with_maker required escrow.maker_amount > 0.
///
/// Use cases:
///   - Empty market: no maker bid at all.
///   - All revealed quotes breached the limit (so winning_quote stayed
///     default).
///   - The selected winner never funded by settle_deadline.
///
/// Constraints:
///   - clock >= settle_deadline,
///   - intent.status is Open (no winner was ever finalized via
///     fund_maker_escrow),
///   - escrow not settled,
///   - escrow.maker_amount == 0 (no maker funded - the funded case uses
///     expire_with_maker instead).
#[derive(Accounts)]
pub struct ExpireNoMaker<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
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

    /// CHECK: receives the rent from taker_vault. Must be intent.taker.
    #[account(
        mut,
        constraint = taker_rent_beneficiary.key() == intent.taker @ NyxbidError::Unauthorized,
    )]
    pub taker_rent_beneficiary: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub(crate) fn handler(ctx: Context<ExpireNoMaker>) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= ctx.accounts.intent.settle_deadline,
        NyxbidError::SettleDeadlineNotReached
    );

    let intent_key = ctx.accounts.intent.key();
    let escrow_bump = ctx.accounts.intent.escrow_bump;
    let signer_seeds: &[&[u8]] = &[ESCROW_SEED, intent_key.as_ref(), &[escrow_bump]];
    let signer = &[signer_seeds];

    let cpi_t = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.taker_vault.to_account_info(),
            to: ctx.accounts.taker_destination.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi_t, ctx.accounts.escrow.taker_amount)?;

    let close_t = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.taker_vault.to_account_info(),
            destination: ctx.accounts.taker_rent_beneficiary.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::close_account(close_t)?;

    let escrow = &mut ctx.accounts.escrow;
    escrow.settled = true;
    escrow.taker_amount = 0;

    let intent = &mut ctx.accounts.intent;
    intent.status = IntentStatus::Expired as u8;

    emit!(Cancelled {
        intent: intent.key(),
        reason: 2, // 0 = cancel, 1 = expire_with_maker, 2 = expire_no_maker
    });

    Ok(())
}
