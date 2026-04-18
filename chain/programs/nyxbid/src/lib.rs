use anchor_lang::prelude::*;

pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("nyxkGtm8x7GMdTWKyy5TKa72pgsebrECrchPDuRSrEQ");

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
}
