# 0012 — Statements are elements

Status: accepted · Date: 2026-10-24
Context: with items as the smallest addressable element, changing one line of a function means
replacing the whole body. For an agent that is expensive (the whole body must be reproduced
correctly) and for a reviewer it is noisy (the diff shows a function, not a line). Most mistakes
are one line long, so the unit of a change should be able to be one line.

Decision: a `stmt` is an element kind. A function or impl body is an ordered list of statement
elements, nested to any depth, each with an id, an owner, a position, attached comments and
(for `let`) a recorded binding. `el add stmt`, `el edit <stmt>`, `el rm <stmt>`, `el mv <stmt>`
operate on one statement; the compiler gate, the span map and the diagnostic delta all
attribute to the innermost statement id.

Consequences — the good and the bad:

- **Good.** An agent can replace `el-0102` without ever reading the rest of the body. A
  diagnostic names the statement, so the error report is actionable at the granularity of the
  fix. The line budget becomes per-statement, so `model.ceiling` can suggest which statements
  to extract. Diffs read like a changelog instead of a file dump.
- **Bad.** Ordering, nesting and attachment (which comment belongs to which statement) become
  real bookkeeping. Reordering is a list operation with ids, so history reads as a move rather
  than a delete plus an insert — which is why it is worth the bookkeeping.
- **Bounded.** A body that cannot be split losslessly is stored as one opaque statement
  (ADR-0013). Nothing else in the design depends on splitting succeeding.

Alternatives rejected: hunks (no identity, so no history and no attribution); line numbers
(invalidated by every edit above them); re-parsing the body on each command (needs a parser,
ADR-0002).

Revisit if: statement bookkeeping becomes the top source of bugs — the fallback (`split = never`)
already exists and costs only granularity, not correctness.
