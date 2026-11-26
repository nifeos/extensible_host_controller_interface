//! **An event the controller wrote**, decoded.
//!
//! Reach for it after the event ring says an entry is ready, to learn what the controller reported.

use super::*;

/// **One event, decoded.** The fields a caller uses, by event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Event {
    /// A transfer finished (or failed).
    #[non_exhaustive]
    Transfer {
        /// The physical address of the TRB it was about. That holds only while the caller issues
        /// no Event Data TRBs: for one of those the controller sets the Event Data bit (bit 2 of
        /// the last dword, xHCI 1.2 section 6.4.2.1) and puts the Event Data TRB's value here
        /// instead, and this decoder does not read that bit. The crate builds no Event Data TRB.
        transfer_request_block: u64,
        /// Bytes *not* transferred: zero for a full buffer, positive for a short packet.
        residual: u32,
        /// The completion code.
        code: CompletionCode,
        /// The device slot, or `None` when the field reads 0, the reserved slot id.
        slot: Option<SlotId>,
        /// The endpoint's device context index (1 for endpoint 0), or `None` when the field reads
        /// 0, which names no endpoint.
        endpoint: Option<DeviceContextIndex>,
    },
    /// A command finished.
    #[non_exhaustive]
    Command {
        /// The physical address of the command TRB.
        transfer_request_block: u64,
        /// The completion code.
        code: CompletionCode,
        /// The slot it concerns: for Enable Slot, the slot the controller chose. `None` when the
        /// field reads 0: a command that names no slot, or an Enable Slot the controller refused.
        slot: Option<SlotId>,
    },
    /// A port's status changed: the caller reads `PORTSC` to learn how.
    #[non_exhaustive]
    PortStatus {
        /// The port, or `None` when the field reads 0, the reserved port number.
        port: Option<PortNumber>,
    },
    /// Any other event type, which this crate does not decode.
    #[non_exhaustive]
    Other {
        /// Its type code.
        kind: u8,
    },
}

/// An event's pointer halves share no bit (dword 0 below, dword 1 shifted above bit 32), which
/// the compiler checks here; the mutation exclusion for [`Event::decode`] names this constant.
/// The constant-assertion lint is off on purpose: the assertion is constant by construction, and
/// the compiler checking it is its job.
#[allow(clippy::assertions_on_constants, clippy::eq_op)]
// This module is private, so nothing in the crate's code reads the constant; the mutation-testing
// exclusion for `Event::decode` refers to it by name, which is why it stays despite the dead-code
// allowance. Removing it leaves that exclusion without its justification.
#[allow(dead_code)]
pub(crate) const EVENT_POINTER_HALVES_DO_NOT_OVERLAP: () =
    assert!((((u32::MAX as u64) << 32) & u32::MAX as u64) == 0);

impl Event {
    /// Decode an event TRB. Total: every input is some variant.
    ///
    /// ```
    /// use extensible_host_controller_interface::{
    ///     CompletionCode, Event, SlotId, TransferRequestBlock,
    /// };
    ///
    /// // A Command Completion Event (type 33): the command at 0x1000 succeeded, for slot 1.
    /// let written = TransferRequestBlock::from_dwords([0x1000, 0, 1 << 24, 1 << 24 | 33 << 10 | 1]);
    /// match Event::decode(written) {
    ///     Event::Command { transfer_request_block, code, slot, .. } => {
    ///         assert_eq!(transfer_request_block, 0x1000);
    ///         assert_eq!(code, CompletionCode::SUCCESS);
    ///         assert_eq!(slot, SlotId::new(1));
    ///     }
    ///     other => panic!("not a command completion: {other:?}"),
    /// }
    /// ```
    pub const fn decode(transfer_request_block: TransferRequestBlock) -> Event {
        let words = transfer_request_block.0;
        let pointer = words[0] as u64 | (words[1] as u64) << 32;
        let code = CompletionCode::new((words[2] >> 24) as u8);
        let slot = SlotId::new((words[3] >> 24) as u8);
        match transfer_request_block.kind() {
            transfer_request_block_type::TRANSFER_EVENT => Event::Transfer {
                transfer_request_block: pointer,
                residual: words[2] & 0x00ff_ffff,
                code,
                slot,
                endpoint: DeviceContextIndex::new(((words[3] >> 16) & 0x1f) as u8),
            },
            transfer_request_block_type::COMMAND_COMPLETION => Event::Command {
                transfer_request_block: pointer,
                code,
                slot,
            },
            transfer_request_block_type::PORT_STATUS_CHANGE => Event::PortStatus {
                port: PortNumber::new((words[0] >> 24) as u8),
            },
            kind => Event::Other { kind },
        }
    }
}
