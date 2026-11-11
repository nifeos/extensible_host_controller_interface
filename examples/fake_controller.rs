//! **A fake controller**: the driver's side of a control transfer against a controller simulated
//! in memory, with no hardware. Run it with `cargo run --example fake_controller`.
//!
//! The driver places a `GET_DESCRIPTOR` control transfer on a transfer ring (Setup, Data and
//! Status Stage TRBs) and rings the doorbell. The fake controller then consumes the ring the way
//! xHCI 1.2 section 4.9.2 says a controller does: from its dequeue pointer, while each TRB's cycle
//! bit matches its own, following the link back to the start. For the Status Stage, which asks
//! for an event, it writes a Transfer Event to the event ring. The driver drains the event ring,
//! decodes the event, and reports it to the transfer ring so that `try_enqueue` has room again.
//!
//! Physical memory here is a vector of TRBs indexed by address. A real driver writes the same
//! values into DMA memory with the ordering and barriers `ProducerRing::enqueue` and
//! `EventRing::is_ready` describe.

use extensible_host_controller_interface::{
    Event, EventRing, ProducerRing, TransferRequestBlock, TransferType, completion,
    transfer_request_block_type,
};

/// Where the fake's physical memory starts, and how many TRBs it holds.
const MEMORY_BASE: u64 = 0x10_0000;
const MEMORY_ENTRIES: usize = 64;
/// The transfer ring's entries, then the event ring's, in that memory.
const TRANSFER_RING: u64 = MEMORY_BASE;
const TRANSFER_RING_ENTRIES: u16 = 8;
const EVENT_RING: u64 = MEMORY_BASE + 0x100;
/// The device's descriptor buffer: outside the TRB memory, since the fake moves no data.
const DESCRIPTOR_BUFFER: u64 = 0x20_0000;
/// The device slot and endpoint 0's device context index.
const SLOT: u8 = 1;
const ENDPOINT_ZERO: u8 = 1;

/// The fake's memory: one TRB per 16 bytes from [`MEMORY_BASE`].
struct Memory(Vec<TransferRequestBlock>);

impl Memory {
    fn read(&self, physical_address: u64) -> TransferRequestBlock {
        self.0[((physical_address - MEMORY_BASE) / 16) as usize]
    }

    fn write(&mut self, physical_address: u64, transfer_request_block: TransferRequestBlock) {
        self.0[((physical_address - MEMORY_BASE) / 16) as usize] = transfer_request_block;
    }
}

/// **The simulated controller**: one transfer ring's consumer and the event ring's producer.
struct FakeController {
    dequeue: u64,
    consumer_cycle: bool,
    event_enqueue: u64,
    event_cycle: bool,
    event_ring_entries: u16,
}

impl FakeController {
    /// What a doorbell does: run every TRB the driver produced, and write an event for each one
    /// that asks for it (Interrupt On Completion, bit 5 of the last dword).
    fn ring_doorbell(&mut self, memory: &mut Memory) {
        loop {
            let t = memory.read(self.dequeue);
            if t.is_cycle() != self.consumer_cycle {
                return;
            }
            if t.kind() == transfer_request_block_type::LINK {
                if t.0[3] & 1 << 1 != 0 {
                    self.consumer_cycle = !self.consumer_cycle;
                }
                self.dequeue = u64::from(t.0[0]) | u64::from(t.0[1]) << 32;
                continue;
            }
            if t.0[3] & 1 << 5 != 0 {
                self.post_transfer_event(memory, self.dequeue, completion::SUCCESS);
            }
            self.dequeue += 16;
        }
    }

    /// Write a Transfer Event (xHCI 1.2 section 6.4.2.1) for the TRB at `pointer`: nothing left
    /// untransferred, completion `code`, the slot and endpoint, and this lap's cycle bit.
    fn post_transfer_event(&mut self, memory: &mut Memory, pointer: u64, code: u8) {
        let event = TransferRequestBlock([
            pointer as u32,
            (pointer >> 32) as u32,
            u32::from(code) << 24,
            u32::from(SLOT) << 24
                | u32::from(ENDPOINT_ZERO) << 16
                | u32::from(transfer_request_block_type::TRANSFER_EVENT) << 10
                | u32::from(self.event_cycle),
        ]);
        memory.write(self.event_enqueue, event);
        self.event_enqueue += 16;
        if self.event_enqueue == EVENT_RING + u64::from(self.event_ring_entries) * 16 {
            self.event_enqueue = EVENT_RING;
            self.event_cycle = !self.event_cycle;
        }
    }
}

fn main() {
    let mut memory = Memory(vec![TransferRequestBlock([0; 4]); MEMORY_ENTRIES]);
    let mut transfers = ProducerRing::new(TRANSFER_RING, TRANSFER_RING_ENTRIES);
    let mut events = EventRing::new(EVENT_RING, EventRing::MINIMUM_SEGMENT_ENTRIES);
    let mut controller = FakeController {
        dequeue: transfers.dequeue_pointer() & !1,
        consumer_cycle: transfers.dequeue_pointer() & 1 != 0,
        event_enqueue: EVENT_RING,
        event_cycle: true,
        event_ring_entries: EventRing::MINIMUM_SEGMENT_ENTRIES,
    };

    // GET_DESCRIPTOR (device, 18 bytes): request type 0x80, request 6, value 0x0100, length 18.
    let packet = 0x0012_0000_0100_0680;
    // Seven usable entries less the one that stays empty hold two control transfers of three
    // TRBs; each round below reports its transfer complete, so the ring always has room.
    assert_eq!(transfers.capacity(), 6);
    let mut transfers_completed = 0;
    for round in 0..4 {
        let stages = [
            TransferRequestBlock::setup(packet, TransferType::DeviceToHost),
            TransferRequestBlock::data(DESCRIPTOR_BUFFER, 18, true),
            TransferRequestBlock::status(true),
        ];
        let mut status_at = 0;
        for stage in stages {
            let placement = transfers
                .try_enqueue(stage)
                .expect("the ring has room: every earlier transfer was reported complete");
            memory.write(
                transfers.entry_physical_address(placement.index),
                placement.transfer_request_block,
            );
            if let Some((at, link)) = placement.link {
                memory.write(transfers.entry_physical_address(at), link);
            }
            status_at = transfers.entry_physical_address(placement.index);
        }
        controller.ring_doorbell(&mut memory);

        // The driver drains the event ring. A real one reads dword 3 first, asks `is_ready`, and
        // reads the rest only after a read barrier; this memory needs no barrier.
        while events.is_ready(&memory.read(events.dequeue_pointer())) {
            let event = Event::decode(memory.read(events.dequeue_pointer()));
            events.advance();
            match event {
                Event::Transfer {
                    transfer_request_block,
                    code,
                    slot,
                    endpoint,
                    ..
                } => {
                    assert_eq!((slot, endpoint), (SLOT, ENDPOINT_ZERO));
                    assert_eq!(transfer_request_block, status_at);
                    println!(
                        "round {round}: transfer at {transfer_request_block:#x} on slot {slot} \
                         ended with {}",
                        completion::name(code)
                    );
                    assert!(transfers.complete(transfer_request_block));
                    transfers_completed += 1;
                }
                other => panic!("the fake controller wrote no such event: {other:?}"),
            }
        }
    }
    assert_eq!(transfers_completed, 4);
    println!("four control transfers ran across the transfer ring's wrap");
}
