# Fork charter: enforcement is the whole point

Thesis (one thing): this fork enforces test-driven development, enforces
a perfect handoff, and enforces small and composable. Nothing else is
the point. Any feature that is not enforcement is out of scope.

## 1. TDD, enforced (not encouraged)

- No production element without a prior failing test. Red must be
  observed, and it must be an assertion failure, not a compile error.
- Green touches production only. A green step that edits a test is
  refused.
- Verify is a required step, not an option: the test passes, the gate
  passes, then the step is done.
- Tests and config are hash-locked during a TDD cycle: a locked test
  cannot be edited to make green pass.
- This is Phase 3's TDD state machine, taken as the fork's core rather
  than one phase among five.

## 2. Perfect handoff, enforced (and itself test-driven)

- Every step transfer satisfies the handoff contract in
  `~/workspace/el-slms/stepwise-handoff.md`: task + last WINDOW steps +
  counts, never the transcript, persisted atomically, resume-faithful.
- The handoff element is itself test-driven: the CTQ tests are gates.
  A handoff change lands only with a failing test first, same as any
  production change. The handoff is not exempt from the rule it enforces.
- A step that cannot prove its handoff — missing state, unpersisted
  step, unverifiable resume — is refused, not warned about.

## 3. Small and composable, enforced as refusals

- Function budget (15 lines), file ceiling (200 lines), one action per
  step: these are refusals, not guidance. Over budget is refused, the
  way an unbalanced block is refused.
- A change that makes a unit bigger must split it first. Bigness is
  never the price of landing a change.

## What the fork refuses

- Features that are not enforcement, however useful.
- Loop conveniences that weaken any of the three: silent fallbacks,
  retry-without-evidence, summary-without-source.
- Anything that lets a step proceed on an unverified handoff.

## How enforcement is verified

- el's `make check` gate, plus the handoff CTQ tests as gates.
- Bench CTQs per run: pass rate, tokens per success, wall-clock and
  wait share, retries per task. Enforcement that cannot be measured
  is a claim, not a property.

## Base (decided 2026-10-29: loop-p2)

- `feat/loop-p2` (PR2 tree): has the handoff primitives
  (status/discover/revision-bound read/evidence) the fork builds on.
  The fork starts once the PR2 push lands; until then the charter is
  the fork's specification, not its branch.
