//! **One Transfer Request Block (TRB)** and the builders for the TRBs a caller writes.
//!
//! Reach for it to build each command and transfer before placing it on a producer ring.

use super::*;

/// **One TRB**: four dwords, the cycle bit in bit 0 of the last. The controller reads each dword
/// little-endian; [`TransferRequestBlock::dwords`] gives them in the host's order.
///
/// The dwords are private, so the builders below are the way to make one. A TRB the controller
/// wrote, such as an event, comes in through [`TransferRequestBlock::from_dwords`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransferRequestBlock(pub(crate) [u32; 4]);

/// Interrupt On Completion: ask for an event when this TRB completes.
pub(crate) const INTERRUPT_ON_COMPLETION: u32 = 1 << 5;

/// Interrupt on Short Packet.
pub(crate) const INTERRUPT_ON_SHORT_PACKET: u32 = 1 << 2;

/// Immediate Data: the TRB's first two dwords are the data itself (the setup packet).
pub(crate) const IMMEDIATE_DATA: u32 = 1 << 6;

/// The transfer control word's fields share no bit: the type at 10:15 against the toggle, short
/// packet, completion and immediate-data bits at 1, 2, 5 and 6, all of them against dword 1's
/// fields at 16 and above, and the endpoint at 16:23 against the slot at 24:31. The compiler
/// checks it here; the mutation exclusions for this file's TRB builders name this constant. The
/// constant-assertion lint is off on purpose: the assertion is constant by construction, and the
/// compiler checking it is its job.
#[allow(clippy::assertions_on_constants, clippy::eq_op)]
// This module is private, so nothing in the crate's code reads the constants; the mutation-testing
// exclusions for the TRB builders refer to them by name, which is why they stay despite the
// dead-code allowance. Removing them leaves those exclusions without their justification.
#[allow(dead_code)]
pub(crate) const TRANSFER_CONTROL_FIELDS_DO_NOT_OVERLAP: () = assert!(
    ((0x3fu32 << 10) & (1 << 1 | 1 << 2 | 1 << 5 | 1 << 6)) == 0
        && ((0x3fu32 << 10 | 1 << 1 | 1 << 2 | 1 << 5 | 1 << 6) & (u32::MAX << 16)) == 0
        && ((0xffu32 << 16) & (0xff << 24)) == 0
);

/// The stamped cycle bit shares no bit with the word the [`TransferRequestBlock::with_cycle`]
/// masks first: the mask clears bit 0 and the stamp is a `bool`, so the compiler's own range
/// check closes the argument. The mutation exclusion for `with_cycle` names this constant. The
/// constant-assertion lint is off on purpose: the assertion is constant by construction, and the
/// compiler checking it is its job.
#[allow(clippy::assertions_on_constants, clippy::eq_op)]
// This module is private, so nothing in the crate's code reads the constant; the mutation-testing
// exclusion for `with_cycle` refers to it by name, which is why it stays despite the dead-code
// allowance. Removing it leaves that exclusion without its justification.
#[allow(dead_code)]
pub(crate) const CYCLE_BIT_AND_CLEARED_WORD_DO_NOT_OVERLAP: () = assert!(((!1u32) & 1) == 0);

/// **Whether `len` bytes at physical `buffer` can be one transfer TRB's data**: true when they do
/// not cross a 64 KiB boundary. A data buffer that a Transfer TRB references shall not span one
/// (xHCI 1.2 section 6.4.1, the note on 64 KiB boundaries); software chains several TRBs for a
/// buffer that does. A buffer that ends exactly at a boundary fits, and so does an empty one.
/// `len` is a `u16`, as the TRB builders take it, so a whole 64 KiB buffer is not asked about.
pub const fn fits_one_transfer_request_block(buffer: u64, len: u16) -> bool {
    (buffer & 0xffff) + len as u64 <= 0x1_0000
}

