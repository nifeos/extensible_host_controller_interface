use super::*;
use crate::registers::find_extended;

/// Falsification: replayable `falsifications/verification.a_producer_ring_stays_in_bounds.patch`
/// **A producer ring never places a TRB on its link slot or past the end**, and places the link
/// exactly on the last slot, from every reachable state.
#[kani::proof]
fn a_producer_ring_stays_in_bounds() {
    let entries: u16 = kani::any();
    let enqueue: u16 = kani::any();
    kani::assume(entries >= 2 && enqueue < entries - 1);
    let mut ring = ProducerRing {
        base: kani::any(),
        entries,
        enqueue,
        cycle: kani::any(),
        completed: kani::any(),
    };
    let p = ring.enqueue(TransferRequestBlock(kani::any()));
    assert!(p.index < entries - 1);
    if let Some((at, link)) = p.link {
        assert_eq!(at, entries - 1);
        assert_eq!(link.kind(), transfer_request_block_type::LINK);
    }
    assert!(ring.enqueue < entries - 1);
}

/// Falsification: replayable `falsifications/verification.the_event_ring_stays_in_bounds_and_flips_only_at_the_wrap.patch`
/// **The event ring's dequeue index stays inside the segment, and its cycle flips exactly when the
/// index wraps**, from every reachable state. The physical address handed to `ERDP` stays inside
/// the segment too, for a segment that does not run past the end of the address space.
#[kani::proof]
fn the_event_ring_stays_in_bounds_and_flips_only_at_the_wrap() {
    let entries: u16 = kani::any();
    let dequeue: u16 = kani::any();
    kani::assume(entries > 0 && dequeue < entries);
    let base: u64 = kani::any();
    // The segment is a physical range that exists, so its end does not wrap the address space.
    kani::assume(base.checked_add(u64::from(entries) * 16).is_some());
    let cycle: bool = kani::any();
    let mut ring = EventRing {
        base,
        entries,
        dequeue,
        cycle,
    };
    ring.advance();
    assert!(ring.dequeue < entries);
    assert_eq!(ring.cycle != cycle, dequeue == entries - 1);
    assert_eq!(ring.dequeue == 0, dequeue == entries - 1);
    let event_ring_dequeue_pointer = ring.dequeue_pointer();
    assert!(
        event_ring_dequeue_pointer >= base
            && event_ring_dequeue_pointer < base + u64::from(entries) * 16
    );
}

/// Falsification: replayable `falsifications/verification.the_capability_walk_ends_and_stays_inside_the_limit.patch`
/// **The extended-capability walk ends within 64 reads, reads only inside `limit`, moves forward on
/// every hop, and returns only an entry it read whose id matched**, whatever dword the controller
/// returns at each offset. The register file is modeled as arbitrary per read, which includes every
/// fixed register file.
#[kani::proof]
#[kani::unwind(66)]
fn the_capability_walk_ends_and_stays_inside_the_limit() {
    let first: u64 = kani::any();
    let limit: u64 = kani::any();
    let id: u8 = kani::any();
    let mut reads = 0u32;
    let mut last: Option<(u64, u32)> = None;
    let found = {
        let mut read = |at: u64| {
            reads += 1;
            assert!(reads <= 64);
            assert!(at != 0 && at.checked_add(4).is_some_and(|end| end <= limit));
            if let Some((before, _)) = last {
                assert!(at > before);
            }
            let head: u32 = kani::any();
            last = Some((at, head));
            head
        };
        find_extended(first, limit, id, &mut read)
    };
    if let Some(at) = found {
        let (read_at, head) = last.unwrap();
        assert_eq!(at, read_at);
        assert_eq!(head as u8, id);
    }
}

