use anchor_lang::prelude::*;
use crate::state::{Vault, SpendRecord};
use crate::errors::PayqError;
use crate::events::SpendRecorded;

#[derive(Accounts)]
#[instruction(agent_id: String, tool_id: String, amount: u64, proposal_hash: [u8; 32])]
pub struct RecordSpend<'info> {
    #[account(mut)]
    pub delegate: Signer<'info>,
    #[account(mut, has_one = delegate)]
    pub vault: Account<'info, Vault>,
    #[account(
        init,
        payer = delegate,
        space = SpendRecord::SIZE,
        seeds = [b"spend", vault.key().as_ref(), &proposal_hash],
        bump,
    )]
    pub spend_record: Account<'info, SpendRecord>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RecordSpend>,
    agent_id: String,
    tool_id: String,
    amount: u64,
    proposal_hash: [u8; 32],
) -> Result<()> {
    require!(agent_id.len() <= 32, PayqError::FieldTooLong);
    require!(tool_id.len() <= 64, PayqError::FieldTooLong);

    let vault = &mut ctx.accounts.vault;
    require!(!vault.paused, PayqError::VaultPaused);

    let now = Clock::get()?.unix_timestamp;

    let day_boundary = vault.last_reset - (vault.last_reset % 86_400) + 86_400;
    if now >= day_boundary {
        vault.spent_today = 0;
        vault.last_reset = now;
    }

    require!(amount <= vault.per_tx_limit, PayqError::ExceedsPerTxLimit);
    require!(
        vault.spent_today.checked_add(amount).ok_or(PayqError::Overflow)? <= vault.daily_limit,
        PayqError::ExceedsDailyLimit
    );

    vault.spent_today += amount;
    vault.total_spent += amount;

    let record = &mut ctx.accounts.spend_record;
    record.vault = vault.key();
    record.agent_id = agent_id.clone();
    record.tool_id = tool_id.clone();
    record.amount = amount;
    record.proposal_hash = proposal_hash;
    record.timestamp = now;
    record.bump = ctx.bumps.spend_record;

    emit!(SpendRecorded {
        vault: vault.key(),
        agent_id,
        tool_id,
        amount,
        proposal_hash,
    });

    Ok(())
}