/// **A control transfer's shape**: the Transfer Type field of its Setup Stage TRB (xHCI 1.2
/// section 6.4.1.2.1), each variant's value its code. Code 1 is reserved and has no variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[doc(alias = "TRT")]
pub enum TransferType {
    /// No Data Stage: the Status Stage follows the Setup Stage.
    #[doc(alias = "No Data Stage")]
    NoDataStage = 0,
    /// A Data Stage host to device.
    #[doc(alias = "OUT Data Stage")]
    HostToDevice = 2,
    /// A Data Stage device to host.
    #[doc(alias = "IN Data Stage")]
    DeviceToHost = 3,
}

/// **Which way a stage's data flows**, as the Direction field of a Data or Status Stage TRB gives
/// it (xHCI 1.2 sections 6.4.1.2.2 and 6.4.1.2.3). The names are the USB specification's, which
/// are the host's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc(alias = "DIR")]
pub enum Direction {
    /// Host to device: Direction 0.
    #[doc(alias = "OUT")]
    Out,
    /// Device to host: Direction 1.
    #[doc(alias = "IN")]
    In,
}

impl Direction {
    /// The Direction field's value: 1 for [`Direction::In`], 0 for [`Direction::Out`].
    const fn field(self) -> u32 {
        match self {
            Direction::Out => 0,
            Direction::In => 1,
        }
    }
}

pub(crate) const fn control(kind: u8) -> u32 {
    (kind as u32) << 10
}

impl TransferRequestBlock {
    /// **A TRB from its four dwords**, as read from memory the controller wrote: an event, or an
    /// entry of a ring the caller inspects. Nothing is checked; [`Event::decode`] reads it.
    ///
    /// ```
    /// use extensible_host_controller_interface::TransferRequestBlock;
    ///
    /// let t = TransferRequestBlock::from_dwords([0x1000, 0, 0, 1]);
    /// assert_eq!((t.dwords(), t.is_cycle()), ([0x1000, 0, 0, 1], true));
    /// ```
    pub const fn from_dwords(dwords: [u32; 4]) -> TransferRequestBlock {
        TransferRequestBlock(dwords)
    }

    /// **The four dwords**, in the host's order, for the caller to write to memory: dwords 0 to 2
    /// first, then dword 3, which holds the cycle bit.
    pub const fn dwords(&self) -> [u32; 4] {
        self.0
    }

    /// **The TRB as the controller reads it**: sixteen bytes, each dword little-endian, for a
    /// caller that writes memory a byte at a time or runs big-endian.
    ///
    /// ```
    /// use extensible_host_controller_interface::TransferRequestBlock;
    ///
    /// let bytes = TransferRequestBlock::enable_slot().to_le_bytes();
    /// assert_eq!(bytes[12..], [0x00, 0x24, 0x00, 0x00]); // type 9 at bits 10 to 15
    /// assert_eq!(TransferRequestBlock::from_le_bytes(bytes), TransferRequestBlock::enable_slot());
    /// ```
    pub const fn to_le_bytes(&self) -> [u8; 16] {
        let [a, b, c, d] = self.0[0].to_le_bytes();
        let [e, f, g, h] = self.0[1].to_le_bytes();
        let [i, j, k, l] = self.0[2].to_le_bytes();
        let [m, n, o, p] = self.0[3].to_le_bytes();
        [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p]
    }

    /// **A TRB from the sixteen bytes the controller wrote**, each dword little-endian: the
    /// inverse of [`TransferRequestBlock::to_le_bytes`].
    pub const fn from_le_bytes(bytes: [u8; 16]) -> TransferRequestBlock {
        let [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p] = bytes;
        TransferRequestBlock([
            u32::from_le_bytes([a, b, c, d]),
            u32::from_le_bytes([e, f, g, h]),
            u32::from_le_bytes([i, j, k, l]),
            u32::from_le_bytes([m, n, o, p]),
        ])
    }

    /// (`interrupt_on_completion`, xHCI 1.2 specification section 6.4.1.1) Whether the TRB asks the
    /// controller for an event when it completes: the Interrupt On Completion bit, bit 5 of
    /// dword 3.
    #[doc(alias = "IOC")]
    #[doc(alias = "asks for a completion event")]
    pub const fn interrupt_on_completion(&self) -> bool {
        self.0[3] & INTERRUPT_ON_COMPLETION != 0
    }

