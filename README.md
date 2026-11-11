# extensible_host_controller_interface

[![check](https://github.com/nifeos/extensible_host_controller_interface/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/nifeos/extensible_host_controller_interface/actions/workflows/ci.yml?query=branch%3Amain)
[![proofs](https://github.com/nifeos/extensible_host_controller_interface/actions/workflows/proofs.yml/badge.svg?branch=main)](https://github.com/nifeos/extensible_host_controller_interface/actions/workflows/proofs.yml?query=branch%3Amain)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/nifeos/extensible_host_controller_interface#license)
[![minimum Rust version](https://img.shields.io/badge/rust-1.88%2B-blue.svg)](https://github.com/nifeos/extensible_host_controller_interface/blob/main/Cargo.toml)

Everything a Universal Serial Bus (USB) host controller (xHCI) driver computes, with nothing it
touches. That covers register offsets and field decode, Transfer Request Block (TRB) construction
and decode, the producer and event ring cycle discipline, slot and endpoint contexts, port status
and the port debounce. The caller performs every volatile access and owns the direct memory access
(DMA) memory. This crate supplies the arithmetic, so it runs in `no_std`, tests on the host and
suits a model checker. It does not touch hardware, allocate, or drive a controller on its own.

## EXAMPLES

```rust
use extensible_host_controller_interface::{Capabilities, ProducerRing, TransferRequestBlock};

// QEMU's qemu-xhci: 0x40 bytes of capability registers, 64 slots, 16 interrupters and 8 ports
// (4 USB 2 and 4 USB 3, the defaults), runtime at 0x1000, doorbells at 0x2000. Port 1's status
// register and the command ring's doorbell:
let caps = Capabilities::decode(0x0100_0040, 0x0800_1040, 0, 0x0000_7000, 0x2000, 0x1000);
assert_eq!((caps.port_status_and_control(1), caps.doorbell(0)), (Some(0x440), 0x2000));

// A four-entry command ring: three usable slots, then a link TRB back to the first.
let mut ring = ProducerRing::new(0x1000, 4);
let a = ring.enqueue(TransferRequestBlock::enable_slot());
assert_eq!((a.index, a.transfer_request_block.is_cycle(), a.link), (0, true, None));
```

The caller writes each placed TRB (and the link, when `link` is `Some`) to DMA memory at
`ring.entry_physical_address(index)`, then rings the doorbell.

## What is proven

Thirteen [Kani](https://github.com/model-checking/kani) harnesses, in `src/verification.rs`:

- `a_producer_ring_stays_in_bounds`: from every reachable state, a producer ring never places a TRB
  on its link slot or past its end, and places the link exactly on the last slot.
- `the_event_ring_stays_in_bounds_and_flips_only_at_the_wrap`: from every reachable state, consuming
  an event leaves the dequeue index inside the segment, flips the cycle bit exactly when the index
  wraps to zero, and keeps the `ERDP` address inside the segment. The proof assumes the segment does not
  run past the end of the 64-bit address space.
- `the_capability_walk_ends_and_stays_inside_the_limit`: for any start, limit and id, and any dword
  the controller returns at each read, `find_extended` makes at most 64 reads. Each read is at a nonzero
  offset whose whole dword is inside `limit` and past the previous read. The result is only an offset
  it read whose id matched.
- `a_device_context_index_is_one_to_thirty_one_and_distinct`: for every endpoint address, the device
  context index is from 1 to 31, endpoint 0 is index 1 in both directions, two different endpoints
  never share an index. The offsets of every index lie inside a 32- or 64-byte-per-context
  device or input context.
- `a_slot_context_keeps_each_field_in_its_own_bits`: for every input, a slot context reads back
  each field from exactly the bits the specification gives it, and the reserved dwords stay zero.
- `an_endpoint_context_keeps_each_field_in_its_own_bits`: for every input, an endpoint context
  reads back each field from exactly the bits the specification gives it. Every endpoint type reads
  back as its own code, the dequeue pointer reassembles from its two dwords, and the reserved bits
  and dwords stay zero.
- `a_port_write_never_acknowledges_what_nobody_asked_to`: for any port status register value,
  `write_value` writes the preserved bits plus exactly the bits asked for, so writing nothing writes
  no change bit, no Port Enabled bit and no reset. The speed id is the four bits it comes from.
- `the_interrupt_interval_is_the_exponent_the_specification_asks_for`: for any speed and
  `bInterval`, a full- or low-speed interval is the largest exponent within the device's period,
  between 3 and 10, and any other is the device's exponent less one. The default control packet exists for
  speeds 1 to 5 and no others.
- `every_event_decodes_from_the_bits_the_specification_gives`: for any four dwords, an event
  decodes to the variant its type names, each field from exactly its own bits, and the cycle bit
  never changes what it says.
- `every_command_trb_puts_its_slot_endpoint_and_pointer_in_their_fields`: for every input, each
  command and data TRB holds its slot, endpoint and pointer where the specification puts them,
  and nothing outside its fields.
- `the_capability_registers_decode_each_field_from_its_own_bits`: for any six capability
  registers, each decoded field comes from exactly its own bits at its own width.
- `the_port_debounce_answers_by_its_timeout`: over four samples at times that do not run
  backwards, the debounce never keeps reading at or past its timeout, and says settled exactly
  when nothing changed for the stable interval.
- `a_producer_and_a_consumer_agree_across_the_wrap`: a modeled controller follows the cycle bits
  and the link of a ring of 3 to 5 entries, through any ten steps within the ring's capacity. It
  sees a TRB exactly when one is outstanding, in the order the producer placed them.

Install Kani and run them as "Building and testing" below describes.

## What the caller does

The crate computes; the caller performs every access, in this order. On x86_64 a compiler fence
is enough for each barrier below. On aarch64 and riscv64, which order memory weakly, each one is
a barrier instruction (Linux spells them `wmb()` and `rmb()`).

To produce a TRB on a command or transfer ring:

1. Keep at most `ring.capacity()` TRBs outstanding: placed, with no completion event yet. The
   ring does not know how far the controller has read, so it never refuses.
2. Call `ring.enqueue`, then write dwords 0 to 2 of the placed TRB at
   `ring.entry_physical_address(index)` with volatile writes.
3. Issue a write barrier, then write dword 3, which holds the cycle bit. Until that bit matches,
   the controller treats the TRB as not yet produced (xHCI 1.2 section 4.9.2).
4. When `link` is `Some`, write the link TRB at its index the same way.
5. Issue a write barrier, then write the doorbell register.

To consume an event:

1. Read dword 3 of the entry at `event_ring.index()` with a volatile read. The controller writes
   that dword last (xHCI 1.2 section 4.11.3).
2. When `event_ring.is_ready` says no, stop. Otherwise issue a read barrier, then read dwords 0 to
   2, and decode the four with `Event::decode`.
3. Call `event_ring.advance()`, and after the events of this pass, write
   `event_ring.dequeue_pointer()` to the interrupter's `ERDP` register (xHCI 1.2 section 4.9.4).

## Building and testing

```sh
cargo build
cargo test
rustup target add x86_64-unknown-none aarch64-unknown-none-softfloat riscv64imac-unknown-none-elf
cargo build --lib --target x86_64-unknown-none   # also aarch64-unknown-none-softfloat, riscv64imac-unknown-none-elf
```

Run `rustup target add` once, before the first bare-metal build. Without it, that build stops with
`can't find crate for core`. The tests run on the host in about a second.

### Running the proofs

The proofs need Kani, which `rustup` does not install. Run `cargo install --locked
kani-verifier --version 0.68.0`, then `cargo kani setup`. The setup step downloads a model
checker, so it needs network access. CI uses version 0.68.0, and another version may verify
differently.

Then `cargo kani`, from the crate's directory, runs all thirteen harnesses. On an x86_64 Linux
desktop that takes about 2 minutes and peaks near 0.8 gigabytes of memory.
`the_capability_walk_ends_and_stays_inside_the_limit` takes about half of that time. CI
runs plain `cargo kani`, with no extra flags. To run one harness, give its full path and `--exact`:

```sh
cargo kani --harness verification::a_producer_ring_stays_in_bounds --exact
```

A passing run prints several lines that look alarming. All of them are expected:

- `warning: use of an unstable feature`, about `#![feature(register_tool)]`, which Kani sets.
- `warning: Found the following unsupported constructs: caller_location (1), foreign function (1)`,
  followed by "Verification will fail if one or more of these constructs is reachable". A run that
  ends in `VERIFICATION:- SUCCESSFUL` never reached them.
- Hundreds of `aborting path on assume(false)` lines. They mark paths that a harness's assumptions
  rule out.
- A check described as unreachable code. It reports code that cannot run, and it is no failure.
- Source paths under `/home/runner/.rustup`, a directory that does not exist on your machine. They
  come from the toolchain Kani ships.

Trust the last line of the run: `Complete - 13 successfully verified harnesses, 0 failures, 13 total.`

### Running one falsification patch

```sh
patch -p1 --no-backup-if-mismatch < falsifications/verification.a_producer_ring_stays_in_bounds.patch
cargo kani --harness verification::a_producer_ring_stays_in_bounds --exact   # ends VERIFICATION:- FAILED
git checkout -- src
```

The harness name is the part of the patch's file name after `verification.`. The run ends with
`Complete - 0 successfully verified harnesses, 1 failures, 1 total.`, and the failed check names
the assertion that the patch breaks. `git checkout -- src` reverts the patch.

## BUGS

- Hubs are not modeled (route string 0), so the crate cannot address a device behind a hub. Only devices
  on the root ports work.
- One page size: the DMA layout assumes 4 KiB pages (`PAGESIZE` bit 0).
- No streams, isochronous or bulk transfers: the crate builds the TRBs a boot keyboard needs and
  nothing more.
- A producer ring never refuses a TRB. The caller keeps at most `ProducerRing::capacity` TRBs
  outstanding, or the next lap overwrites one the controller has not reached.
- The TRB builders do not check the 64 KiB rule: a data buffer must not cross a 64 KiB boundary
  (xHCI 1.2 section 6.4.1). `fits_one_transfer_request_block` says whether one does, and the
  caller chains several TRBs for a buffer that crosses one.

The reference is the xHCI 1.2 specification, chapters 4 to 7.

## Minimum Rust version

1.88. To check it, install that toolchain and its bare-metal target, then test and build with it:

```sh
rustup toolchain install 1.88 --profile minimal --target x86_64-unknown-none
cargo +1.88 test
cargo +1.88 build --lib --target x86_64-unknown-none
```

## License

Licensed under either of the MIT license ([LICENSE-MIT](LICENSE-MIT)) or the Apache License,
Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), at your option.
