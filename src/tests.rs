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
    let mut ring = EventRing::new(0x2000, 2);
    let fresh = TransferRequestBlock([0, 0, 0, 1]);
    assert!(ring.is_ready(&fresh));
    ring.advance();
    assert_eq!(ring.index(), 1);
    assert_eq!(ring.dequeue_pointer(), 0x2010);
    ring.advance();
    assert_eq!(ring.dequeue_pointer(), 0x2000);
    assert!(!ring.is_ready(&fresh));
    assert_eq!(ring.segment_table_entry(), [0x2000, 0, 2, 0]);
    // A ring above 4 GiB: the table entry's second dword is the base's high half.
    let high = EventRing::new(0x3_0000_2000, 4);
    assert_eq!(high.segment_table_entry(), [0x2000, 3, 4, 0]);
}

#[test]
fn a_setup_transfer_request_block_carries_the_packet_and_the_transfer_type() {
    let t = TransferRequestBlock::setup(0x0008_0000_0100_0680, Some(true));
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
    for (data_in, transfer_type, status_in) in [
        (None, 0, true),
        (Some(false), 2, true),
        (Some(true), 3, false),
    ] {
        let setup = TransferRequestBlock::setup(0, data_in);
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
    let ring = EventRing::new(0x2000, 4);
    assert_eq!(ring.index(), 0);
}

#[test]
fn a_producer_ring_of_two_links_every_time() {
    let mut ring = ProducerRing::new(0x9000, 2);
    let p = ring.enqueue(TransferRequestBlock::enable_slot());
    assert_eq!(p.index, 0);
    let (at, link) = p.link.unwrap();
    assert_eq!(at, 1);
    assert_eq!(link.kind(), transfer_request_block_type::LINK);
    assert_eq!(link.0[0], 0x9000);
    assert_eq!(ring.dequeue_pointer(), 0x9000); // back at 0, cycle now 0
}

/// **A producer ring holds two fewer TRBs than its entries**: one entry is the link, and one stays
/// empty so that a full ring and an empty one never share an enqueue position (xHCI 1.2 section
/// 4.9.2.2).
#[test]
fn a_producer_ring_holds_two_fewer_than_its_entries() {
    assert_eq!(ProducerRing::new(0x1000, 2).capacity(), 0);
    assert_eq!(ProducerRing::new(0x1000, 4).capacity(), 2);
    assert_eq!(ProducerRing::new(0x1000, 256).capacity(), 254);
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

/// **An endpoint type code above 7 stays in its three bits** (xHCI 1.2 Table 6-9): bit 6, Host
/// Initiate Disable and Max Burst Size read zero.
#[test]
fn an_endpoint_type_above_seven_stays_in_its_field() {
    let endpoint = context::endpoint(0xff, 8, 0, 0x1000, 8);
    assert_eq!((endpoint[1] >> 3) & 0x7, 7);
    assert_eq!((endpoint[1] >> 6) & 0x3ff, 0);
    assert_eq!(
        context::endpoint(context::INTERRUPT_IN, 8, 0, 0, 8)[1] >> 3 & 0x7,
        7
    );
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