/// Falsification: replayable `falsifications/verification.a_device_context_index_is_one_to_thirty_one_and_distinct.patch`
/// **The device context index of any endpoint address is a real index, and its offsets stay in the
/// context.** Endpoint 0 is index 1 in either direction; every other endpoint number and
/// direction has its own index from 2 to 31; and for either context size the xHCI allows (32 or 64
/// bytes) a device context holds the offset of every index and an input context one more.
#[kani::proof]
fn a_device_context_index_is_one_to_thirty_one_and_distinct() {
    let address: u8 = kani::any();
    let other: u8 = kani::any();
    let index = context::endpoint_index(address);
    assert!((1..=31).contains(&index));
    if address & 0x0f == 0 {
        assert_eq!(index, 1);
    }
    let same_endpoint = address & 0x8f == other & 0x8f;
    if address & 0x0f != 0 && other & 0x0f != 0 && !same_endpoint {
        assert_ne!(index, context::endpoint_index(other));
    }
    let context_bytes: u64 = if kani::any() { 32 } else { 64 };
    assert!(context::device_offset(index, context_bytes) + context_bytes <= 32 * context_bytes);
    assert!(context::input_offset(index + 1, context_bytes) + context_bytes <= 33 * context_bytes);
}

/// Falsification: replayable `falsifications/verification.a_slot_context_keeps_each_field_in_its_own_bits.patch`
/// **A slot context holds each field in its own bits, for every input.** What goes in comes back
/// out of exactly the field the specification gives it (xHCI 1.2 section 6.2.2), and no field
/// spills into a neighbor: the speed and the context count are masked to their widths, and the
/// reserved dwords stay zero.
#[kani::proof]
fn a_slot_context_keeps_each_field_in_its_own_bits() {
    let speed: u8 = kani::any();
    let port: u8 = kani::any();
    let entries: u8 = kani::any();
    let slot = context::slot(speed, port, entries);
    assert_eq!((slot[0] >> 20) & 0xf, speed as u32 & 0xf);
    assert_eq!(slot[0] >> 27, entries as u32 & 0x1f);
    assert_eq!(slot[0] & !((0xf << 20) | (0x1f << 27)), 0);
    assert_eq!(slot[1], (port as u32) << 16);
    assert_eq!(slot[2..], [0; 6]);
}

/// Falsification: replayable `falsifications/verification.an_endpoint_context_keeps_each_field_in_its_own_bits.patch`
/// **An endpoint context holds each field in its own bits, for every input.** What goes in comes
/// back out of exactly the field the specification gives it (xHCI 1.2 section 6.2.3, Table 6-9
/// for dword 1), and no field spills into a neighbor: the endpoint type is masked to bits 3 to 5,
/// so the reserved bit 6, Host Initiate Disable at bit 7 and Max Burst Size at bits 8 to 15 stay
/// zero; the dequeue pointer is reassembled from its two dwords, and the reserved dwords stay
/// zero.
#[kani::proof]
fn an_endpoint_context_keeps_each_field_in_its_own_bits() {
    let code: u8 = kani::any();
    let kind = match code {
        1 => context::EndpointType::IsochronousOut,
        2 => context::EndpointType::BulkOut,
        3 => context::EndpointType::InterruptOut,
        4 => context::EndpointType::Control,
        5 => context::EndpointType::IsochronousIn,
        6 => context::EndpointType::BulkIn,
        _ => context::EndpointType::InterruptIn,
    };
    let maximum_packet: u16 = kani::any();
    let interval: u8 = kani::any();
    let dequeue: u64 = kani::any();
    let average_trb: u16 = kani::any();
    let endpoint = context::endpoint(kind, maximum_packet, interval, dequeue, average_trb);
    assert_eq!(endpoint[0], (interval as u32) << 16);
    assert_eq!(endpoint[1] & 1, 0);
    assert_eq!((endpoint[1] >> 1) & 0x3, 3);
    assert_eq!((endpoint[1] >> 3) & 0x7, kind as u32);
    assert_eq!((endpoint[1] >> 6) & 0x3ff, 0);
    assert_eq!(endpoint[1] >> 16, maximum_packet as u32);
    assert_eq!(endpoint[2] as u64 | (endpoint[3] as u64) << 32, dequeue);
    assert_eq!(endpoint[4] & 0xffff, average_trb as u32);
    let maximum_service_interval_payload = if kind == context::EndpointType::Control {
        0
    } else {
        maximum_packet as u32
    };
    assert_eq!(endpoint[4] >> 16, maximum_service_interval_payload);
    assert_eq!(endpoint[5..], [0; 3]);
}

