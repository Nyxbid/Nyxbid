use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::Cancelled;
use crate::state::{
    Escrow, Intent, IntentStatus, Reputation, ESCROW_SEED, MAKER_VAULT_SEED, REPUTATION_SEED,
    TAKER_VAULT_SEED,
};

/// Permissionless expiry. Anyone can call this after resolve_deadline
/// if the auction is still Open. Both legs (if funded) are returned to
/// their original owners.
///
/// Two shapes:
///  - Maker never funded: only the taker_vault is refunded/closed.
///    `maker_vault` and `maker_destination` and `maker_rent_beneficiary`
///    are passed but ignored.
///  - Maker funded: both vaults refunded/closed.
///
/// We keep the account list fixed for predictable IDL/SDK ergonomics.
/// The maker leg accounts may be dummy-equal-to-taker accounts if there
/// is no maker_vault \u2014 but we can't actually fake a missing PDA, so the
/// flow currently requires the maker_vault PDA exists. To keep this
/// simple in Phase 1, we split into two flavours: this instruction
/// expects maker_vault is present (escrow.maker != default). For the
/// no-maker case use cancel before resolve_deadline, or expire_no_maker.
#[derive(Accounts)]
pub struct Expire<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Account<'info, Intent>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, intent.key().as_ref()],
        bump = intent.escrow_bump,
        constraint = !escrow.settled @ NyxbidError::AlreadySettled,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [TAKER_VAULT_SEED, intent.key().as_ref()],
        bump
    )]
    pub taker_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = taker_destination.mint == escrow.taker_mint @ NyxbidError::WrongLockMint,
        constraint = taker_destination.owner == intent.taker @ NyxbidError::Unauthorized,
    )]
    pub taker_destination: Account<'info, TokenAccount>,

    /// CHECK: receives the rent from taker_vault. Must be intent.taker.
    #[account(
        mut,
        constraint = taker_rent_beneficiary.key() == intent.taker @ NyxbidError::Unauthorized,
    )]
    pub taker_rent_beneficiary: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [MAKER_VAULT_SEED, intent.key().as_ref()],
        bump
    )]
    pub maker_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = maker_destination.mint == escrow.maker_mint @ NyxbidError::WrongLockMint,
        constraint = maker_destination.owner == escrow.maker @ NyxbidError::Unauthorized,
    )]
    pub maker_destination: Account<'info, TokenAccount>,

    /// CHECK: receives the rent from maker_vault. Must be escrow.maker.
    #[account(
        mut,
        constraint = maker_rent_beneficiary.key() == escrow.maker @ NyxbidError::Unauthorized,
    )]
    pub maker_rent_beneficiary: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [REPUTATION_SEED, escrow.maker.as_ref()],
        bump = reputation.bump,
        constraint = reputation.maker == escrow.maker @ NyxbidError::Unauthorized,
    )]
    pub reputation: Account<'info, Reputation>,

    pub token_program: Program<'info, Token>,
}

pub(crate) fn handler(ctx: Context<Expire>) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= ctx.accounts.intent.resolve_deadline,
        NyxbidError::ResolveDeadlineNotReached
    );
    // Maker funding is required to use this instruction; if none, taker
    // should have used cancel before the deadline. Surface a clear error.
    require!(
        ctx.accounts.escrow.maker_amount > 0,
        NyxbidError::MakerNotFunded
    );

    let intent_key = ctx.accounts.intent.key();
    let escrow_bump = ctx.accounts.intent.escrow_bump;
    let signer_seeds: &[&[u8]] = &[ESCROW_SEED, intent_key.as_ref(), &[escrow_bump]];
    let signer = &[signer_seeds];

    // Refund taker leg.
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

    // Refund maker leg.
    let cpi_m = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.maker_vault.to_account_info(),
            to: ctx.accounts.maker_destination.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi_m, ctx.accounts.escrow.maker_amount)?;

    let close_m = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.maker_vault.to_account_info(),
            destination: ctx.accounts.maker_rent_beneficiary.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::close_account(close_m)?;

    let escrow = &mut ctx.accounts.escrow;
    escrow.settled = true;

    let intent = &mut ctx.accounts.intent;
    intent.status = IntentStatus::Expired as u8;

    // The maker funded but never revealed in time. Count as a failed reveal.
    let rep = &mut ctx.accounts.reputation;
    rep.failed_reveals = rep.failed_reveals.saturating_add(1);

    emit!(Cancelled {
        intent: intent.key(),
        reason: 1, // 0 = cancel, 1 = expire
    });

    Ok(())
}
