# Changelog

Dates are UTC. One line per release, what changed and why, so a reader of the diff has the
sentence beside it.

## 0.1.0-pre.2

- Breaking: the nine compile-time overlap checks (the `*_DO_NOT_OVERLAP` constants of `context`,
  `event`, `port`, `registers` and `transfer_request_block`) are crate-private, so they leave the
  public surface. They only ever asserted at compile time that bit fields share no bit.
- The crate forbids `unsafe` code. Every code the crate hands back has a name; ring sizes a
  controller refuses are refused; a producer ring can refuse at capacity (`try_enqueue`).
- The endpoint type is an enum and the minor revision reads from a value that is not 0. Two
  tri-state fields are enums, and the port offset is bounded.
- Interrupters, Supported Protocol capabilities and the driver's waits are named, with bit
  mnemonics as aliases, and examples show a whole transfer.
- Event decoding, the builders, capability decode, the debounce and the ring wrap have proofs.

## 0.1.0-pre.1

- First version.

## BUGS

- The template's lag check guards the CI pin, not this file; a change that matters to a user of
  the crate belongs in a release line here.
