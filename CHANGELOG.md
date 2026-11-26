# Changelog

Dates are UTC. One line per release, what changed and why, so a reader of the diff has the
sentence beside it.

## Unreleased

Every name below that is new is provisional until the first release.

- The dwords of a Transfer Request Block (TRB) are private: `from_dwords`, `dwords` and
  `interrupt_on_completion` read and build them, and `to_le_bytes` and `from_le_bytes` give the
  form memory holds.
- `ProducerRing::enqueue`, which never refused, is no longer public; `try_enqueue` is the way in.
- The data direction is `Direction` (`In`, `Out`), not a `bool`.
- Slots, device context indices, ports and speeds are `SlotId`, `DeviceContextIndex`,
  `PortNumber` and `PortSpeed`, each refusing its reserved values, so the compiler tells them
  apart. The `port::speed` constants are `PortSpeed`'s associated constants.
- `doorbell` answers `None` past the last slot, as every other register offset does.
- `Event` and its variants, `TransferRequestBlockPlacement`, `DebounceVerdict`, `Capabilities`
  and `SupportedProtocol` are `#[non_exhaustive]`; the two register structures read through
  methods named after their old fields.
- A completion code is `CompletionCode`, with the specification's codes as associated constants
  and `name` as a method; the `completion` module is gone.
- `Capabilities` reads 64-bit addressing (`has_64_bit_addressing`) and the event ring segment
  bound (`maximum_event_ring_segments`).
- The Host Controller Event decodes as `Event::HostController`, with its code, not as `Other`.
- The boot-keyboard direct memory access (DMA) layout and `PAGE_BYTES` leave the crate for
  `examples/boot_keyboard_layout.rs`: the crate fixes no layout.
- Every public type derives `Hash`; the placement, `try_enqueue`, `complete` and the builders are
  `#[must_use]`.
- The documentation speaks of the caller, not of one driver and a kernel, the main entry points
  carry examples, the TRB builders say they carry at most 65,535 bytes, and the manifest names
  its documentation.

## 0.1.0-pre.4

- The crate's code does not change. The README names the checks that have not run on a controller in
  silicon, and the stranger-test record is current.

## 0.1.0-pre.3

- No `Unreleased` section existed, so these lines come from the commits since 0.1.0-pre.2.
- The `endpoint` builder masks the endpoint type to its three bits again, and its falsification
  patch drops the mask. Every code fits the field today, so no value changes.
- Each module says when a driver reaches for it, and the two ring proofs cover only the sizes
  the constructors accept.
- The README gives the Kani install command on one line a reader can paste.

## 0.1.0-pre.2

- Breaking: the nine compile-time overlap checks (the `*_DO_NOT_OVERLAP` constants of `context`,
  `event`, `port`, `registers` and `transfer_request_block`) are crate-private, so they leave the
  public surface. They only ever asserted at compile time that bit fields share no bit.
- The crate forbids `unsafe` code. The crate names every code it hands back, refuses the ring
  sizes a controller refuses, and lets a producer ring refuse at capacity (`try_enqueue`).
- The endpoint type is an enum and the minor revision reads from a value that is not 0. Two
  tri-state fields are enums, and the port offset has a bound.
- The crate names interrupters, Supported Protocol capabilities and the driver's waits, with bit
  mnemonics as aliases, and examples show a whole transfer.
- Event decoding, the builders, capability decode, the debounce and the ring wrap have proofs.

## 0.1.0-pre.1

- First version.

## BUGS

- The template's lag check guards the CI pin, not this file; a change that matters to a user of
  the crate belongs in a release line here.
