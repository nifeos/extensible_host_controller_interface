# The stranger cadence

The release's whole claim is that a stranger, given this repository and nothing else, can build
the crate and run its proofs from the README alone. A real agent runs that check on a
cadence, and the weekly `stranger cadence` job fails when the
last run is past 28 days.

What a run is:

1. Clone the repository fresh.
2. Follow the README and nothing else: build, test, and run the proofs.
3. Note every place the README lied, omitted, or assumed. Those are findings against the README,
   not against the stranger.
4. Record the date on the line below.

Last run: 2026-11-15

Result of that run: a fresh clone, following the README alone, built the crate, passed its unit and
documentation tests, built the library for all three bare-metal targets, passed the tests at Rust
1.85, the stated minimum, and ran the fake controller example to four successful control
transfers. One falsification patch made its harness fail as the README says. It found no gap. Its
limits: the toolchains and Kani were already installed, so the install steps ran only as a
separate `cargo install` into an empty cargo home, and Kani ran a few harnesses and not the whole
`cargo kani` run, so the timing the README gives went unchecked. The record is
[issue #18](https://github.com/nifeos/extensible_host_controller_interface/issues/18). The
previous run, which ran every proof, is in [its appendix](stranger-test-run-2026-11-11.md).