    /// The TRB's type code.
    pub const fn kind(&self) -> u8 {
        ((self.0[3] >> 10) & 0x3f) as u8
    }

    /// The cycle bit.
    pub const fn is_cycle(&self) -> bool {
        self.0[3] & 1 != 0
    }

    /// The same TRB stamped with `cycle`.
    pub(crate) const fn with_cycle(self, cycle: bool) -> TransferRequestBlock {
        let mut words = self.0;
        words[3] = (words[3] & !1) | cycle as u32;
        TransferRequestBlock(words)
    }

    const fn pointer(physical_address: u64, rest: [u32; 2]) -> TransferRequestBlock {
        TransferRequestBlock([
            physical_address as u32,
            (physical_address >> 32) as u32,
            rest[0],
            rest[1],
        ])
    }

    /// A link TRB back to `target` with Toggle Cycle set, which is what flips the consumer's
    /// cycle as it follows the link.
    #[must_use]
    pub const fn link(target: u64) -> TransferRequestBlock {
        TransferRequestBlock::pointer(
            target,
            [0, control(transfer_request_block_type::LINK) | 1 << 1],
        )
    }

    /// Enable Slot: ask the controller for a device slot.
    #[must_use]
    pub const fn enable_slot() -> TransferRequestBlock {
        TransferRequestBlock([0, 0, 0, control(transfer_request_block_type::ENABLE_SLOT)])
    }

    /// Disable Slot `slot`: give it back.
    #[must_use]
    pub const fn disable_slot(slot: SlotId) -> TransferRequestBlock {
        TransferRequestBlock([
            0,
            0,
            0,
            control(transfer_request_block_type::DISABLE_SLOT) | (slot.get() as u32) << 24,
        ])
    }

    /// (`address_device`, xHCI 1.2 specification section 4.6.5) Address Device: take the input
    /// context at `input` (physical) for `slot`, and send the device its address.
    #[doc(alias = "Address Device command")]
    #[must_use]
    pub const fn address_device(input: u64, slot: SlotId) -> TransferRequestBlock {
        TransferRequestBlock::pointer(
            input,
            [
                0,
                control(transfer_request_block_type::DEVICE_ADDRESSING_COMMAND)
                    | (slot.get() as u32) << 24,
            ],
        )
    }

    /// Configure Endpoint: add the endpoints the input context's add flags name.
    #[must_use]
    pub const fn configure_endpoint(input: u64, slot: SlotId) -> TransferRequestBlock {
        TransferRequestBlock::pointer(
            input,
            [
                0,
                control(transfer_request_block_type::CONFIGURE_ENDPOINT)
                    | (slot.get() as u32) << 24,
            ],
        )
    }

    /// Evaluate Context: update the fields the input context names (here, endpoint 0's packet
    /// size once the device has said what it is).
    #[must_use]
    pub const fn evaluate_context(input: u64, slot: SlotId) -> TransferRequestBlock {
        TransferRequestBlock::pointer(
            input,
            [
                0,
                control(transfer_request_block_type::EVALUATE_CONTEXT) | (slot.get() as u32) << 24,
            ],
        )
    }

    /// Reset Endpoint `device_context_index` of `slot`, after a stall halted it. The index is at
    /// most 31, so it keeps to the five bits of the Endpoint ID field (xHCI 1.2 section 6.4.3.7).
    #[must_use]
    pub const fn reset_endpoint(
        slot: SlotId,
        device_context_index: DeviceContextIndex,
    ) -> TransferRequestBlock {
        TransferRequestBlock([
            0,
            0,
            0,
            control(transfer_request_block_type::RESET_ENDPOINT)
                | (device_context_index.get() as u32) << 16
                | (slot.get() as u32) << 24,
        ])
    }

