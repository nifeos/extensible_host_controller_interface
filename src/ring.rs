//! **The rings**: the producer rings the driver fills and the event ring it drains.

use super::*;

/// **A ring the driver produces into**: the command ring and every transfer ring. Its last entry is
/// always a link TRB back to the first.
///
/// # Examples
///
/// Four entries are three usable slots and a link TRB; the third enqueue also writes the link, and
/// the cycle flips for the next lap. The four enqueues below show the wrap only: a driver keeps at
/// most [`capacity`](ProducerRing::capacity) of them outstanding, here two, and waits for the
/// controller to consume one before it places the next.
///
/// ```
/// use extensible_host_controller_interface::{ProducerRing, TransferRequestBlock};
///
/// let mut ring = ProducerRing::new(0x1000, 4);
/// assert_eq!(ring.capacity(), 2);
/// let a = ring.enqueue(TransferRequestBlock::enable_slot());
/// assert_eq!((a.index, a.transfer_request_block.is_cycle(), a.link), (0, true, None));
/// ring.enqueue(TransferRequestBlock::enable_slot());
/// let c = ring.enqueue(TransferRequestBlock::enable_slot());
/// assert_eq!(c.index, 2);
/// let (at, link) = c.link.unwrap(); // the third TRB fills the lap, so the link is written too
/// assert_eq!(at, 3);
/// assert!(link.is_cycle()); // stamped with the lap it ends, so the controller follows it
/// let d = ring.enqueue(TransferRequestBlock::enable_slot());
/// assert_eq!((d.index, d.transfer_request_block.is_cycle()), (0, false)); // second lap, cycle flipped
/// assert_eq!(ring.entry_physical_address(d.index), 0x1000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProducerRing {
    pub(crate) base: u64,
    pub(crate) entries: u16,
    pub(crate) enqueue: u16,
    pub(crate) cycle: bool,
    /// The index of the oldest TRB the caller has not reported through
    /// [`complete`](ProducerRing::complete): where the controller's dequeue pointer is, as far as
    /// the completion events have said.
    pub(crate) completed: u16,
}

/// **What [`ProducerRing::enqueue`] says to write**: the TRB at `index`, stamped with this lap's
/// cycle, and when the ring has just filled its lap, the link TRB at the last index too.
///
/// The driver writes `transfer_request_block` and then `link`, each with the cycle dword last: until
/// a TRB's cycle bit matches, the controller treats it as not yet produced, so the dword that
/// carries it goes in after the rest. [`ProducerRing::enqueue`] states the order and the barriers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferRequestBlockPlacement {
    /// Where the TRB goes.
    pub index: u16,
    /// The TRB, cycle stamped.
    pub transfer_request_block: TransferRequestBlock,
    /// The link TRB and where it goes, when this enqueue filled the lap.
    pub link: Option<(u16, TransferRequestBlock)>,
}

impl ProducerRing {
    /// **The fewest entries a producer ring may have**: the link, one entry that stays empty
    /// (xHCI 1.2 section 4.9.2.2) and one TRB, so that [`capacity`](Self::capacity) is at least 1.
    pub const MINIMUM_SEGMENT_ENTRIES: u16 = 3;

    /// **The most entries a producer ring may have**: a transfer or command ring segment is at most
    /// 64 KiB (xHCI 1.2 Table 6-1), which is 4096 TRBs of 16 bytes.
    pub const MAXIMUM_SEGMENT_ENTRIES: u16 = 4096;

    /// A ring of `entries` TRBs (the last of them the link) at physical `base`, in the state a
    /// zeroed ring starts in: producer cycle 1, so the zeros read as not produced.
    ///
    /// # Panics
    /// When `entries` is outside [`MINIMUM_SEGMENT_ENTRIES`](Self::MINIMUM_SEGMENT_ENTRIES) to
    /// [`MAXIMUM_SEGMENT_ENTRIES`](Self::MAXIMUM_SEGMENT_ENTRIES); every caller passes a constant.
    pub const fn new(base: u64, entries: u16) -> ProducerRing {
        assert!(
            entries >= Self::MINIMUM_SEGMENT_ENTRIES && entries <= Self::MAXIMUM_SEGMENT_ENTRIES,
            "a producer ring holds 3 to 4096 entries"
        );
        ProducerRing {
            base,
            entries,
            enqueue: 0,
            cycle: true,
            completed: 0,
        }
    }

