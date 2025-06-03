use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::SolCall;
use anchor_lang::prelude::*;

use crate::{
    constants::{GAS_FEE_RECEIVER, MESSENGER_SEED, REMOTE_MESSENGER_ADDRESS},
    instructions::{send_call, Call},
    solidity::CrossChainMessenger::{self},
    state::Messenger,
};

#[derive(Accounts)]
pub struct SendMessage<'info> {
    // Messenger accounts
    #[account(mut, seeds = [MESSENGER_SEED], bump)]
    pub messenger: Account<'info, Messenger>,

    // Portal accounts
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut, address = GAS_FEE_RECEIVER)]
    /// CHECK: This is the hardcoded gas fee receiver account.
    pub gas_fee_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn send_message_handler(
    ctx: Context<SendMessage>,
    target: [u8; 20],
    message: Vec<u8>,
    min_gas_limit: u64,
) -> Result<()> {
    let relay_message_call = CrossChainMessenger::relayMessageCall {
        nonce: U256::from(ctx.accounts.messenger.nonce),
        sender: FixedBytes::from(ctx.accounts.authority.key().to_bytes()),
        target: target.into(),
        minGasLimit: U256::from(min_gas_limit),
        message: message.clone().into(),
    }
    .abi_encode();

    send_call(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.authority,
        &ctx.accounts.gas_fee_receiver,
        Call {
            to: REMOTE_MESSENGER_ADDRESS,
            gas_limit: min_gas_limit,
            is_creation: false,
            data: relay_message_call,
        },
    )?;

    ctx.accounts.messenger.nonce += 1;

    Ok(())
}
