use anchor_lang::prelude::*;

#[event]
pub struct VaultCreated {
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub delegate: Pubkey,
    pub label: String,
}

#[event]
pub struct VaultUpdated {
    pub vault: Pubkey,
    pub daily_limit: u64,
    pub per_tx_limit: u64,
    pub delegate: Pubkey,
    pub paused: bool,
}

#[event]
pub struct VaultClosed {
    pub vault: Pubkey,
    pub authority: Pubkey,
}

#[event]
pub struct SpendRecorded {
    pub vault: Pubkey,
    pub agent_id: String,
    pub tool_id: String,
    pub amount: u64,
    pub proposal_hash: [u8; 32],
}
