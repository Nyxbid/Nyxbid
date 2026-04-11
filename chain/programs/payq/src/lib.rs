use anchor_lang::prelude::*;

declare_id!("6PG9BxoEc5APJkWbjYMuQd8epu2pQcq96DTynYFUDtgu");

#[program]
pub mod payq {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Payq program: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