    /// **The most TRBs the caller may have outstanding**: placed, and not yet consumed by the
    /// controller. One entry is the link, and one more stays empty, because the ring is full when
    /// advancing the Enqueue Pointer would make it equal to the Dequeue Pointer (xHCI 1.2 section
    /// 4.9.2.2). Placing more overwrites a TRB the controller has not reached.
    pub const fn capacity(&self) -> u16 {
        self.entries - 2
    }

    /// The physical address of entry `index`.
    pub const fn entry_physical_address(&self, index: u16) -> u64 {
        self.base + index as u64 * 16
    }

    /// The physical address and cycle of the next TRB to be written: what a Transfer Ring (TR) Dequeue Pointer or
    /// `CRCR` is set to (the cycle in bit 0, as both registers take it).
    pub const fn dequeue_pointer(&self) -> u64 {
        self.entry_physical_address(self.enqueue) | self.cycle as u64
    }

    /// **How many TRBs are outstanding**: placed, and not yet reported through
    /// [`complete`](Self::complete).
    const fn outstanding(&self) -> u16 {
        let usable = self.entries - 1;
        (self.enqueue + usable - self.completed) % usable
    }

    /// **Place one TRB if the ring has room**, as [`enqueue`](Self::enqueue) does, or `None` when
    /// [`capacity`](Self::capacity) TRBs are already outstanding. The ring knows only what
    /// [`complete`](Self::complete) has told it, so a caller that uses this reports every
    /// completion event for the ring there.
    pub fn try_enqueue(
        &mut self,
        transfer_request_block: TransferRequestBlock,
    ) -> Option<TransferRequestBlockPlacement> {
        if self.outstanding() == self.capacity() {
            return None;
        }
        Some(self.enqueue(transfer_request_block))
    }

    /// **Report a completion event** for the TRB at `transfer_request_block_physical_address`
    /// (the event's TRB pointer): the controller has consumed it and every TRB before it, so
    /// their entries are free for [`try_enqueue`](Self::try_enqueue). Answers `false`, and changes
    /// nothing, when the address is not an outstanding TRB of this ring, such as a stale or
    /// foreign event.
    pub fn complete(&mut self, transfer_request_block_physical_address: u64) -> bool {
        let usable = self.entries - 1;
        let Some(offset) = transfer_request_block_physical_address.checked_sub(self.base) else {
            return false;
        };
        if offset % 16 != 0 || offset / 16 >= u64::from(usable) {
            return false;
        }
        let index = (offset / 16) as u16;
        if (index + usable - self.completed) % usable >= self.outstanding() {
            return false;
        }
        self.completed = (index + 1) % usable;
        true
    }

    /// **Place one TRB**, and the link if this fills the lap.
    ///
    /// This never refuses: the caller keeps at most [`capacity`](Self::capacity) TRBs
    /// outstanding, counting each one out when its completion event arrives.
    /// [`try_enqueue`](Self::try_enqueue) does that counting for a caller that reports each event
    /// through [`complete`](Self::complete).
    ///
    /// The caller then writes the placement to memory in this order (xHCI 1.2 section 4.9.2:
    /// the controller takes a TRB as produced once its cycle bit matches). First dwords 0 to 2 of
    /// the TRB at [`entry_physical_address`](Self::entry_physical_address)`(index)`, then a write
    /// barrier, then dword 3, which holds the cycle bit. The link, when there is one, goes the
    /// same way after it. Last comes a write barrier that orders all of that memory before the
    /// doorbell register write. On a weakly ordered machine (aarch64, riscv64) both barriers are
    /// instructions, not only compiler fences; Linux spells them `wmb()`.
    pub fn enqueue(
        &mut self,
        transfer_request_block: TransferRequestBlock,
    ) -> TransferRequestBlockPlacement {
        let index = self.enqueue;
        let placed = transfer_request_block.with_cycle(self.cycle);
        self.enqueue += 1;
        let mut link = None;
        if self.enqueue == self.entries - 1 {
            link = Some((
                self.enqueue,
                TransferRequestBlock::link(self.base).with_cycle(self.cycle),
            ));
            self.enqueue = 0;
            self.cycle = !self.cycle;
        }
        TransferRequestBlockPlacement {
            index,
            transfer_request_block: placed,
            link,
        }
    }
}

