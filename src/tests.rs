//! The crate's host tests, kept out of `lib.rs` so the crate root stays an index of its modules.

extern crate std;
use super::*;
use crate::transfer_request_block::*;

#[test]
fn events_decode_by_type() {
    // A command completion for Enable Slot, success, slot 1.
    let t = TransferRequestBlock([0x1000, 0, 1 << 24, 33 << 10 | 1 << 24 | 1]);
    assert_eq!(
        Event::decode(t),
        Event::Command {
            transfer_request_block: 0x1000,
            code: completion::SUCCESS,
            slot: 1
        }
    );
    // A transfer event on endpoint 3 (EP1 IN), short by 2.
    let t = TransferRequestBlock([0x7000, 0, 13 << 24 | 2, 32 << 10 | 3 << 16 | 1 << 24 | 1]);
    assert_eq!(
        Event::decode(t),
        Event::Transfer {
            transfer_request_block: 0x7000,
            residual: 2,
            code: completion::SHORT_PACKET,
            slot: 1,
            endpoint: 3
        }
    );
    let t = TransferRequestBlock([5 << 24, 0, 1 << 24, 34 << 10 | 1]);
    assert_eq!(Event::decode(t), Event::PortStatus { port: 5 });
    // A TRB above 4 GiB keeps its high half.
    let t = TransferRequestBlock([0x1000, 0x2, 1 << 24, 33 << 10 | 1 << 24 | 1]);
    assert!(matches!(
        Event::decode(t),
        Event::Command {
            transfer_request_block: 0x2_0000_1000,
            ..
        }
    ));
}

/// The event ring's consumer flips its cycle at the wrap, so last lap's events read as stale.
#[test]
fn the_event_ring_laps() {
    let mut ring = EventRing::new(0x2000, 16);
    let fresh = TransferRequestBlock([0, 0, 0, 1]);
    assert!(ring.is_ready(&fresh));
    ring.advance();
    assert_eq!(ring.index(), 1);
    assert_eq!(ring.dequeue_pointer(), 0x2010);
    for _ in 1..16 {
        assert!(ring.is_ready(&fresh));
        ring.advance();
    }
    assert_eq!(ring.dequeue_pointer(), 0x2000);
    assert!(!ring.is_ready(&fresh));
    assert_eq!(ring.segment_table_entry(), [0x2000, 0, 16, 0]);
    // A ring above 4 GiB: the table entry's second dword is the base's high half.
    let high = EventRing::new(0x3_0000_2000, 4096);
    assert_eq!(high.segment_table_entry(), [0x2000, 3, 4096, 0]);
}

#[test]
fn a_setup_transfer_request_block_carries_the_packet_and_the_transfer_type() {
    let t = TransferRequestBlock::setup(0x0008_0000_0100_0680, TransferType::DeviceToHost);
    assert_eq!(t.0[0], 0x0100_0680);
    assert_eq!(t.0[1], 0x0008_0000);
    assert_eq!(t.0[2], 8);
    assert_eq!(t.kind(), transfer_request_block_type::SETUP);
    assert_eq!((t.0[3] >> 16) & 3, 3);
    assert!(t.0[3] & IMMEDIATE_DATA != 0);
    // The status stage of an IN transfer flows OUT, and of a no-data transfer flows IN.
    assert_eq!((TransferRequestBlock::status(true).0[3] >> 16) & 1, 0);
    assert_eq!((TransferRequestBlock::status(false).0[3] >> 16) & 1, 1);
}

#[test]
fn the_interval_field_follows_the_speed() {
    // Full speed, 10 ms: 80 microframes, so 2^6 = 64 is the largest period not over it.
    assert_eq!(interrupt_interval(port::speed::FULL, 10), 6);
    assert_eq!(interrupt_interval(port::speed::LOW, 1), 3);
    assert_eq!(interrupt_interval(port::speed::FULL, 255), 10);
    assert_eq!(interrupt_interval(port::speed::FULL, 0), 3);
    // High speed states an exponent plus one.
    assert_eq!(interrupt_interval(port::speed::HIGH, 4), 3);
    assert_eq!(interrupt_interval(port::speed::HIGH, 0), 0);
    assert_eq!(interrupt_interval(port::speed::SUPER, 200), 15);
}

