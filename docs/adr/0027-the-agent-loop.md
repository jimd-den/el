# 0027 — The agent loop: find, atomic batch add, crate-level attribution

Status: accepted · Date: 2026-10-26

Context: measured on a 6,147-byte program, el costs ~31% more tokens than
plain editing (build 8,340 vs 6,353 bytes; edit loop 1,740 vs 1,017). The
edit loop is the thing to optimize — how fast an agent finds out it is
wrong — but token savings must not weaken the invariant that every
committed tree passed its gate. Three measured drivers: discovery costs
1,503 bytes (`tree` dumps the whole crate outline because `show` takes
ids only), one command per element (22 commands on the test program),
and a crate-level error whose identity flips with its span's owner.

Decision 1 — `el find <query>` is a read verb, catalog-only per ADR-0020.
It matches a named element when the query equals its name, its alias, or
its `module::name` path, and prints one outline line per match
(`el-XXXX alias`) with a JSON payload (id, kind, alias, module). Order is
the renderer's (module, id). No match is `notfound.element`, exit 6 —
a miss is a named refusal, not an empty success. Statements have no
names and stay out of scope; their owner is found first, then `show`.

Decision 2 — `el batch` reads a JSON array from stdin; each item carries
the fields of one `add` (kind, name, in, vis, sig, body). Every item
becomes a `Change` through `change::add` with a literal body source, and
all of them ride ONE `Mutation`: plan-all against the working catalog
(later items see earlier ones, so a module and its first function batch
together), gate once on the final tree, commit once under one sequence.
Any failure applies nothing: a plan violation names its change
(`change {i} ({kind} {name} in {module}): <violation>`) before anything
is staged — the same naming serves every multi-change mutation — and a
gate rejection restores the old tree and journals one
`rejected:<n>` attempt whose diagnostics name batch items through their
aliases. `--dry-run` previews the whole batch; `--fast` and `--strict`
pass through as for `add`. Statements, anchors and `--doc` are refused
`usage.unsupported` in v1: statements need ids that do not exist until
planning, and the other two have separate semantics. A module item's
`body` is its preamble, which is the one place a batch may write prose.
Rejected alternative: one transaction per item (the `apply` shape) —
that is exactly the partial application this decision forbids.

Decision 3 — the gate never attributes code `E0601`. "Main function not
found in crate" is about the crate, not an element: its span points at
arbitrary file content, so attributing it makes its fingerprint flip
whenever the span's owner changes (witness: adding a struct to a
main-less crate was rejected as 1 new error, the scaffold's unattributed
E0601 counted as fixed). Unattributed, it falls back to (code, path,
message) and stays pre-existing across adds until `fn main` lands. No
other code is carved out: for every other diagnostic the span's owner
is the identity, and dropping it would let an identical error in a
second element hide behind the first.

Consequences: `--quiet` is already the agent mode (notes suppressed,
ids and errors stay on stdout) — no new output format was needed.
`find` plus `batch` are the token levers; the correctness invariant is
unchanged, and a batch undoes as one entry because undo restores a
snapshot. Content search stays M11's job; `find` searches names only.

Revisit if: batch items need statement anchors, or a second crate-level
code shows the same span instability (carve it out by the same rule:
the code names a crate condition, not an element).
