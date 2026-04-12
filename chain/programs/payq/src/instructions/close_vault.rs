use anchor_lang::prelude::*;
use crate::state::Vault;
use crate::events::VaultClosed;

#[derive(Accounts)]
pub struct CloseVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, close = authority, has_one = authority)]
    pub vault: Account<'info, Vault>,
}

pub fn handler(ctx: Context<CloseVault>) -> Result<()> {
    emit!(VaultClosed {
        vault: ctx.accounts.vault.key(),
        authority: ctx.accounts.authority.key(),
    });
    Ok(())
}
