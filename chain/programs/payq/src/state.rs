use anchor_lang::prelude::*;

#[account]
pub struct Vault {
    pub authority: Pubkey,
    pub delegate: Pubkey,
    pub label: String,
    pub daily_limit: u64,
    pub per_tx_limit: u64,
    pub total_spent: u64,
    pub spent_today: u64,
    pub last_reset: i64,
    pub paused: bool,
    pub bump: u8,
}

impl Vault {
    /// 8 disc + 32 authority + 32 delegate + (4+32) label + 5×8 nums + 1 paused + 1 bump
    pub const SIZE: usize = 8 + 32 + 32 + (4 + 32) + 40 + 1 + 1;
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
