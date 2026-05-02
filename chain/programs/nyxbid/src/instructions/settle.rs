use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::events::Settled;
use crate::state::{
    Escrow, Intent, IntentStatus, Quote, Receipt, Reputation, ESCROW_SEED, MAKER_VAULT_SEED,
    RECEIPT_SEED, REPUTATION_SEED, TAKER_VAULT_SEED,
};

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
        constraint = winning_quote.key() == intent.winning_quote,
        constraint = winning_quote.revealed @ NyxbidError::AlreadyRevealed,
    )]
    pub winning_quote: Box<Account<'info, Quote>>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, intent.key().as_ref()],
        bump = intent.escrow_bump,
        constraint = !escrow.settled @ NyxbidError::AlreadySettled,
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
    let intent_key = ctx.accounts.intent.key();
    let escrow_bump = ctx.accounts.intent.escrow_bump;

    let signer_seeds: &[&[u8]] = &[ESCROW_SEED, intent_key.as_ref(), &[escrow_bump]];
    let signer = &[signer_seeds];

    // Leg 1: taker_vault -> maker_destination.
    let cpi1 = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.taker_vault.to_account_info(),
            to: ctx.accounts.maker_destination.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi1, ctx.accounts.escrow.taker_amount)?;

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
    receipt.filled_size = quote.revealed_size;
    receipt.filled_price = quote.revealed_price;
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
        filled_price: receipt.filled_price,
        filled_size: receipt.filled_size,
    });

    Ok(())
}
