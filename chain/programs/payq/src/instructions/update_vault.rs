use anchor_lang::prelude::*;
use crate::state::Vault;
use crate::events::VaultUpdated;

#[derive(Accounts)]
pub struct UpdateVault<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub vault: Account<'info, Vault>,
}

pub fn handler(
    ctx: Context<UpdateVault>,
    daily_limit: u64,
    per_tx_limit: u64,
    delegate: Pubkey,
    paused: bool,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.daily_limit = daily_limit;
    vault.per_tx_limit = per_tx_limit;
    vault.delegate = delegate;
    vault.paused = paused;

    emit!(VaultUpdated {
        vault: vault.key(),
        daily_limit,
        per_tx_limit,
        delegate,
        paused,
    });

    Ok(())
}