/// **A simulated controller consuming what the driver produces**: it reads from its dequeue
/// index while the cycle bit matches its own, follows a link TRB and toggles its cycle when the
/// link says so (xHCI 1.2 section 4.9.2), and returns what it ran. Three laps of a four-entry
/// ring, so every wrap and every link is exercised: the controller must run exactly the TRBs
/// produced, in order, and stop where production stops.
#[test]
fn a_simulated_controller_runs_exactly_what_was_produced_across_laps() {
    const ENTRIES: u16 = 4;
    let base = 0x8000;
    let mut memory = [TransferRequestBlock([0; 4]); ENTRIES as usize];
    let mut ring = ProducerRing::new(base, ENTRIES);
    let (mut dequeue, mut consumer_cycle_state) = (0u16, true);
    let run = |memory: &[TransferRequestBlock; ENTRIES as usize],
               dequeue: &mut u16,
               consumer_cycle_state: &mut bool| {
        let mut ran = std::vec::Vec::new();
        for _ in 0..16 {
            let t = memory[*dequeue as usize];
            if t.is_cycle() != *consumer_cycle_state {
                break;
            }
            if t.kind() == transfer_request_block_type::LINK {
                let target = u64::from(t.0[0]) | u64::from(t.0[1]) << 32;
                assert_eq!(target, base, "a link leads back to the ring's start");
                if t.0[3] & 1 << 1 != 0 {
                    *consumer_cycle_state = !*consumer_cycle_state;
                }
                *dequeue = 0;
                continue;
            }
            ran.push(t.0[0]);
            *dequeue += 1;
        }
        ran
    };
    let mut next = 1u32;
    // Never more than the ring's capacity between two runs, which is the producer's rule, not
    // the ring's: one of the three usable entries stays empty, so a full ring never looks empty.
    for burst in [2u32, 1, 2, 2, 1, 2, 2] {
        let mut produced = std::vec::Vec::new();
        for _ in 0..burst {
            let p = ring.enqueue(TransferRequestBlock::normal(u64::from(next), 8));
            memory[p.index as usize] = p.transfer_request_block;
            if let Some((at, link)) = p.link {
                memory[at as usize] = link;
            }
            produced.push(next);
            next += 1;
        }
        assert_eq!(
            run(&memory, &mut dequeue, &mut consumer_cycle_state),
            produced
        );
        // Where the controller stopped is where the driver will write next.
        assert_eq!(ring.dequeue_pointer() & !1, base + u64::from(dequeue) * 16);
        assert_eq!(ring.dequeue_pointer() & 1 != 0, consumer_cycle_state);
    }
}

/// **The three shapes of a control transfer**, as a controller reads them: the setup stage's
/// transfer type must agree with the data stage's direction, and the status stage flows the
/// other way (xHCI 1.2 section 4.11.2.2). Only the status stage asks for an event on success,
/// which is what the driver waits for; the data stage asks only on a short packet.
#[test]
fn control_transfers_agree_on_direction_in_all_three_shapes() {
    for (shape, data_in, transfer_type, status_in) in [
        (TransferType::NoDataStage, None, 0, true),
        (TransferType::HostToDevice, Some(false), 2, true),
        (TransferType::DeviceToHost, Some(true), 3, false),
    ] {
        let setup = TransferRequestBlock::setup(0, shape);
        assert_eq!((setup.0[3] >> 16) & 3, transfer_type);
        let status = TransferRequestBlock::status(data_in == Some(true));
        assert_eq!(status.kind(), transfer_request_block_type::STATUS);
        assert_eq!((status.0[3] >> 16) & 1 != 0, status_in);
        assert!(status.0[3] & INTERRUPT_ON_COMPLETION != 0);
        if let Some(inbound) = data_in {
            let data = TransferRequestBlock::data(0x1_2345_6000, 18, inbound);
            assert_eq!(data.kind(), transfer_request_block_type::DATA);
            assert_eq!((data.0[0], data.0[1], data.0[2]), (0x2345_6000, 1, 18));
            assert_eq!((data.0[3] >> 16) & 1 != 0, inbound);
            assert_eq!(
                (
                    data.0[3] & INTERRUPT_ON_COMPLETION,
                    data.0[3] & INTERRUPT_ON_SHORT_PACKET != 0
                ),
                (0, true)
            );
        }
    }
    let normal = TransferRequestBlock::normal(0x4000, 8);
    assert_eq!(
        (normal.kind(), normal.0[2]),
        (transfer_request_block_type::NORMAL, 8)
    );
    assert!(
        normal.0[3] & INTERRUPT_ON_COMPLETION != 0 && normal.0[3] & INTERRUPT_ON_SHORT_PACKET != 0
    );
}

