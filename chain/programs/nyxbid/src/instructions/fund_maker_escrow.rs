use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::AuctionResolved;
use crate::state::{
    quote_notional, Escrow, Intent, IntentStatus, Quote, Reputation, Side, ESCROW_SEED,
    MAKER_VAULT_SEED, REPUTATION_SEED,
};

/// The winning maker locks the opposite leg of the trade into a
/// PDA-owned vault, after the auction's reveal window has selected them.
///
/// Lifecycle (Phase 1):
///   1. create_intent: taker locks their leg in taker_vault.
///   2. submit_quote: makers post sealed commitments (no funding).
///   3. reveal_quote (during the reveal window): each maker reveals.
///      The program keeps the best valid revealed quote in
///      intent.winning_quote (lowest price for buy / highest for sell).
///   4. After resolve_deadline the winner is final.
///   5. fund_maker_escrow (this instruction): only the maker pointed to
///      by intent.winning_quote can call this, and only between
///      resolve_deadline and settle_deadline. The amount and mint must
///      match what the revealed price implies.
///   6. settle: dual CPI atomic swap.
///
/// Single maker_vault per intent is intentional: the auction selects one
/// winner, and only that winner ever needs to lock capital. Losing
/// makers never tied up funds.
///
/// Side effects on the first successful call:
///   - intent.status -> Resolved.
///   - reputation.quotes_won += 1.
///   - AuctionResolved event emitted (preserves event compatibility with
///     downstream indexers that watched the old resolve_auction ix).
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct FundMakerEscrowParams {
    /// Amount of `maker_lock_mint` to lock. Must equal the notional
    /// implied by intent.winning_price and intent.size for the side.
    pub amount: u64,
}

#[derive(Accounts)]
pub struct FundMakerEscrow<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Box<Account<'info, Intent>>,

    /// The currently winning quote. Must equal intent.winning_quote and
    /// must belong to `maker`.
    #[account(
        mut,
        constraint = quote.key() == intent.winning_quote @ NyxbidError::NotWinningMaker,
        constraint = quote.maker == maker.key() @ NyxbidError::NotWinningMaker,
        constraint = quote.revealed @ NyxbidError::NotRevealed,
        constraint = !quote.maker_funded @ NyxbidError::MakerAlreadyFunded,
    )]
    pub quote: Box<Account<'info, Quote>>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, intent.key().as_ref()],
        bump = intent.escrow_bump,
        constraint = !escrow.settled @ NyxbidError::AlreadySettled,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    /// The mint the maker delivers. Buy intent => base_mint; sell => quote_mint.
    pub maker_lock_mint: Box<Account<'info, Mint>>,

    #[account(mut, token::authority = maker)]
    pub maker_source: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = maker,
        token::mint = maker_lock_mint,
        token::authority = escrow,
        seeds = [MAKER_VAULT_SEED, intent.key().as_ref()],
        bump
    )]
    pub maker_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [REPUTATION_SEED, maker.key().as_ref()],
        bump = reputation.bump,
        constraint = reputation.maker == maker.key() @ NyxbidError::Unauthorized,
    )]
    pub reputation: Box<Account<'info, Reputation>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub(crate) fn handler(
    ctx: Context<FundMakerEscrow>,
    params: FundMakerEscrowParams,
) -> Result<()> {
    require!(params.amount > 0, NyxbidError::ZeroAmount);

    // Lifecycle window: post-reveal-deadline (winner is final), before
    // settle-deadline (the grace period for the winner to fund + settle).
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= ctx.accounts.intent.resolve_deadline,
        NyxbidError::ResolveDeadlineNotReached
    );
    require!(
        clock.unix_timestamp < ctx.accounts.intent.settle_deadline,
        NyxbidError::SettleDeadlinePassed
    );

    let side = Side::from_u8(ctx.accounts.intent.side).ok_or(NyxbidError::InvalidSide)?;

    // What mint and amount must the maker actually lock, given the
    // revealed price?
    let revealed_price = ctx.accounts.quote.revealed_price;
    let revealed_size = ctx.accounts.quote.revealed_size;
    let (expected_mint, expected_amount) = match side {
        // Buy: maker delivers base_mint, sized by revealed_size.
        Side::Buy => (ctx.accounts.intent.base_mint, revealed_size),
        // Sell: maker delivers quote_mint, sized by quote_notional().
        Side::Sell => (
            ctx.accounts.intent.quote_mint,
            quote_notional(revealed_size, revealed_price)
                .ok_or(NyxbidError::MathOverflow)?,
        ),
    };
    require_keys_eq!(
        ctx.accounts.maker_lock_mint.key(),
        expected_mint,
        NyxbidError::WrongLockMint
    );
    require_keys_eq!(
        ctx.accounts.maker_source.mint,
        expected_mint,
        NyxbidError::WrongLockMint
    );
    require!(
        params.amount == expected_amount,
        NyxbidError::WrongFundAmount
    );
    require!(
        ctx.accounts.maker_source.amount >= params.amount,
        NyxbidError::InsufficientDeposit
    );

    let cpi = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.maker_source.to_account_info(),
            to: ctx.accounts.maker_vault.to_account_info(),
            authority: ctx.accounts.maker.to_account_info(),
        },
    );
    token::transfer(cpi, params.amount)?;

    let escrow = &mut ctx.accounts.escrow;
    escrow.maker = ctx.accounts.maker.key();
    escrow.maker_amount = params.amount;
    escrow.maker_mint = expected_mint;
    escrow.maker_vault_bump = ctx.bumps.maker_vault;

    let quote = &mut ctx.accounts.quote;
    quote.maker_funded = true;

    // Finalize the auction state. This is the moment the auction is
    // canonically Resolved: a winner exists, has revealed correctly, and
    // has locked the opposite leg.
    let intent = &mut ctx.accounts.intent;
    intent.status = IntentStatus::Resolved as u8;

    let rep = &mut ctx.accounts.reputation;
    rep.quotes_won = rep.quotes_won.saturating_add(1);

    emit!(AuctionResolved {
        intent: intent.key(),
        winning_quote: quote.key(),
        clearing_price: revealed_price,
        filled_size: revealed_size,
    });

    Ok(())
}
