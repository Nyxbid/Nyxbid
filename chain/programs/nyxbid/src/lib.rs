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

    pub fn reveal_quote(
        ctx: Context<RevealQuote>,
        params: RevealQuoteParams,
    ) -> Result<()> {
        instructions::reveal_quote::handler(ctx, params)
    }

    pub fn fund_maker_escrow(
        ctx: Context<FundMakerEscrow>,
        params: FundMakerEscrowParams,
    ) -> Result<()> {
        instructions::fund_maker_escrow::handler(ctx, params)
    }

    pub fn settle(ctx: Context<Settle>) -> Result<()> {
        instructions::settle::handler(ctx)
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        instructions::cancel::handler(ctx)
    }

    pub fn expire_with_maker(ctx: Context<ExpireWithMaker>) -> Result<()> {
        instructions::expire_with_maker::handler(ctx)
    }

    pub fn expire_no_maker(ctx: Context<ExpireNoMaker>) -> Result<()> {
        instructions::expire_no_maker::handler(ctx)
    }
}