/// **Every command names its slot where a controller looks for it** (bits 31:24 of the last
/// dword), the pointer commands carry their input context, and the endpoint commands their
/// endpoint (bits 20:16). A simulated command dispatcher decodes each one back.
#[test]
fn every_command_decodes_back_to_its_slot_endpoint_and_pointer() {
    let decode = |t: TransferRequestBlock| {
        (
            t.kind(),
            (t.0[3] >> 24) as u8,
            ((t.0[3] >> 16) & 0x1f) as u8,
            u64::from(t.0[0]) | u64::from(t.0[1]) << 32,
        )
    };
    assert_eq!(
        decode(TransferRequestBlock::enable_slot()),
        (transfer_request_block_type::ENABLE_SLOT, 0, 0, 0)
    );
    assert_eq!(
        decode(TransferRequestBlock::disable_slot(7)),
        (transfer_request_block_type::DISABLE_SLOT, 7, 0, 0)
    );
    assert_eq!(
        decode(TransferRequestBlock::address_device(0x5000, 1)),
        (
            transfer_request_block_type::DEVICE_ADDRESSING_COMMAND,
            1,
            0,
            0x5000
        )
    );
    assert_eq!(
        decode(TransferRequestBlock::configure_endpoint(0x5000, 2)),
        (
            transfer_request_block_type::CONFIGURE_ENDPOINT,
            2,
            0,
            0x5000
        )
    );
    assert_eq!(
        decode(TransferRequestBlock::evaluate_context(0x5000, 3)),
        (transfer_request_block_type::EVALUATE_CONTEXT, 3, 0, 0x5000)
    );
    assert_eq!(
        decode(TransferRequestBlock::reset_endpoint(1, 3)),
        (transfer_request_block_type::RESET_ENDPOINT, 1, 3, 0)
    );
    assert_eq!(
        decode(TransferRequestBlock::set_dequeue(0x7001, 1, 3)),
        (
            transfer_request_block_type::SET_TRANSFER_RING_DEQUEUE_POINTER,
            1,
            3,
            0x7001
        )
    );
    // An event type this driver does not act on decodes as itself, never as a known one.
    assert_eq!(
        Event::decode(TransferRequestBlock([0, 0, 0, 37 << 10])),
        Event::Other { kind: 37 }
    );
    let ring = EventRing::new(0x2000, 16);
    assert_eq!(ring.index(), 0);
}

/// **The smallest producer ring, three entries, links on every second enqueue.**
#[test]
fn a_producer_ring_of_three_links_every_second_time() {
    let mut ring = ProducerRing::new(0x9000, 3);
    assert_eq!(ring.enqueue(TransferRequestBlock::enable_slot()).link, None);
    let p = ring.enqueue(TransferRequestBlock::enable_slot());
    assert_eq!(p.index, 1);
    let (at, link) = p.link.unwrap();
    assert_eq!(at, 2);
    assert_eq!(link.kind(), transfer_request_block_type::LINK);
    assert_eq!(link.0[0], 0x9000);
    assert_eq!(ring.dequeue_pointer(), 0x9000); // back at 0, cycle now 0
}

/// **A producer ring holds two fewer TRBs than its entries**: one entry is the link, and one stays
/// empty so that a full ring and an empty one never share an enqueue position (xHCI 1.2 section
/// 4.9.2.2).
#[test]
fn a_producer_ring_holds_two_fewer_than_its_entries() {
    assert_eq!(ProducerRing::new(0x1000, 3).capacity(), 1);
    assert_eq!(ProducerRing::new(0x1000, 4).capacity(), 2);
    assert_eq!(ProducerRing::new(0x1000, 4096).capacity(), 4094);
}

/// **A producer ring of two entries is refused**: its capacity would be zero.
#[test]
#[should_panic(expected = "a producer ring holds 3 to 4096 entries")]
fn a_producer_ring_of_two_is_refused() {
    ProducerRing::new(0x1000, 2);
}

