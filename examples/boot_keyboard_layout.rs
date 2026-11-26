//! **One driver's DMA region layout**: how a single-slot boot-keyboard driver might lay out the
//! structures it shares with the controller, one page each from the region's base. Run it with
//! `cargo run --example boot_keyboard_layout`.
//!
//! The crate fixes no layout: a driver lays out its own region, and this is one such layout, not
//! a rule. The pages through [`SCRATCHPAD_ARRAY`] are the driver's own. The controller's
//! scratchpad pages follow and need not be mapped into the driver: it writes their physical
//! addresses into [`SCRATCHPAD_ARRAY`] and never touches them. An input-output memory management
//! unit (IOMMU), where there is one, confines the device to the whole region.
//!
//! The layout assumes 4 KiB pages, bit 0 of the Page Size register, and a driver that takes it
//! refuses a controller without that bit.

use extensible_host_controller_interface::{Capabilities, EventRing, ProducerRing};

/// The page size this layout is in: 4 KiB, bit 0 of the Page Size register.
const PAGE_BYTES: u64 = 4096;

/// The device context base address array: one pointer per slot, entry 0 the scratchpad array.
pub const DEVICE_CONTEXT_ARRAY: u64 = 0;
/// The command ring.
pub const COMMAND_RING: u64 = 1;
/// The event ring's one segment.
pub const EVENT_RING: u64 = 2;
/// The event ring segment table (one entry).
pub const EVENT_SEGMENT_TABLE: u64 = 3;
/// The input context, rewritten for each command that takes one.
pub const INPUT_CONTEXT: u64 = 4;
/// The output device context of the one slot this driver drives at a time.
pub const DEVICE_CONTEXT: u64 = 5;
/// Endpoint 0's transfer ring.
pub const CONTROL_RING: u64 = 6;
/// The keyboard's interrupt `IN` transfer ring.
pub const INTERRUPT_RING: u64 = 7;
/// The control transfers' data buffer: descriptors land here.
pub const CONTROL_BUFFER: u64 = 8;
/// The interrupt transfers' buffers: one [`REPORT_SLOT`]-byte slot per queued report.
pub const REPORT_BUFFERS: u64 = 9;
/// The scratchpad buffer array: one pointer per scratchpad page.
pub const SCRATCHPAD_ARRAY: u64 = 10;

/// How many TRBs a ring page holds, the last of a producer ring's being its link.
pub const RING_ENTRIES: u16 = 256;
/// The byte stride of one report buffer, a cache line so no two transfers share one.
pub const REPORT_SLOT: u64 = 64;
/// How many interrupt transfers the driver keeps queued, each into its own report slot. More than
/// one, so a key arriving while the driver handles the last does not wait for a requeue.
pub const REPORTS_QUEUED: u16 = 4;

/// The most scratchpad pages the scratchpad array's one page can name.
pub const MAXIMUM_SCRATCHPADS: u16 = 512;

// The queued reports fit their page, the ring holds every one of them and its link, and the
// scratchpad array's page names every scratchpad this layout accepts. Checked by the compiler, so
// an edit to any one constant that breaks the others fails the build.
const _: () = assert!(REPORTS_QUEUED as u64 * REPORT_SLOT <= PAGE_BYTES);
const _: () = assert!(REPORTS_QUEUED < RING_ENTRIES - 1);
const _: () = assert!(MAXIMUM_SCRATCHPADS as u64 * 8 == PAGE_BYTES);

/// Where page `page` of a region at physical `base` starts.
const fn page_physical_address(base: u64, page: u64) -> u64 {
    base + page * PAGE_BYTES
}

fn main() {
    // QEMU's qemu-xhci: 64 slots and no scratchpad pages, at a region placed at 1 MiB.
    let capabilities =
        Capabilities::decode(0x0100_0040, 0x0800_1040, 0, 0x0000_7000, 0x2000, 0x1000);
    let base = 0x10_0000;
    assert!(
        capabilities.scratchpads() <= MAXIMUM_SCRATCHPADS,
        "the scratchpad array's one page names every scratchpad"
    );
    assert!(
        base + (SCRATCHPAD_ARRAY + 1 + u64::from(capabilities.scratchpads())) * PAGE_BYTES
            <= 1 << 32
            || capabilities.has_64_bit_addressing(),
        "a controller without 64-bit addressing reaches only the low 4 GiB"
    );
    let command_ring = ProducerRing::new(page_physical_address(base, COMMAND_RING), RING_ENTRIES);
    let event_ring = EventRing::new(page_physical_address(base, EVENT_RING), RING_ENTRIES);
    let pages = [
        ("device context base address array", DEVICE_CONTEXT_ARRAY),
        ("command ring", COMMAND_RING),
        ("event ring", EVENT_RING),
        ("event ring segment table", EVENT_SEGMENT_TABLE),
        ("input context", INPUT_CONTEXT),
        ("output device context", DEVICE_CONTEXT),
        ("control transfer ring", CONTROL_RING),
        ("interrupt transfer ring", INTERRUPT_RING),
        ("control buffer", CONTROL_BUFFER),
        ("report buffers", REPORT_BUFFERS),
        ("scratchpad buffer array", SCRATCHPAD_ARRAY),
    ];
    for (name, page) in pages {
        println!("{:#010x} {name}", page_physical_address(base, page));
    }
    println!(
        "{} commands fit the command ring at {:#x}; the event ring's first entry is {:#x}",
        command_ring.capacity(),
        command_ring.entry_physical_address(0),
        event_ring.dequeue_pointer(),
    );
    for report in 0..REPORTS_QUEUED {
        println!(
            "report {report} lands at {:#x}",
            page_physical_address(base, REPORT_BUFFERS) + u64::from(report) * REPORT_SLOT
        );
    }
}
