#![no_std]
#![forbid(unsafe_code)]
//! **The extensible host controller interface (xHCI), as pure logic**: everything a USB host
//! controller driver computes, with nothing it touches. A kernel finds and confines the register
//! file and direct memory access (DMA) memory and a user-space driver drives them; this is the
//! arithmetic between.
//!
//! [`registers`] and [`Capabilities`] name the registers; [`direct_memory_access`] is the DMA region's layout; a
//! Transfer Request Block (TRB, [`TransferRequestBlock`]), [`Event`], [`ProducerRing`] and
//! [`EventRing`] are the ring discipline (xHCI 1.2, chapters 4 and 6); [`context`] holds slot and
//! endpoint contexts; [`extended_capabilities`] holds the extended capability ids; [`port`] is the
//! port status register.
//!
//! # EXAMPLES
//!
//! Decode the capability registers, then place a TRB on a command ring:
//!
//! ```
//! use extensible_host_controller_interface::{Capabilities, ProducerRing, TransferRequestBlock};
//!
//! // QEMU's qemu-xhci: 0x40 bytes of capability registers, 64 slots, 16 interrupters and 8
//! // ports, runtime at 0x1000, doorbells at 0x2000.
//! let capabilities = Capabilities::decode(0x0100_0040, 0x0800_1040, 0, 0x0000_7000, 0x2000, 0x1000);
//! assert_eq!(capabilities.port_status_and_control(1), Some(0x440));
//! assert_eq!(capabilities.doorbell(0), 0x2000);
//!
//! // A four-entry command ring: three usable slots, then a link TRB back to the first.
//! let mut ring = ProducerRing::new(0x1000, 4);
//! let placement = ring.try_enqueue(TransferRequestBlock::enable_slot()).unwrap();
//! assert_eq!((placement.index, placement.link), (0, None));
//! // Write placement.transfer_request_block at this address, then ring doorbell 0.
//! assert_eq!(ring.entry_physical_address(placement.index), 0x1000);
//! // When the Command Completion Event names that TRB, report it.
//! assert!(ring.complete(0x1000));
//! ```
//!
//! `examples/fake_controller.rs` in the repository runs whole control transfers against a
//! simulated controller.

//! # BUGS
//!
//! - One page size: [`direct_memory_access`] assumes 4 KiB (`PAGESIZE` bit 0), and the driver refuses otherwise.
//! - No streams, isochronous or bulk: the TRBs a keyboard needs and nothing more.
//! - Speeds are the default ids in [`port::speed`]: a controller whose
//!   [`extended_capabilities::SupportedProtocol::speed_id_count`] is not 0 defines its own, and
//!   nothing here decodes its Protocol Speed ID dwords.
//! - Hubs are not modeled (route string 0), so a device behind a hub cannot be addressed.
//! - [`ProducerRing::enqueue`] never refuses a TRB, and [`ProducerRing::try_enqueue`] refuses at
//!   [`ProducerRing::capacity`] only while the caller reports every completion through
//!   [`ProducerRing::complete`]; the ring cannot see the controller's dequeue pointer.
//! - The TRB builders do not check the 64 KiB rule (xHCI 1.2 section 6.4.1);
//!   [`fits_one_transfer_request_block`] says whether a buffer crosses a boundary.

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

pub mod context;
pub mod direct_memory_access;
pub mod extended_capabilities;
pub mod port;
pub mod registers;

pub use registers::Capabilities;

/// The page size every layout here is in.
pub const PAGE_BYTES: u64 = 4096;

/// **How long to wait for [`registers::STATUS_HALTED`]** after clearing [`registers::COMMAND_RUN`]. The controller halts
/// within 16 ms (xHCI 1.2 section 5.4.1, Table 5-20); this is Linux's `XHCI_MAX_HALT_USEC`, twice
/// that.
pub const HALT_TIMEOUT_MILLISECONDS: u64 = 32;
/// **How long to wait for [`registers::COMMAND_RESET`] and then [`registers::STATUS_NOT_READY`] to clear** after a
/// reset. The specification sets no bound; this is Linux's `XHCI_RESET_LONG_USEC`, the bound it
/// uses when it first brings a controller up.
pub const CONTROLLER_RESET_TIMEOUT_MILLISECONDS: u64 = 10_000;
/// **How long to wait for a Command Completion Event** after ringing doorbell 0. This is Linux's
/// `XHCI_CMD_DEFAULT_TIMEOUT`, which it takes from USB 3.2 section 9.2.6.1.
pub const COMMAND_COMPLETION_TIMEOUT_MILLISECONDS: u64 = 5000;

/// Transfer Request Block (TRB) type codes (xHCI 1.2 Table 6-91).
pub mod transfer_request_block_type;

/// Completion codes (xHCI 1.2 section 6.4.5, Table 6-90): every code the specification defines,
/// and [`completion::name`] for a transcript.
pub mod completion;

#[cfg(test)]
mod tests;

mod event;
mod ring;
mod transfer_request_block;

pub use context::interrupt_interval;
pub use event::Event;
pub use ring::{EventRing, ProducerRing, TransferRequestBlockPlacement};
pub use transfer_request_block::{
    TransferRequestBlock, TransferType, fits_one_transfer_request_block,
};

#[cfg(kani)]
mod verification;
