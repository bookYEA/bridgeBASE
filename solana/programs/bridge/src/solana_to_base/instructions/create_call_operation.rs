use anchor_lang::prelude::*;

use crate::solana_to_base::{
    internal::process_call_operation, Call, CallType, Operation, OutgoingMessageHeader,
    MESSAGE_HEADER_SEED, OPERATION_SEED,
};

#[derive(Accounts)]
#[instruction(id: u64, data: Vec<u8>)]
pub struct CreateCallOperation<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub from: Signer<'info>,

    #[account(
        init_if_needed,
        seeds = [MESSAGE_HEADER_SEED, from.key().as_ref(), id.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + OutgoingMessageHeader::INIT_SPACE,
    )]
    pub outgoing_message_header: Account<'info, OutgoingMessageHeader>,

    #[account(
        init,
        seeds = [
            OPERATION_SEED,
            outgoing_message_header.key().as_ref(),
            outgoing_message_header.operation_count.to_le_bytes().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + Operation::call_space(data.len()),
    )]
    pub call_operation: Account<'info, Operation>,

    pub system_program: Program<'info, System>,
}

pub fn create_call_operation_handler(
    ctx: Context<CreateCallOperation>,
    _id: u64,
    ty: CallType,
    gas_limit: u64,
    to: [u8; 20],
    value: u128,
    data: Vec<u8>,
) -> Result<()> {
    let call = Call {
        ty,
        to,
        value,
        data,
    };

    process_call_operation(gas_limit, &call)?;

    // Create the call operation.
    *ctx.accounts.call_operation = Operation::new_call(call);

    // Update the message header.
    ctx.accounts.outgoing_message_header.gas_limit += gas_limit;
    ctx.accounts.outgoing_message_header.operation_count += 1;

    Ok(())
}