/// Falsification: replayable `falsifications/verification.a_port_write_never_acknowledges_what_nobody_asked_to.patch`
/// **A write to the port status register never writes back a change bit or the Port Enabled bit
/// the caller did not ask for, and the speed id is the four bits it comes from.** Whatever the
/// register read, the written value is the preserved bits of it plus exactly the bits asked for,
/// and writing nothing writes no write-one-to-clear bit.
#[kani::proof]
fn a_port_write_never_acknowledges_what_nobody_asked_to() {
    let port_status_and_control: u32 = kani::any();
    let bits: u32 = kani::any();
    let written = port::write_value(port_status_and_control, bits);
    assert_eq!(
        written & !bits,
        port_status_and_control & port::PRESERVE & !bits
    );
    assert_eq!(written & bits, bits);
    let nothing = port::write_value(port_status_and_control, 0);
    assert_eq!(nothing & port::CHANGES, 0);
    assert_eq!(nothing & port::ENABLED, 0);
    assert_eq!(nothing & port::RESET, 0);
    assert!(port::speed_of(port_status_and_control) <= 0xf);
    assert_eq!(
        port::speed_of(port_status_and_control) as u32,
        (port_status_and_control >> 10) & 0xf
    );
}

/// Falsification: replayable `falsifications/verification.the_interrupt_interval_is_the_exponent_the_specification_asks_for.patch`
/// **The interrupt interval is the exponent the specification asks for, and the default control
/// packet is known exactly for the five speeds that exist.** For a full- or low-speed device it is
/// the largest exponent whose period (`2^n` microframes) does not exceed the device's
/// millisecond period, between 3 and 10; for the others it is the device's exponent less one,
/// at most 15. `default_control_packet` answers for speeds 1 to 5 and for no other id.
#[kani::proof]
fn the_interrupt_interval_is_the_exponent_the_specification_asks_for() {
    let speed: u8 = kani::any();
    let b_interval: u8 = kani::any();
    let interval = context::interrupt_interval(speed, b_interval);
    if speed == port::speed::FULL || speed == port::speed::LOW {
        let microframes = (if b_interval == 0 { 1 } else { b_interval }) as u32 * 8;
        assert!((3..=10).contains(&interval));
        assert!(1u32 << interval <= microframes);
        assert!((1u32 << (interval + 1)) > microframes);
    } else {
        assert!(interval <= 15);
        let wanted = if b_interval == 0 {
            1
        } else {
            b_interval.min(16)
        };
        assert_eq!(interval + 1, wanted);
    }
    assert_eq!(
        port::default_control_packet(speed).is_some(),
        (1..=5).contains(&speed)
    );
}