/// **A producer ring larger than one 64 KiB segment is refused** (xHCI 1.2 Table 6-1).
#[test]
#[should_panic(expected = "a producer ring holds 3 to 4096 entries")]
fn a_producer_ring_of_4097_is_refused() {
    ProducerRing::new(0x1000, 4097);
}

/// **An event ring segment under 16 entries is refused** (xHCI 1.2 Table 6-95).
#[test]
#[should_panic(expected = "an event ring segment holds 16 to 4096 entries")]
fn an_event_ring_of_15_is_refused() {
    EventRing::new(0x2000, 15);
}

/// **An event ring segment over 4096 entries is refused** (xHCI 1.2 Table 6-95).
#[test]
#[should_panic(expected = "an event ring segment holds 16 to 4096 entries")]
fn an_event_ring_of_4097_is_refused() {
    EventRing::new(0x2000, 4097);
}

/// **A buffer fits one transfer TRB exactly when it stays inside one 64 KiB block**, ending at the
/// boundary included (xHCI 1.2 section 6.4.1).
#[test]
fn a_buffer_fits_one_transfer_request_block_until_it_crosses_64_kibibytes() {
    assert!(fits_one_transfer_request_block(0x1_0000, u16::MAX));
    assert!(fits_one_transfer_request_block(0x1_0001, u16::MAX));
    assert!(!fits_one_transfer_request_block(0x1_0002, u16::MAX));
    assert!(fits_one_transfer_request_block(0x1_fff8, 8));
    assert!(!fits_one_transfer_request_block(0x1_fff8, 9));
    assert!(fits_one_transfer_request_block(0x1_ffff, 0));
    assert!(fits_one_transfer_request_block(u64::MAX, 1));
    assert!(!fits_one_transfer_request_block(u64::MAX, 2));
}

/// **Each endpoint type writes its own code** (xHCI 1.2 Table 6-9) in bits 3 to 5, and bit 6,
/// Host Initiate Disable and Max Burst Size read zero.
#[test]
fn each_endpoint_type_writes_its_own_code() {
    use context::EndpointType::*;
    let kinds = [
        IsochronousOut,
        BulkOut,
        InterruptOut,
        Control,
        IsochronousIn,
        BulkIn,
        InterruptIn,
    ];
    for (code, kind) in (1..).zip(kinds) {
        let endpoint = context::endpoint(kind, 8, 0, 0x1000, 8);
        assert_eq!((endpoint[1] >> 3) & 0x7, code);
        assert_eq!((endpoint[1] >> 6) & 0x3ff, 0);
    }
}

/// **A device context index above 31 stays in the five-bit Endpoint ID** of Reset Endpoint and
/// Set TR Dequeue Pointer (xHCI 1.2 sections 6.4.3.7 and 6.4.3.9): bits 21 to 23 read zero.
#[test]
fn an_endpoint_index_above_thirty_one_stays_in_its_field() {
    for built in [
        TransferRequestBlock::reset_endpoint(1, 0xff),
        TransferRequestBlock::set_dequeue(0x1000, 1, 0xff),
    ] {
        assert_eq!(built.0[3] >> 16 & 0xff, 0x1f);
    }
}

/// **Every completion code has its specification name** (xHCI 1.2 Table 6-90): the 36 defined
/// codes in order, code 30 and 37 to 191 reserved, and the two vendor ranges by name.
#[test]
fn every_completion_code_has_its_specification_name() {
    let table = [
        "Invalid",
        "Success",
        "Data Buffer Error",
        "Babble Detected Error",
        "USB Transaction Error",
        "TRB Error",
        "Stall Error",
        "Resource Error",
        "Bandwidth Error",
        "No Slots Available Error",
        "Invalid Stream Type Error",
        "Slot Not Enabled Error",
        "Endpoint Not Enabled Error",
        "Short Packet",
        "Ring Underrun",
        "Ring Overrun",
        "VF Event Ring Full Error",
        "Parameter Error",
        "Bandwidth Overrun Error",
        "Context State Error",
        "No Ping Response Error",
        "Event Ring Full Error",
        "Incompatible Device Error",
        "Missed Service Error",
        "Command Ring Stopped",
        "Command Aborted",
        "Stopped",
        "Stopped - Length Invalid",
        "Stopped - Short Packet",
        "Max Exit Latency Too Large Error",
        "Reserved",
        "Isoch Buffer Overrun",
        "Event Lost Error",
        "Undefined Error",
        "Invalid Stream ID Error",
        "Secondary Bandwidth Error",
        "Split Transaction Error",
        "Reserved",
    ];
    for (code, expected) in (0..).zip(table) {
        assert_eq!(completion::name(code), expected, "code {code}");
    }
    assert_eq!(completion::name(completion::STALL), "Stall Error");
    assert_eq!(
        completion::name(completion::SPLIT_TRANSACTION_ERROR),
        "Split Transaction Error"
    );
    for code in 37..=191 {
        assert_eq!(completion::name(code), "Reserved");
    }
    for code in 192..=223 {
        assert_eq!(completion::name(code), "Vendor Defined Error");
    }
    for code in 224..=255 {
        assert_eq!(completion::name(code), "Vendor Defined Info");
    }
}

