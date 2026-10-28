# 0032 — Journaled undo and redo

Status: accepted · Date: 2026-10-27

## Context

The SLM bench, attempt 3, task 10-apply: the agent ran a
successful apply (journal sequence 13, "apply 6 changes"), then
ran `el undo`. Undo printed only `head 13 -> 12` and journaled
nothing. The agent then read an empty tree, found nothing in
`log` marking sequence 13 as restorable — log shows no head
position at all — and finished confused in an empty crate, so
the task failed after succeeding.

Undo and redo were head moves plus a full materialize (FR-29):
they restored the catalog snapshot of a sequence and moved
`head`, journaling nothing. Redo existed only while head sat
behind the journal tip, and a new mutation truncated that
future to the attic (FR-30). Nothing was destroyed — the
snapshots stayed — but from the agent's side the work had
vanished with no visible road back: undo's output named
neither what it undid nor the command that restores it.

The order behind this branch: undo must not silently delete
a commit. Every head state must be a journaled sequence, so
the journal is the whole story and `log` alone orients the
agent.

## Decision

1. **Undo appends.** `undo(n)` appends one `undo` entry per
   sequence stepped back, not one entry for the jump, so the
   undo(n)/redo-step symmetry survives: undo 2 then redo
   twice returns exactly. Each step restores the previous
   sequence's tree and catalog (the revert skeleton:
   truncate the future when head sits behind the tip, set
   the inflight marker, restore the projection, append the
   entry, advance head, copy the catalog, clear the marker).
   Each entry: `op = undo`, `target` = the sequence restored
   to, `from` = the pre-step tree digest, `to` = the restored
   tree digest, `gate = model` (a history move restores a
   previously gated state, as revert already does),
   `msg = "undo <undone entry's msg>"`.
2. **Redo appends.** Redo scans the journal backward from
   the tip while entries are `undo` or `redo` — the trailing
   move run. Inside it, an undo entry is redone when a later
   redo entry in the run has `target` equal to the undo's
   undone sequence (`target + 1`). Redo appends a `redo`
   entry restoring that sequence's tree and catalog,
   `target` = the restored sequence,
   `msg = "redo <restored entry's msg>"`. A new mutation
   after undos breaks the run, so redo after a new change
   still fails exactly as FR-30 required — and the undone
   work is no longer orphaned to the attic for it: it stays
   in the journal, revertible by sequence. For crates written
   by older binaries with head behind the tip, redo keeps
   the old head-move path when a `head + 1` entry exists.
3. **Undo output names the road back.** Each appended step
   prints `undid <msg> (head <old> -> <new>)`, and undo
   prints a final `redo with `el redo`` line. The skill
   page's undo/redo lines change to match: undo appends an
   entry restoring an earlier state, redo appends an entry
   restoring an undone state.
4. **Log needs no change.** Undo and redo entries render
   through the existing journal line format, so `log` alone
   now shows every move and every road back. Head is always
   the tip for crates written by this version, so no
   behind-tip marker is needed.

## Alternatives rejected

- **Announce-only undo** (same head move, louder output):
  it still deletes whole commits from history silently in
  effect — the journal would still end at the undone
  sequence with no record of the move. It violates the
  principle instead of honoring it.
- **Peel-one-change undo** (undoing an apply removes only
  its last change): it produces states that were never
  committed, needs change-level inversion inside a sequence
  plus a peel stack for redo, and breaks apply atomicity's
  history shape. More machinery for a weaker invariant.
- **Removing undo**: agents need a back-step, and
  revert-to-sequence is coarser, not finer.

## Consequences

- The journal grows with undo/redo entries; each names its
  step, so the growth is readable, not noise.
- Old binaries cannot parse the new `undo`/`redo` op
  strings; journals are not promised cross-version, and
  dogfood runs the new binary.
- `Op` gains `Undo` and `Redo` (ALL becomes 14); `doctor`,
  `diff`, `show --history`, and `log --attic` need no
  change — they already render ops through `as_str`.
- The FR-30 test's count assertions are superseded: after
  undo plus a new change, nothing moves to the attic,
  because there is no future to orphan. Its replacement
  pins the new invariant (undone entries stay journaled,
  redo fails, revert to the undone sequence restores).
- The `undo_restores_bytes` head assertion moves from a
  fixed sequence to tip + steps; its bytes assertions are
  unchanged. Both test changes are justified in the PR
  body, as the push tooling squashes to one commit.

## Revisit if

- Undo spam (an agent undoing in a loop) makes journals
  unreadable in practice; the remedy then is a
  compacting read of `log`, not silent moves.
