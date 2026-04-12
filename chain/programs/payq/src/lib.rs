use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("mfn7v3f9qfyGMXeFFGzzqRWCfykPt6DCc9rxyWqd8bu");

#[program]
pub mod payq {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        label: String,
        daily_limit: u64,
        per_tx_limit: u64,
        delegate: Pubkey,
    ) -> Result<()> {
        instructions::initialize_vault::handler(ctx, label, daily_limit, per_tx_limit, delegate)
    }

    pub fn update_vault(
        ctx: Context<UpdateVault>,
        daily_limit: u64,
        per_tx_limit: u64,
        delegate: Pubkey,
        paused: bool,
    ) -> Result<()> {
        instructions::update_vault::handler(ctx, daily_limit, per_tx_limit, delegate, paused)
    }

    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        instructions::close_vault::handler(ctx)
    }

    pub fn record_spend(
        ctx: Context<RecordSpend>,
        agent_id: String,
        tool_id: String,
        amount: u64,
        proposal_hash: [u8; 32],
    ) -> Result<()> {
        instructions::record_spend::handler(ctx, agent_id, tool_id, amount, proposal_hash)
    }
}