/// Falsification: replayable `falsifications/verification.every_event_decodes_from_the_bits_the_specification_gives.patch`
/// **Every event TRB decodes to the variant its type names, each field from exactly its own
/// bits**, for any four dwords the controller writes (xHCI 1.2 sections 6.4.2.1 to 6.4.2.3). The
/// Transfer Event's pointer reassembles from dwords 0 and 1, its residual is the 24-bit length,
/// its completion code is the top byte of dword 2, its endpoint is the five-bit Endpoint ID and its
/// slot is the top byte of dword 3. The cycle bit never changes what an event says.
#[kani::proof]
fn every_event_decodes_from_the_bits_the_specification_gives() {
    let words: [u32; 4] = kani::any();
    let event = Event::decode(TransferRequestBlock(words));
    let pointer = words[0] as u64 | (words[1] as u64) << 32;
    let kind = ((words[3] >> 10) & 0x3f) as u8;
    match event {
        Event::Transfer {
            transfer_request_block,
            residual,
            code,
            slot,
            endpoint,
        } => {
            assert_eq!(kind, transfer_request_block_type::TRANSFER_EVENT);
            assert_eq!(transfer_request_block, pointer);
            assert_eq!(residual, words[2] & 0x00ff_ffff);
            assert_eq!(code as u32, words[2] >> 24);
            assert_eq!(endpoint as u32, (words[3] >> 16) & 0x1f);
            assert_eq!(slot as u32, words[3] >> 24);
        }
        Event::Command {
            transfer_request_block,
            code,
            slot,
        } => {
            assert_eq!(kind, transfer_request_block_type::COMMAND_COMPLETION);
            assert_eq!(transfer_request_block, pointer);
            assert_eq!(code as u32, words[2] >> 24);
            assert_eq!(slot as u32, words[3] >> 24);
        }
        Event::PortStatus { port } => {
            assert_eq!(kind, transfer_request_block_type::PORT_STATUS_CHANGE);
            assert_eq!(port as u32, words[0] >> 24);
        }
        Event::Other { kind: other } => {
            assert_eq!(other, kind);
            assert!(
                kind != transfer_request_block_type::TRANSFER_EVENT
                    && kind != transfer_request_block_type::COMMAND_COMPLETION
                    && kind != transfer_request_block_type::PORT_STATUS_CHANGE
            );
        }
    }
    let mut flipped = words;
    flipped[3] ^= 1;
    assert_eq!(Event::decode(TransferRequestBlock(flipped)), event);
}

/// Falsification: replayable `falsifications/verification.every_command_trb_puts_its_slot_endpoint_and_pointer_in_their_fields.patch`
/// **Every command and data TRB the crate builds holds its slot, endpoint and pointer in the
/// fields the specification gives them, and nothing else outside its type**, for every input (xHCI
/// 1.2 sections 6.4.1 and 6.4.3). The slot is bits 24 to 31 of dword 3, the endpoint is the
/// five-bit Endpoint ID at bits 16 to 20, the pointer is dwords 0 and 1, the type reads back, and
/// the cycle bit is clear until the ring stamps it.
#[kani::proof]
fn every_command_trb_puts_its_slot_endpoint_and_pointer_in_their_fields() {
    let pointer: u64 = kani::any();
    let slot: u8 = kani::any();
    let index: u8 = kani::any();
    let len: u16 = kani::any();
    let inbound: bool = kani::any();
    let type_bits = 0x3f << 10;
    let slot_bits = 0xff << 24;
    let endpoint_bits = 0x1f << 16;
    let slot_field = (slot as u32) << 24;
    let endpoint_field = (index as u32 & 0x1f) << 16;
    let lo = pointer as u32;
    let hi = (pointer >> 32) as u32;

    let with_pointer = [
        (
            TransferRequestBlock::address_device(pointer, slot),
            transfer_request_block_type::DEVICE_ADDRESSING_COMMAND,
        ),
        (
            TransferRequestBlock::configure_endpoint(pointer, slot),
            transfer_request_block_type::CONFIGURE_ENDPOINT,
        ),
        (
            TransferRequestBlock::evaluate_context(pointer, slot),
            transfer_request_block_type::EVALUATE_CONTEXT,
        ),
    ];
    for (built, kind) in with_pointer {
        assert_eq!(built.kind(), kind);
        assert_eq!(built.0, [lo, hi, 0, (kind as u32) << 10 | slot_field]);
    }

    let set = TransferRequestBlock::set_dequeue(pointer, slot, index);
    assert_eq!(
        set.kind(),
        transfer_request_block_type::SET_TRANSFER_RING_DEQUEUE_POINTER
    );
    assert_eq!(set.0[..3], [lo, hi, 0]);
    assert_eq!(set.0[3] & !(type_bits | slot_bits | endpoint_bits), 0);
    assert_eq!(
        set.0[3] & (slot_bits | endpoint_bits),
        slot_field | endpoint_field
    );

    let reset = TransferRequestBlock::reset_endpoint(slot, index);
    assert_eq!(reset.kind(), transfer_request_block_type::RESET_ENDPOINT);
    assert_eq!(reset.0[..3], [0, 0, 0]);
    assert_eq!(reset.0[3] & !(type_bits | slot_bits | endpoint_bits), 0);
    assert_eq!(
        reset.0[3] & (slot_bits | endpoint_bits),
        slot_field | endpoint_field
    );

    let disable = TransferRequestBlock::disable_slot(slot);
    assert_eq!(disable.kind(), transfer_request_block_type::DISABLE_SLOT);
    assert_eq!(
        disable.0,
        [0, 0, 0, (disable.kind() as u32) << 10 | slot_field]
    );

    for built in [
        TransferRequestBlock::data(pointer, len, inbound),
        TransferRequestBlock::normal(pointer, len),
    ] {
        assert_eq!(built.0[..3], [lo, hi, len as u32]);
        assert!(!built.is_cycle());
    }
    let data = TransferRequestBlock::data(pointer, len, inbound);
    assert_eq!(data.kind(), transfer_request_block_type::DATA);
    assert_eq!(data.0[3] >> 16 & 1 != 0, inbound);
    assert_eq!(
        TransferRequestBlock::normal(pointer, len).kind(),
        transfer_request_block_type::NORMAL
    );
}

