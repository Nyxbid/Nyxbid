use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::Settled;
use crate::state::{
    quote_notional, Escrow, Intent, IntentStatus, Quote, Receipt, Reputation, Side, ESCROW_SEED,
    MAKER_VAULT_SEED, RECEIPT_SEED, REPUTATION_SEED, TAKER_VAULT_SEED,
};

/// Atomic settlement.
///
/// Two CPI transfers and (on buy intents) one optional refund:
///
///   1. taker_vault -> maker_destination, amount = taker_paid.
///   2. maker_vault -> taker_destination, amount = escrow.maker_amount.
///   3. (buy only) taker_vault -> taker_refund_destination,
///      amount = escrow.taker_amount - taker_paid.
///
/// Where `taker_paid` is recomputed from the *executed* price:
///   - Buy:  taker_paid = quote_notional(filled_size, filled_price).
///           This is the cost at the *revealed* price, not the limit.
///           Any overpayment locked at create_intent (filled_price was
///           below limit_price) is refunded to the taker.
///   - Sell: taker_paid = escrow.taker_amount (== intent.size of base).
///           Sell side already has no price-dependent overpayment because
///           the locked amount was never priced.
///
/// This makes price-improvement flow to the taker on both sides, matching
/// the protocol promise that "the best valid bid wins" - the taker
/// transacts at the executed price, not the worst-case limit they posted.
#[derive(Accounts)]
pub struct Settle<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        constraint = intent.status == IntentStatus::Resolved as u8 @ NyxbidError::IntentNotResolved,
    )]
    pub intent: Box<Account<'info, Intent>>,

    #[account(
        constraint = winning_quote.key() == intent.winning_quote @ NyxbidError::NotWinningMaker,
        constraint = winning_quote.revealed @ NyxbidError::NotRevealed,
        constraint = winning_quote.maker_funded @ NyxbidError::MakerNotFunded,
    )]
    pub winning_quote: Box<Account<'info, Quote>>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, intent.key().as_ref()],
        bump = intent.escrow_bump,
        constraint = !escrow.settled @ NyxbidError::AlreadySettled,
        close = taker_rent_beneficiary,
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
        seeds = [MAKER_VAULT_SEED, intent.key().as_ref()],
        bump
    )]
    pub maker_vault: Box<Account<'info, TokenAccount>>,

    /// Maker's destination for the leg the taker locked.
    #[account(
        mut,
        constraint = maker_destination.mint == escrow.taker_mint @ NyxbidError::WrongLockMint,
        constraint = maker_destination.owner == winning_quote.maker @ NyxbidError::Unauthorized,
    )]
    pub maker_destination: Box<Account<'info, TokenAccount>>,

    /// Taker's destination for the leg the maker locked.
    #[account(
        mut,
        constraint = taker_destination.mint == escrow.maker_mint @ NyxbidError::WrongLockMint,
        constraint = taker_destination.owner == intent.taker @ NyxbidError::Unauthorized,
    )]
    pub taker_destination: Box<Account<'info, TokenAccount>>,

    /// Optional. Required on buy intents when the revealed price beats
    /// the limit, so the taker's overpay can be refunded. Same mint as
    /// the locked leg (escrow.taker_mint), owned by intent.taker.
    /// On sell intents and on buy intents with no price improvement,
    /// pass `None`.
    #[account(
        mut,
        constraint = taker_refund_destination.mint == escrow.taker_mint @ NyxbidError::WrongLockMint,
        constraint = taker_refund_destination.owner == intent.taker @ NyxbidError::Unauthorized,
    )]
    pub taker_refund_destination: Option<Box<Account<'info, TokenAccount>>>,

    /// CHECK: must equal intent.taker; rent for taker_vault is returned here.
    #[account(
        mut,
        constraint = taker_rent_beneficiary.key() == intent.taker @ NyxbidError::Unauthorized,
    )]
    pub taker_rent_beneficiary: UncheckedAccount<'info>,

    /// CHECK: must equal escrow.maker; rent for maker_vault is returned here.
    #[account(
        mut,
        constraint = maker_rent_beneficiary.key() == escrow.maker @ NyxbidError::Unauthorized,
    )]
    pub maker_rent_beneficiary: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + Receipt::INIT_SPACE,
        seeds = [RECEIPT_SEED, intent.key().as_ref()],
        bump
    )]
    pub receipt: Box<Account<'info, Receipt>>,

    #[account(
        mut,
        seeds = [REPUTATION_SEED, winning_quote.maker.as_ref()],
        bump = reputation.bump,
        constraint = reputation.maker == winning_quote.maker @ NyxbidError::Unauthorized,
    )]
    pub reputation: Box<Account<'info, Reputation>>,

    /// Sanity-check the mints are still the same as recorded.
    #[account(constraint = base_mint.key() == intent.base_mint @ NyxbidError::WrongLockMint)]
    pub base_mint: Box<Account<'info, Mint>>,
    #[account(constraint = quote_mint.key() == intent.quote_mint @ NyxbidError::WrongLockMint)]
    pub quote_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Settle>) -> Result<()> {
    let clock = Clock::get()?;

    // Settle must happen inside the grace window. Past the deadline,
    // the taker can call expire_with_maker to recover their leg and
    // the winning maker takes a failed_reveals reputation hit.
    require!(
        clock.unix_timestamp < ctx.accounts.intent.settle_deadline,
        NyxbidError::SettleDeadlinePassed
    );

    let intent_key = ctx.accounts.intent.key();
    let escrow_bump = ctx.accounts.intent.escrow_bump;
    let side = Side::from_u8(ctx.accounts.intent.side).ok_or(NyxbidError::InvalidSide)?;
    let filled_price = ctx.accounts.winning_quote.revealed_price;
    let filled_size = ctx.accounts.winning_quote.revealed_size;
    let locked_taker_amount = ctx.accounts.escrow.taker_amount;

    // Recompute the amount the taker actually owes at the executed price.
    // For buys this is below the locked worst-case when filled_price <
    // limit_price; the difference is refunded.
    let taker_paid = match side {
        Side::Buy => quote_notional(filled_size, filled_price)
            .ok_or(NyxbidError::MathOverflow)?,
        Side::Sell => locked_taker_amount,
    };
    require!(
        taker_paid <= locked_taker_amount,
        NyxbidError::MathOverflow
    );
    let refund = locked_taker_amount.saturating_sub(taker_paid);

    let signer_seeds: &[&[u8]] = &[ESCROW_SEED, intent_key.as_ref(), &[escrow_bump]];
    let signer = &[signer_seeds];

    // Leg 1: taker_vault -> maker_destination, the executed price.
    let cpi1 = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.taker_vault.to_account_info(),
            to: ctx.accounts.maker_destination.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi1, taker_paid)?;

    // Leg 2: maker_vault -> taker_destination.
    let cpi2 = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.maker_vault.to_account_info(),
            to: ctx.accounts.taker_destination.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi2, ctx.accounts.escrow.maker_amount)?;

    // Buy-side price-improvement refund.
    if refund > 0 {
        let refund_to = ctx
            .accounts
            .taker_refund_destination
            .as_ref()
            .ok_or(NyxbidError::MissingRefundDestination)?;
        let cpi_refund = CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.taker_vault.to_account_info(),
                to: refund_to.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_refund, refund)?;
    }

    // Close both vaults, returning rent to the original payers.
    let close_taker = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.taker_vault.to_account_info(),
            destination: ctx.accounts.taker_rent_beneficiary.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::close_account(close_taker)?;

    let close_maker = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.maker_vault.to_account_info(),
            destination: ctx.accounts.maker_rent_beneficiary.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::close_account(close_maker)?;

    let escrow = &mut ctx.accounts.escrow;
    escrow.settled = true;

    let intent = &mut ctx.accounts.intent;
    let quote = &ctx.accounts.winning_quote;
    let receipt = &mut ctx.accounts.receipt;
    receipt.intent = intent.key();
    receipt.taker = intent.taker;
    receipt.maker = quote.maker;
    receipt.base_mint = intent.base_mint;
    receipt.quote_mint = intent.quote_mint;
    receipt.filled_size = filled_size;
    receipt.filled_price = filled_price;
    receipt.settled_at = clock.unix_timestamp;
    receipt.bump = ctx.bumps.receipt;

    intent.status = IntentStatus::Settled as u8;

    let rep = &mut ctx.accounts.reputation;
    rep.settled_count = rep.settled_count.saturating_add(1);

    emit!(Settled {
        intent: intent.key(),
        receipt: receipt.key(),
        maker: receipt.maker,
        taker: receipt.taker,
        filled_price,
        filled_size,
    });

    Ok(())
}
