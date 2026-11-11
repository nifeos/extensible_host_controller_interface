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

Last run: 2026-11-11

Result of that run: the stranger reached a passing build, 30 unit tests and 2 documentation tests,
and all 7 Kani proofs in about 90 seconds, from the README alone. The same tests passed at Rust
1.88, the stated minimum. One falsification patch made its harness fail as the README says. Kani
0.68.0 was already on the machine, so the run did not test the install steps. It found no false
claim. It found five places where the README left the reader to guess: the build and test commands,
how long the proofs take and how to run one, how to run a harness against a patch and revert it,
whether a passing run's warnings are normal, and how to check the minimum Rust version. The full
record is in [the run appendix](stranger-test-run-2026-11-11.md).
