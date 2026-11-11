# Stranger run, 2026-11-11

The first run of this repository's stranger test. This page records what the stranger did, what
it found, and where the README was not enough. The run changed nothing in the README. The README now
covers each finding below.

## How it was run

- One agent session, model Opus 5.5, acting as both stranger and operator.
- A fresh clone of `main` from GitHub at `bea0257`, in a new empty directory.
- Its guide was the README and nothing else. It read no source file, no rustdoc and no other
  document before the proofs passed. This page, `CONTRIBUTING.md` and the pull request template
  came after, to record the run.
- Task, in short: from the README alone, reach a build, the tests and the proofs, and write down
  every step where the README was not enough.
- The machine: x86_64 Linux, with stable Rust 1.99.1, a 1.88 toolchain and Kani 0.68.0 already
  installed.

## Result

| Measure | Outcome |
|---|---|
| Build | passed first time, `cargo build`, no warnings, under a second |
| Tests | passed: `cargo test`, 30 unit tests and 2 documentation tests, about a second |
| Minimum Rust version | passed: `cargo +1.88 test`, the same 30 and 2 |
| Proofs | passed: `cargo kani`, `Complete - 7 successfully verified harnesses, 0 failures, 7 total.`, in 91 seconds; `the_capability_walk_ends_and_stays_inside_the_limit` took 73 of them, and each other harness under 2 |
| Falsification | `patch -p1` of `verification.a_producer_ring_stays_in_bounds.patch`, then `cargo kani --harness a_producer_ring_stays_in_bounds`, ended `VERIFICATION:- FAILED` and `0 successfully verified harnesses, 1 failures, 1 total.`; `patch -R -p1` left `git status` clean |
| Kani installed by the stranger | no. Kani 0.68.0 was already on the machine, so the run did not test the install steps |

A passing `cargo kani` printed 5,613 lines. Among them were two warnings (`use of an unstable
feature`, and `Found the following unsupported constructs: caller_location (1), foreign function
(1)`), 551 lines of `aborting path on assume(false)`, one check whose status read unreachable, and source
paths under `/home/runner/.rustup`, a directory that does not exist on the machine that ran it.

## Where the README was not enough

1. The README never names the build or test commands. It has no `cargo build` or `cargo test`;
   the stranger guessed them from Cargo convention, and they worked.
2. The README gives no idea how long `cargo kani` takes, and does not say that one harness can run
   alone. It took 91 seconds here, 73 of them in one harness. The stranger found
   `cargo kani --harness` from Kani's own usage, not from the README.
3. The falsification paragraph says to apply a patch with `patch -p1` and to revert it afterward,
   but names neither the command that runs the one harness against the patch nor the command that
   reverts it. The stranger used `cargo kani --harness <name>` and `patch -R -p1`.
4. The README does not say what a passing proof run prints besides its last line. The two warnings,
   the `aborting path on assume(false)` lines, the unreachable check and the CI runner's paths
   all appear in a run that passes, and nothing tells a reader to expect them.
5. The README states the minimum Rust version, 1.88, but not how to check it. The stranger ran
   `cargo +1.88 test`, which needs a 1.88 toolchain; this machine already had one.

## Questions it would have asked a person

1. Which commands build and test the crate?
2. How long should `cargo kani` take, and can one harness run alone?
3. After applying a falsification patch, which command shows that harness fail, and how is the patch
   reverted?
4. Are the Kani warnings, the `aborting path` lines and the unreachable check normal in a
   passing run?
5. How does a reader confirm that the crate builds at Rust 1.88?

## Confident wrong answers

None. Every claim the README made that the run could check held: the example compiled and passed
as a documentation test, there were seven harnesses under the names it gives, every one passed, and
the falsification patch failed at its own assertion.

## Limits of this run

- Rust, the 1.88 toolchain and Kani were already installed, so the install steps went untested.
- The stranger was not a fresh context. It had the organization's instructions in context and had
  read the general protocol for a stranger test. Nothing enforced that it read only the README.
- The stranger was also the operator, and wrote this record.
- Only one of the seven falsification patches ran.
