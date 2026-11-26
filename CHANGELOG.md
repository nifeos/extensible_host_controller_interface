# Changelog

Dates are UTC. One line per release, what changed and why, so a reader of the diff has the
sentence beside it.

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
