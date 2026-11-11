# Security policy

A security report here is about shipped code. The crate holds no `unsafe` code (`src/lib.rs` forbids
it), touches no hardware and allocates nothing, so it cannot corrupt memory by itself. What it can
do is hand a driver a wrong answer: a field decoded from the wrong bits, a register offset that
names the wrong register, a ring step that places a Transfer Request Block where the controller does
not look, a port write that acknowledges a change nobody asked to, or a Kani proof that does not
hold. Any of those is worth a report, and so is a way to make it panic or spend unbounded time.
Report through GitHub's private vulnerability reporting, which this repository has enabled. What you
get back is a real person reading the report, not a triage queue.

Fuzz crash inputs are public, because a crash in a crate with no `unsafe` code is a panic or a
hang and never a memory-safety flaw.

Two facts about how this code is written are worth knowing before you read it. Most of it was
written by AI agents working in parallel, with one human reviewing the design and the
outcomes. And every property the crate claims is a Kani harness in the tree, with a falsification
patch that shows what turns it red; a report that contradicts a harness is also a bug in the
harness, and both halves are interesting.

## Supported versions

One line: the newest release. The shared workflows fail a call at an older template release, and
 Dependabot keeps the crate's own dependencies weekly, so a fix lands as a version bump you can
take.
