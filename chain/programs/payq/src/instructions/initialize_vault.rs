use anchor_lang::prelude::*;
use crate::state::Vault;
use crate::errors::PayqError;
use crate::events::VaultCreated;

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

pub fn handler(
    ctx: Context<InitializeVault>,
    label: String,
    daily_limit: u64,
    per_tx_limit: u64,
    delegate: Pubkey,
) -> Result<()> {
    require!(label.len() <= 32, PayqError::LabelTooLong);

    let vault = &mut ctx.accounts.vault;
    vault.authority = ctx.accounts.authority.key();
    vault.delegate = delegate;
    vault.label = label.clone();
    vault.daily_limit = daily_limit;
    vault.per_tx_limit = per_tx_limit;
    vault.total_spent = 0;
    vault.spent_today = 0;
    vault.last_reset = Clock::get()?.unix_timestamp;
    vault.paused = false;
    vault.bump = ctx.bumps.vault;

    emit!(VaultCreated {
        vault: vault.key(),
        authority: vault.authority,
        delegate,
        label,
    });

    Ok(())
}