/// Falsification: replayable `falsifications/verification.the_capability_registers_decode_each_field_from_its_own_bits.patch`
/// **The capability registers decode each field from exactly its own bits, at its own width**,
/// for any six dwords (xHCI 1.2 section 5.3). `CAPLENGTH` is a byte, `MaxSlots` a byte,
/// `MaxIntrs` eleven bits, `MaxPorts` a byte, the scratchpad count ten bits from its two halves,
/// the extended capabilities pointer a dword offset, and the doorbell and runtime offsets keep
/// their reserved low bits clear.
#[kani::proof]
fn the_capability_registers_decode_each_field_from_its_own_bits() {
    let length: u32 = kani::any();
    let structural_1: u32 = kani::any();
    let structural_2: u32 = kani::any();
    let parameters_1: u32 = kani::any();
    let doorbell: u32 = kani::any();
    let runtime: u32 = kani::any();
    let capabilities = Capabilities::decode(
        length,
        structural_1,
        structural_2,
        parameters_1,
        doorbell,
        runtime,
    );
    assert_eq!(capabilities.operational, (length & 0xff) as u64);
    assert_eq!(capabilities.version as u32, length >> 16);
    assert_eq!(capabilities.maximum_slots as u32, structural_1 & 0xff);
    assert_eq!(
        capabilities.maximum_interrupters as u32,
        (structural_1 >> 8) & 0x7ff
    );
    assert!(capabilities.maximum_interrupters < 1 << 11);
    assert_eq!(capabilities.maximum_ports as u32, structural_1 >> 24);
    let high = (structural_2 >> 21) & 0x1f;
    let low = structural_2 >> 27;
    assert_eq!(capabilities.scratchpads as u32, high << 5 | low);
    assert!(capabilities.scratchpads < 1 << 10);
    assert_eq!(
        capabilities.context_bytes,
        if parameters_1 & 1 << 2 != 0 { 64 } else { 32 }
    );
    assert_eq!(capabilities.extended, ((parameters_1 >> 16) as u64) * 4);
    assert_eq!(capabilities.extended % 4, 0);
    assert_eq!(capabilities.doorbells, (doorbell as u64) & !0x3);
    assert_eq!(capabilities.doorbells >> 2, (doorbell >> 2) as u64);
    assert_eq!(capabilities.runtime, (runtime as u64) & !0x1f);
    assert_eq!(capabilities.runtime >> 5, (runtime >> 5) as u64);
}

