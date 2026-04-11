use anchor_lang::prelude::*;

declare_id!("9zxiEPoEV4M4CTtjeixXhFq9Y8n8cCTCiDdkomyb5YEq");

#[program]
pub mod neurocracy {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
