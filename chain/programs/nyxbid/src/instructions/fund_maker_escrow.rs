use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::error::NyxbidError;
use crate::state::{
    Escrow, Intent, IntentStatus, Quote, Side, ESCROW_SEED, MAKER_VAULT_SEED,
};

/// The maker locks the opposite leg of the trade into a PDA-owned vault.
/// Must be called after submit_quote and before resolve_auction.
///
/// The amount is the maker's own forecast of what they will reveal. The
/// program does not check it against the commitment here (the commitment
/// is still sealed). Resolve will reject the reveal if the funded amount
/// is wrong, costing the maker the auction.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct FundMakerEscrowParams {
    /// Amount of `maker_lock_mint` to lock.
    pub amount: u64,
}

#[derive(Accounts)]
pub struct FundMakerEscrow<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Box<Account<'info, Intent>>,

    #[account(
        mut,
        constraint = quote.intent == intent.key(),
        constraint = quote.maker == maker.key() @ NyxbidError::Unauthorized,
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

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub(crate) fn handler(
    ctx: Context<FundMakerEscrow>,
    params: FundMakerEscrowParams,
) -> Result<()> {
    require!(params.amount > 0, NyxbidError::ZeroAmount);

    let side = Side::from_u8(ctx.accounts.intent.side).ok_or(NyxbidError::InvalidSide)?;
    let expected_mint = match side {
        // Taker buys base => maker delivers base.
        Side::Buy => ctx.accounts.intent.base_mint,
        // Taker sells base => maker delivers quote.
        Side::Sell => ctx.accounts.intent.quote_mint,
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

    let quote = &mut ctx.accounts.quote;
    quote.maker_funded = true;

    Ok(())
}
