use anchor_lang::prelude::*;

declare_id!("Gwi8c92gteE63Z9i78nXmStWWP9tf6wLN5jaXC9tdGjp");

#[program]
pub mod bridge {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
