use anchor_lang::prelude::*;

use crate::error::NyxbidError;
use crate::events::Cancelled;
use crate::state::{Intent, IntentStatus};

#[derive(Accounts)]
pub struct Cancel<'info> {
    pub signer: Signer<'info>,

    #[account(
        mut,
        constraint = signer.key() == intent.taker @ NyxbidError::Unauthorized,
        constraint = intent.status == IntentStatus::Open as u8 @ NyxbidError::IntentNotOpen,
    )]
    pub intent: Account<'info, Intent>,
}

pub(crate) fn handler(ctx: Context<Cancel>) -> Result<()> {
    let intent = &mut ctx.accounts.intent;
    intent.status = IntentStatus::Cancelled as u8;

    emit!(Cancelled {
        intent: intent.key(),
        reason: 0,
    });

    Ok(())
}
