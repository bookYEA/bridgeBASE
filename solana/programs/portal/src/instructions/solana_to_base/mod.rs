pub mod send_call;
pub mod send_message;

pub use send_call::*;
pub use send_message::*;

pub struct Call {
    pub to: [u8; 20],
    pub gas_limit: u64,
    pub is_creation: bool,
    pub data: Vec<u8>,
}
