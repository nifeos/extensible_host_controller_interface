/// Normal: a data buffer on a bulk or interrupt ring.
pub const NORMAL: u8 = 1;
/// Setup Stage of a control transfer.
pub const SETUP: u8 = 2;
/// Data Stage of a control transfer.
pub const DATA: u8 = 3;
/// Status Stage of a control transfer.
pub const STATUS: u8 = 4;
/// Link: the producer ring's way back to its start.
pub const LINK: u8 = 6;
/// Enable Slot command.
pub const ENABLE_SLOT: u8 = 9;
/// Disable Slot command.
pub const DISABLE_SLOT: u8 = 10;
/// Address Device command.
#[doc(alias = "Address Device")]
pub const DEVICE_ADDRESSING_COMMAND: u8 = 11;
/// Configure Endpoint command.
pub const CONFIGURE_ENDPOINT: u8 = 12;
/// Evaluate Context command.
pub const EVALUATE_CONTEXT: u8 = 13;
/// Reset Endpoint command: take an endpoint out of the Halted state a stall left it in.
pub const RESET_ENDPOINT: u8 = 14;
/// Set TR Dequeue Pointer command: move an endpoint's ring past what a stall abandoned.
#[doc(alias = "Set TR Dequeue Pointer")]
pub const SET_TRANSFER_RING_DEQUEUE_POINTER: u8 = 16;
/// Transfer Event.
pub const TRANSFER_EVENT: u8 = 32;
/// Command Completion Event.
pub const COMMAND_COMPLETION: u8 = 33;
/// Port Status Change Event.
pub const PORT_STATUS_CHANGE: u8 = 34;
