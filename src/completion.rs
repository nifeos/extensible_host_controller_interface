/// The command or transfer did what was asked.
pub const SUCCESS: u8 = 1;
/// USB transaction error: the device did not answer, or answered garbage.
pub const TRANSACTION_ERROR: u8 = 4;
/// The device stalled the endpoint: it refused the request.
pub const STALL: u8 = 6;
/// The transfer finished with fewer bytes than the buffer: normal for a descriptor read.
pub const SHORT_PACKET: u8 = 13;
/// No slot was free for Enable Slot.
pub const NO_SLOTS: u8 = 9;
