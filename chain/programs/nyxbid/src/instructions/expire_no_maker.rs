use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::Cancelled;
use crate::state::{
    Escrow, Intent, IntentStatus, Quote, Reputation, ESCROW_SEED, REPUTATION_SEED,
    TAKER_VAULT_SEED,
};

/// Recovery path when no maker funded the maker_vault by `settle_deadline`.
/// Permissionless: anyone can trigger after the deadline. Refunds the
/// taker's leg only and closes taker_vault.
///
/// Three sub-cases this instruction handles:
///
/// 1. **Empty market** \u2014 no maker submitted any quote.
///    `intent.winning_quote == default`. No penalty: no one to penalize.
///
/// 2. **All quotes breached the limit** \u2014 makers submitted but every
///    reveal failed `LimitBreached`, so `winning_quote` stayed default.
///    No penalty.
///
/// 3. **Reveal-but-don't-fund grief** \u2014 a maker successfully revealed
///    the best price (taking the `winning_quote` slot, so subsequent
///    reveals at worse prices could not displace them) but never called
///    `fund_maker_escrow`. `winning_quote != default` and the named
///    maker is the abandoner.
///
///    This case **must** apply the `failed_reveals` penalty, otherwise
///    a maker could grief the taker for free: lock the winning slot
///    with no capital, never fund, and walk away costless. Closes the
///    P1 finding from review.
///
/// To handle all three with a single instruction, `winning_quote` and
/// `reputation` are Optional accounts:
/// - When `intent.winning_quote == default`, callers pass `None` for
///   both. No reputation mutation.
/// - When `intent.winning_quote != default`, callers MUST pass both,
///   pointing at the on-chain quote and the maker's Reputation PDA.
///   The handler verifies, then bumps `failed_reveals` by 1.
///
/// Constraints:
///   - clock >= settle_deadline,
///   - intent.status is Open (no winner was ever finalized via
///     fund_maker_escrow \u2014 if it had been, status would be Resolved
///     and expire_with_maker would apply),
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
        close = taker_rent_beneficiary,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        seeds = [TAKER_VAULT_SEED, intent.key().as_ref()],
        bump = escrow.taker_vault_bump,
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

    /// Optional. MUST be present when `intent.winning_quote != default`.
    /// MUST equal `intent.winning_quote` if present.
    pub winning_quote: Option<Box<Account<'info, Quote>>>,

    /// Optional. MUST be present when `intent.winning_quote != default`.
    /// MUST be the Reputation PDA for `winning_quote.maker`.
    #[account(mut)]
    pub winning_maker_reputation: Option<Box<Account<'info, Reputation>>>,

    pub token_program: Program<'info, Token>,
}

pub(crate) fn handler(ctx: Context<ExpireNoMaker>) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= ctx.accounts.intent.settle_deadline,
        NyxbidError::SettleDeadlineNotReached
    );

    // Branch on whether a winning quote was selected during the reveal
    // window but never funded.
    let winning_quote_key = ctx.accounts.intent.winning_quote;
    let has_winner = winning_quote_key != Pubkey::default();

    if has_winner {
        // Reveal-but-don't-fund grief: the optional accounts MUST be present
        // and the caller MUST pass the canonical winning quote + that
        // maker's reputation PDA.
        let quote = ctx
            .accounts
            .winning_quote
            .as_ref()
            .ok_or(NyxbidError::MissingWinnerAccounts)?;
        let rep = ctx
            .accounts
            .winning_maker_reputation
            .as_mut()
            .ok_or(NyxbidError::MissingWinnerAccounts)?;

        require_keys_eq!(
            quote.key(),
            winning_quote_key,
            NyxbidError::NotWinningMaker
        );

        // Verify the reputation PDA actually belongs to this maker.
        let (expected_rep, _bump) = Pubkey::find_program_address(
            &[REPUTATION_SEED, quote.maker.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(rep.key(), expected_rep, NyxbidError::Unauthorized);
        require_keys_eq!(rep.maker, quote.maker, NyxbidError::Unauthorized);

        rep.failed_reveals = rep.failed_reveals.saturating_add(1);
    } else {
        // Empty market or all-quotes-breached. No winner to penalize.
        // The optional accounts must NOT be passed (defense against a
        // caller trying to spuriously bump some unrelated reputation).
        require!(
            ctx.accounts.winning_quote.is_none()
                && ctx.accounts.winning_maker_reputation.is_none(),
            NyxbidError::UnexpectedWinnerAccounts
        );
    }

    // Refund taker's leg.
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
