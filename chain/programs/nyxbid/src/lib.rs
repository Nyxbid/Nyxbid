use anchor_lang::prelude::*;

pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("E9sMPu6uUJTfe72ePWr8BNjEKejUnMqsdFV6rGtsHiX2");

#[program]
pub mod nyxbid {
    use super::*;

    pub fn create_intent(
        ctx: Context<CreateIntent>,
        params: CreateIntentParams,
    ) -> Result<()> {
        instructions::create_intent::handler(ctx, params)
    }

    pub fn submit_quote(
        ctx: Context<SubmitQuote>,
        params: SubmitQuoteParams,
    ) -> Result<()> {
        instructions::submit_quote::handler(ctx, params)
    }

    pub fn fund_maker_escrow(
        ctx: Context<FundMakerEscrow>,
        params: FundMakerEscrowParams,
    ) -> Result<()> {
        instructions::fund_maker_escrow::handler(ctx, params)
    }

    pub fn resolve_auction(
        ctx: Context<ResolveAuction>,
        params: ResolveAuctionParams,
    ) -> Result<()> {
        instructions::resolve_auction::handler(ctx, params)
    }

    pub fn settle(ctx: Context<Settle>) -> Result<()> {
        instructions::settle::handler(ctx)
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        instructions::cancel::handler(ctx)
    }

    pub fn expire(ctx: Context<Expire>) -> Result<()> {
        instructions::expire::handler(ctx)
    }
}