    /// Set TR Dequeue Pointer of endpoint `device_context_index` of `slot` to `pointer` (the ring
    /// position with the consumer cycle in bit 0, [`ProducerRing::dequeue_pointer`]), skipping
    /// what a stall left on the ring. The index is at most 31, so it keeps to the five bits of the
    /// Endpoint ID field (xHCI 1.2 section 6.4.3.9).
    #[must_use]
    pub const fn set_dequeue(
        pointer: u64,
        slot: SlotId,
        device_context_index: DeviceContextIndex,
    ) -> TransferRequestBlock {
        TransferRequestBlock::pointer(
            pointer,
            [
                0,
                control(transfer_request_block_type::SET_TRANSFER_RING_DEQUEUE_POINTER)
                    | (device_context_index.get() as u32) << 16
                    | (slot.get() as u32) << 24,
            ],
        )
    }

    /// A control transfer's Setup Stage, the eight-byte packet carried immediately, and
    /// `transfer_type` saying whether a Data Stage follows and which way it flows.
    #[must_use]
    pub const fn setup(packet: u64, transfer_type: TransferType) -> TransferRequestBlock {
        let transfer_type = transfer_type as u32;
        TransferRequestBlock([
            packet as u32,
            (packet >> 32) as u32,
            8,
            control(transfer_request_block_type::SETUP) | IMMEDIATE_DATA | transfer_type << 16,
        ])
    }

    /// A control transfer's Data Stage: `len` bytes at `buffer` (physical), flowing `direction`,
    /// which agrees with the Setup Stage's [`TransferType`]. The bytes must not cross a 64 KiB
    /// boundary ([`fits_one_transfer_request_block`] says whether they do); this builder does not
    /// check.
    ///
    /// `len` is a `u16`, so one TRB carries at most 65,535 bytes, though the Transfer Length field
    /// holds 17 bits and a TRB may carry 65,536 (xHCI 1.2 section 6.4.1.1). A control or interrupt
    /// transfer never comes near that; a builder for bulk transfers would take a `u32`.
    #[must_use]
    pub const fn data(buffer: u64, len: u16, direction: Direction) -> TransferRequestBlock {
        TransferRequestBlock::pointer(
            buffer,
            [
                len as u32,
                control(transfer_request_block_type::DATA)
                    | direction.field() << 16
                    | INTERRUPT_ON_SHORT_PACKET,
            ],
        )
    }

    /// A control transfer's Status Stage, flowing `direction`: opposite to the Data Stage, and
    /// [`Direction::In`] when there is none (xHCI 1.2 section 4.11.2.2). Asks for the completion
    /// event.
    ///
    /// ```
    /// use extensible_host_controller_interface::{Direction, TransferRequestBlock, TransferType};
    ///
    /// // GET_DESCRIPTOR: the device sends 18 bytes, and the status stage answers host to device.
    /// let stages = [
    ///     TransferRequestBlock::setup(0x0012_0000_0100_0680, TransferType::DeviceToHost),
    ///     TransferRequestBlock::data(0x20_0000, 18, Direction::In),
    ///     TransferRequestBlock::status(Direction::Out),
    /// ];
    /// assert!(stages[2].interrupt_on_completion());
    /// ```
    #[must_use]
    pub const fn status(direction: Direction) -> TransferRequestBlock {
        TransferRequestBlock([
            0,
            0,
            0,
            control(transfer_request_block_type::STATUS)
                | direction.field() << 16
                | INTERRUPT_ON_COMPLETION,
        ])
    }

    /// A Normal TRB: `len` bytes at `buffer` (physical), with an event on completion or a short
    /// packet. What an interrupt `IN` endpoint's ring holds. The bytes must not cross a 64 KiB
    /// boundary ([`fits_one_transfer_request_block`] says whether they do); this builder does not
    /// check.
    ///
    /// `len` is a `u16`, so one TRB carries at most 65,535 bytes, though the Transfer Length field
    /// holds 17 bits and a TRB may carry 65,536 (xHCI 1.2 section 6.4.1.1). A control or interrupt
    /// transfer never comes near that; a builder for bulk transfers would take a `u32`.
    #[must_use]
    pub const fn normal(buffer: u64, len: u16) -> TransferRequestBlock {
        TransferRequestBlock::pointer(
            buffer,
            [
                len as u32,
                control(transfer_request_block_type::NORMAL)
                    | INTERRUPT_ON_COMPLETION
                    | INTERRUPT_ON_SHORT_PACKET,
            ],
        )
    }
}