/// Falsification: replayable `falsifications/verification.the_port_debounce_answers_by_its_timeout.patch`
/// **The port debounce answers by its timeout, and says settled exactly when nothing changed for
/// the stable interval**, over four samples at any times that do not run backwards and any
/// connections seen. A sample at or past [`port::DEBOUNCE_TIMEOUT_MILLISECONDS`] from the start is
/// never `Pending`, and a `Settled` comes only after [`port::DEBOUNCE_STABLE_MILLISECONDS`] with
/// the same connections.
#[kani::proof]
// Comparing two 32-byte connection sets is a `memcmp` loop of 32 steps, past the four samples.
#[kani::unwind(33)]
fn the_port_debounce_answers_by_its_timeout() {
    let start: u64 = kani::any();
    let first: port::Connections = kani::any();
    let mut debounce = port::PortDebounce::new(start, first);
    let mut now = start;
    let mut last = first;
    let mut changed = start;
    for _ in 0..4 {
        let step: u64 = kani::any();
        kani::assume(step <= 4 * port::DEBOUNCE_TIMEOUT_MILLISECONDS);
        now = now.saturating_add(step);
        let seen: port::Connections = kani::any();
        if seen != last {
            last = seen;
            changed = now;
        }
        let answer = debounce.sample(now, seen);
        let settled = now - changed >= port::DEBOUNCE_STABLE_MILLISECONDS;
        let timed_out = now - start >= port::DEBOUNCE_TIMEOUT_MILLISECONDS;
        assert_eq!(answer == port::DebounceVerdict::Settled, settled);
        if timed_out {
            assert_ne!(answer, port::DebounceVerdict::Pending);
        } else {
            assert_ne!(answer, port::DebounceVerdict::TimedOut);
        }
    }
}

/// Falsification: replayable `falsifications/verification.a_producer_and_a_consumer_agree_across_the_wrap.patch`
/// **A producer ring and a controller that follows the cycle bits agree on every TRB, across the
/// wrap**, for rings of 3 to 5 entries and any interleaving of ten steps that keeps the producer
/// within [`ProducerRing::capacity`]. The modeled controller follows link TRBs and flips its
/// cycle state where they say to (xHCI 1.2 sections 4.9.2 and 6.4.4.1). It sees a TRB exactly when
/// one is outstanding, and it sees the TRBs in the order the producer placed them.
#[kani::proof]
#[kani::unwind(11)]
fn a_producer_and_a_consumer_agree_across_the_wrap() {
    const MOST: usize = 5;
    let entries: u16 = kani::any();
    kani::assume((3..=MOST as u16).contains(&entries));
    let base: u64 = 0x1000;
    let mut memory = [TransferRequestBlock([0; 4]); MOST];
    let mut ring = ProducerRing::new(base, entries);
    let (mut dequeue, mut consumer_cycle) = (0usize, true);
    let (mut produced, mut consumed) = (0u32, 0u32);
    for _ in 0..10 {
        let outstanding = produced - consumed;
        if kani::any() && outstanding < u32::from(ring.capacity()) {
            produced += 1;
            let p = ring.enqueue(TransferRequestBlock::normal(u64::from(produced), 8));
            memory[p.index as usize] = p.transfer_request_block;
            if let Some((at, link)) = p.link {
                memory[at as usize] = link;
            }
        } else {
            let mut seen = memory[dequeue];
            if seen.is_cycle() == consumer_cycle && seen.kind() == transfer_request_block_type::LINK
            {
                if seen.0[3] & 1 << 1 != 0 {
                    consumer_cycle = !consumer_cycle;
                }
                assert_eq!(u64::from(seen.0[0]) | u64::from(seen.0[1]) << 32, base);
                dequeue = 0;
                seen = memory[dequeue];
            }
            let ready = seen.is_cycle() == consumer_cycle;
            assert_eq!(ready, outstanding > 0);
            if ready {
                consumed += 1;
                assert_eq!(seen.0[0], consumed);
                dequeue += 1;
            }
        }
    }
}