/// **The event ring, from the consumer's side**: one segment of `entries` TRBs at a physical base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventRing {
    pub(crate) base: u64,
    pub(crate) entries: u16,
    pub(crate) dequeue: u16,
    pub(crate) cycle: bool,
}

impl EventRing {
    /// **The fewest entries an event ring segment may have** (xHCI 1.2 section 6.5, Table 6-95,
    /// the Ring Segment Size field).
    pub const MINIMUM_SEGMENT_ENTRIES: u16 = 16;

    /// **The most entries an event ring segment may have** (xHCI 1.2 section 6.5, Table 6-95).
    pub const MAXIMUM_SEGMENT_ENTRIES: u16 = 4096;

    /// A zeroed ring: consumer cycle 1, so the zeros read as not yet written.
    ///
    /// # Panics
    /// When `entries` is outside [`MINIMUM_SEGMENT_ENTRIES`](Self::MINIMUM_SEGMENT_ENTRIES) to
    /// [`MAXIMUM_SEGMENT_ENTRIES`](Self::MAXIMUM_SEGMENT_ENTRIES), which a controller refuses; every
    /// caller passes a constant.
    pub const fn new(base: u64, entries: u16) -> EventRing {
        assert!(
            entries >= Self::MINIMUM_SEGMENT_ENTRIES && entries <= Self::MAXIMUM_SEGMENT_ENTRIES,
            "an event ring segment holds 16 to 4096 entries"
        );
        EventRing {
            base,
            entries,
            dequeue: 0,
            cycle: true,
        }
    }

    /// The entry the next event will be at.
    pub const fn index(&self) -> u16 {
        self.dequeue
    }

    /// Whether `transfer_request_block`, read at [`index`](Self::index), is an event the controller
    /// has written this lap.
    ///
    /// The controller writes an event's dword 3, which holds the cycle bit, after the other three
    /// (xHCI 1.2 section 4.11.3, the implementation note on Event TRB updating). So the caller
    /// reads dword 3 first and asks this; only when it is ready does a read barrier follow, and
    /// then dwords 0 to 2. Reading all four and then asking can pair a fresh cycle bit with stale
    /// data on a weakly ordered machine (aarch64, riscv64), where the barrier is an instruction;
    /// Linux spells it `rmb()`. After the event is handled, [`advance`](Self::advance) and the
    /// `ERDP` write follow (xHCI 1.2 section 4.9.4).
    pub const fn is_ready(&self, transfer_request_block: &TransferRequestBlock) -> bool {
        transfer_request_block.is_cycle() == self.cycle
    }

    /// Consume the entry at [`index`](Self::index).
    pub fn advance(&mut self) {
        self.dequeue += 1;
        if self.dequeue == self.entries {
            self.dequeue = 0;
            self.cycle = !self.cycle;
        }
    }

    /// The physical address to write to `ERDP`: where the next event will be.
    pub const fn dequeue_pointer(&self) -> u64 {
        self.base + self.dequeue as u64 * 16
    }

    /// The one event ring segment table entry describing this ring.
    pub const fn segment_table_entry(&self) -> [u32; 4] {
        [
            self.base as u32,
            (self.base >> 32) as u32,
            self.entries as u32,
            0,
        ]
    }
}
