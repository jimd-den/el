# 0021 — Statements are addressed, renumbered, and attached to their comments

Status: accepted · Date: 2026-10-26
Context: M7's second half makes the statement the unit of edit. Three decisions
were forced: what an anchor means, what happens to sibling positions when one
moves, and what the policy's `split = auto|always|never` does to a body the grammar
cannot prove.

Decision:

1. **Ids are stable; indices are derived.** An insert or reorder keeps every
   existing id and renumbers the sibling list `0..n`. `children_of` sorts by
   `(index, id)`, so source order is deterministic and a move reads as a move in
   history, not as a delete plus an add.
2. **`add stmt <owner>` takes exactly one anchor.** `--after`, `--before`, `--at`
   (zero-based), or no anchor for append. `mv <stmt> --after <stmt>` reorders
   inside one body; a cross-body anchor is `model.cross_body_move`, refused before
   any write.
3. **`edit <id> --body` resolves the id in the catalog.** A new `EditBody` change is
   an element edit when the id is an element and a statement edit when it is a
   statement; the apply layer decides, not the command line. A statement edit keeps
   its id and replaces its text and children. One top-level statement is required
   (`model.bad_stmt`), so a body cannot silently become several statements.
4. **`split` is policy.** `never` stores one opaque statement and `show` reports
   `opaque`; `always` refuses a body the grammar cannot prove; `auto` falls back to
   opaque. L0's unbalanced check skips statements whose owner element is opaque:
   otherwise `never` would not be the escape hatch FR-72 promises.
5. **Leading comments and attributes belong to the following statement.** The
   grammar attaches them to that statement's text, so a reorder or move carries
   them; a trailing run with nothing after it stays a statement. `show doc` takes
   only the leading `///`/`//!` lines, not the code that follows.

Alternatives rejected: renumbering ids on reorder (breaks history and references);
a separate `Statement` enum in every command (the model already has one); storing
comments as siblings and moving them by a separate lookup (a statement could be
orphaned); making `auto` always refuse (the splitter is lexical and FR-63/72
require an opaque fallback).

Consequences: a statement's text may begin with its doc comment; editing that
statement replaces the comment too, which is why `show doc` derives the comment
lines. `--at` is a position in the current sibling list, so a later insert shifts
it. Revisit when `--doc`/`--attr` become first-class fields: the attachment rule
then moves from text to fields, and this ADR's consequence narrows.
