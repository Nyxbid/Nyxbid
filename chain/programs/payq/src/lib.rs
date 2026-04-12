use anchor_lang::prelude::*;

declare_id!("mfn7v3f9qfyGMXeFFGzzqRWCfykPt6DCc9rxyWqd8bu");

#[program]
pub mod payq {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        label: String,
        daily_limit: u64,
        per_tx_limit: u64,
    ) -> Result<()> {
        require!(label.len() <= 32, PayqError::LabelTooLong);
        let vault = &mut ctx.accounts.vault;
        vault.authority = ctx.accounts.authority.key();
        vault.label = label;
        vault.daily_limit = daily_limit;
        vault.per_tx_limit = per_tx_limit;
        vault.total_spent = 0;
        vault.spent_today = 0;
        vault.last_reset = Clock::get()?.unix_timestamp;
        vault.bump = ctx.bumps.vault;
        Ok(())
    }

    pub fn record_spend(
        ctx: Context<RecordSpend>,
        agent_id: String,
        tool_id: String,
        amount: u64,
        proposal_hash: [u8; 32],
    ) -> Result<()> {
        require!(agent_id.len() <= 32, PayqError::FieldTooLong);
        require!(tool_id.len() <= 64, PayqError::FieldTooLong);

        let vault = &mut ctx.accounts.vault;
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
        record.agent_id = agent_id;
        record.tool_id = tool_id;
        record.amount = amount;
        record.proposal_hash = proposal_hash;
        record.timestamp = now;
        record.bump = ctx.bumps.spend_record;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(label: String)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = Vault::SIZE,
        seeds = [b"vault", authority.key().as_ref(), label.as_bytes()],
        bump,
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(agent_id: String, tool_id: String, amount: u64, proposal_hash: [u8; 32])]
pub struct RecordSpend<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority,
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        init,
        payer = authority,
        space = SpendRecord::SIZE,
        seeds = [b"spend", vault.key().as_ref(), &proposal_hash],
        bump,
    )]
    pub spend_record: Account<'info, SpendRecord>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Vault {
    pub authority: Pubkey,
    pub label: String,
    pub daily_limit: u64,
    pub per_tx_limit: u64,
    pub total_spent: u64,
    pub spent_today: u64,
    pub last_reset: i64,
    pub bump: u8,
}

impl Vault {
    pub const SIZE: usize = 8 + 32 + (4 + 32) + 32 + 8 + 1;
}

#[account]
pub struct SpendRecord {
    pub vault: Pubkey,
    pub agent_id: String,
    pub tool_id: String,
    pub amount: u64,
    pub proposal_hash: [u8; 32],
    pub timestamp: i64,
    pub bump: u8,
}

impl SpendRecord {
    pub const SIZE: usize = 8 + 32 + (4 + 32) + (4 + 64) + 8 + 32 + 8 + 1;
}

#[error_code]
pub enum PayqError {
    #[msg("Label must be 32 characters or fewer")]
    LabelTooLong,
    #[msg("Field exceeds maximum length")]
    FieldTooLong,
    #[msg("Amount exceeds per-transaction limit")]
    ExceedsPerTxLimit,
    #[msg("Cumulative spend exceeds daily limit")]
    ExceedsDailyLimit,
    #[msg("Arithmetic overflow")]
    Overflow,
}
