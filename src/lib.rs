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

//! # BUGS
//!
//! - One page size: [`direct_memory_access`] assumes 4 KiB (`PAGESIZE` bit 0), and the driver refuses otherwise.
//! - No streams, isochronous or bulk: the TRBs a keyboard needs and nothing more.
//! - Hubs are not modeled (route string 0), so a device behind a hub cannot be addressed.
//! - A [`ProducerRing`] never refuses a TRB: the caller keeps at most
//!   [`ProducerRing::capacity`] outstanding.
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