/// **`try_enqueue` refuses exactly at capacity, and `complete` frees what the controller
/// consumed**: a four-entry ring takes two TRBs, refuses a third, and takes one more for each one
/// reported, across the wrap.
#[test]
fn try_enqueue_refuses_at_capacity_until_a_completion_frees_an_entry() {
    let mut ring = ProducerRing::new(0x1000, 4);
    let a = ring
        .try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    let b = ring
        .try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    assert_eq!((a.index, b.index), (0, 1));
    assert_eq!(ring.try_enqueue(TransferRequestBlock::enable_slot()), None);
    assert!(ring.complete(ring.entry_physical_address(a.index)));
    let c = ring
        .try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    assert_eq!(c.index, 2);
    assert!(c.link.is_some());
    assert_eq!(ring.try_enqueue(TransferRequestBlock::enable_slot()), None);
    // One event for the newest TRB frees both before it.
    assert!(ring.complete(ring.entry_physical_address(c.index)));
    let d = ring
        .try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    let e = ring
        .try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    assert_eq!((d.index, e.index), (0, 1));
    assert!(!d.transfer_request_block.is_cycle());
    assert_eq!(ring.try_enqueue(TransferRequestBlock::enable_slot()), None);
}

/// **`complete` refuses an address that is not an outstanding TRB of the ring**, and changes
/// nothing: one below the ring, one not on a 16-byte boundary, the link entry, one past the
/// ring, and one already reported.
#[test]
fn complete_refuses_what_is_not_outstanding() {
    let mut ring = ProducerRing::new(0x1000, 4);
    let a = ring
        .try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    ring.try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    let before = ring;
    for physical_address in [0xff0, 0x1008, 0x1030, 0x1040, 0x1020] {
        assert!(!ring.complete(physical_address), "{physical_address:#x}");
        assert_eq!(ring, before);
    }
    assert!(ring.complete(ring.entry_physical_address(a.index)));
    assert!(!ring.complete(ring.entry_physical_address(a.index)));
    assert!(ring.complete(0x1010));
    assert!(!ring.complete(0x1010));
    assert!(!ring.complete(0x1000));
}

/// **`complete` refuses the link entry once the ring has wrapped, and frees exactly the TRBs up
/// to the one it is given**: in a four-entry ring with the first two TRBs reported and the next
/// two outstanding across the wrap, the link's address is still refused; in an eight-entry ring,
/// reporting the second of three TRBs leaves one outstanding, so five more fit and a sixth does
/// not.
#[test]
fn complete_counts_by_entry_index_across_the_wrap() {
    let mut ring = ProducerRing::new(0x1000, 4);
    for _ in 0..2 {
        ring.try_enqueue(TransferRequestBlock::enable_slot())
            .unwrap();
    }
    assert!(ring.complete(0x1010));
    ring.try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    ring.try_enqueue(TransferRequestBlock::enable_slot())
        .unwrap();
    let before = ring;
    assert!(!ring.complete(0x1030));
    assert_eq!(ring, before);

    let mut ring = ProducerRing::new(0x1000, 8);
    for _ in 0..3 {
        ring.try_enqueue(TransferRequestBlock::enable_slot())
            .unwrap();
    }
    assert!(ring.complete(0x1010));
    for _ in 0..5 {
        ring.try_enqueue(TransferRequestBlock::enable_slot())
            .unwrap();
    }
    assert_eq!(ring.try_enqueue(TransferRequestBlock::enable_slot()), None);
}
