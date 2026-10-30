# 0037 — The enforced TDD cycle

Status: accepted · Date: 2026-10-30

## Context

The fork's thesis is enforcement: test-driven development, a perfect
handoff, and small composable elements are refusals, not guidance.
The loop stack already supplies the primitives this needs — the
versioned envelope, `rev_of` digests, the scoped runner (ADR-0035)
— so the cycle is a state machine over them, not a new subsystem.
This ADR records the five decisions where the fork's strictness met
el's existing contracts.

Numbering note: 0036 is the renderer empty-block fix, stacked on
`feat/loop-p2`; this fork takes 0037 so the two numbers do not
collide when the lines rejoin.

## Decisions

### 1. Strict only; no policy mode, no silent fallback

The cycle runs in strict mode and nothing else. A policy mode that
warned instead of refusing would be the old guidance with a new
name, and the fork exists to end that. Every violation below is a
refusal with a named code, never a warning.

### 2. Locks are element revisions, not file hashes

`start` records the test lock (every `Kind::Test` element's
`rev_of` digest), the production baseline (every other element),
and the config lock (sha256 of `el.policy` and `Cargo.toml`).
File hashes would be wrong here: el renders `#[test]` items into
the same `src/*.rs` files as production code (ADR-0030), so a
test edit and a production edit touch the same file. The lock
granularity must match the element granularity the machine
reasons about. Config stays file-hashed because no element
covers it.

### 3. Red is classified, not just failed

`red` runs the target test and reads the output: an assertion
failure (a FAILED line naming the target, no compile errors) is
red; a compile error, a missing test, or a harness failure is a
setup error; a timeout is a setup error, never red evidence.
Each maps to `EL_INVALID_RED`, because a red that was never a
real assertion proves nothing about the fix that follows.

### 4. Impact is a catalog scan, and its limit is a fallback

`impacted` runs the target test plus every test whose statements
mention a changed production name, whole-word, over the catalog —
the same rule `explain` scans with, and stated as honestly: it is
not name resolution. A changed element with no name (an impl
block) cannot be scanned for, so impact is unavailable and the
full suite runs instead, recorded as `fallback: "all"`. The
fallback is the honest answer to an unanswerable question, not a
silent widening: the report says which path ran.

### 5. Verify closes the run; abandon is the deliberate close

`verify` requires the compiler to pass and every lock frozen since
the last blessed state; a stale baseline is `EL_TDD_PHASE`, not a
re-bless. On success the active marker is removed and the record
stays as history. `abandon` removes the marker from any phase
without advancing it: the escape hatch for a run that cannot
proceed (a locked test that genuinely needs correction), kept
deliberate by being its own verb rather than a flag on the next
phase.

## Consequences

- A run is one active marker per crate; `start` refuses while one
  exists. There is no queue and no nesting.
- A test that legitimately needs correction requires a fresh
  cycle: `green` never accepts a test change, and there is no
  in-cycle repair path.
- `verify` runs `cargo check --all-targets`, the gate's scope
  (ADR-0026), not the gate itself: verify reports, it does not
  commit, so it never writes a baseline.
- The handoff gates (CTQ tests as el gates) are a second build on
  this machine, not part of this ADR.
