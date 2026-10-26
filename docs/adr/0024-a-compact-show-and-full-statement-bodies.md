# 0024 — A compact `show`, and statement listings that carry full bodies

Status: accepted · Date: 2026-10-26

Context: measured on a real two-module program, the edit loop through the
read verbs cost more than the plain-file loop (790 bytes vs 480). Two
measured causes: `show` prints ten field lines (~180 bytes) to convey one
statement, and `show --statements` truncated every statement to its first
line while the JSON `statement_tree` already carried the full text — so
the lines half and the payload half disagreed about what exists.

Decision:
- `show <id> --terse` collapses the header to one line: `<id> <kind>
  <alias> <module> <signature>` for an element, `<id> stmt owner <owner>
  parent <parent> <text>` for a statement. Doc, mode, and sequence are
  dropped in terse form; they remain one full `show` away.
- With `--statements`, terse lists the statement tree under the header:
  one id-bearing line per statement, full text, continuations of a
  multi-line statement indented without a repeated id, nesting by indent.
  No `statements:` section header in terse form.
- `show --statements` in both forms now prints full statement text, not
  the first line. The lines half and the payload half agree again.
- `--source` and `--history` sections keep their existing shape under
  `--terse`, and the JSON payload is identical with or without `--terse`:
  terse is a rendering of the lines half only, so no `--json` consumer
  changes.

Alternatives: make terse the default (rejected: the field form is the
documented contract and scripts may parse it); a separate `show1` verb
(rejected: a flag on the existing verb is the smaller interface); a
compact `tree` too (deferred: the three-level outline is already the
cheap path at ~140 bytes, and scope stays on the measured cost).

Consequences: the edit loop for one statement becomes `tree` (outline)
→ `show <fn> --statements --terse` (header + bodies) → `edit <stmt>`,
with no whole-program dump and no source read. The remaining cost floor
is the header line plus the function's own statements, which is the
information the edit actually needs.

Revisit if: a terse consumer wants doc or sequence inline, or the
statement listing needs a machine-parseable separator between a
statement's continuation lines and its next sibling.
