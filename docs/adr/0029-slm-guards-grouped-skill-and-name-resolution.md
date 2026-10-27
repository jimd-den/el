# 0029 — SLM guards: a grouped skill page and a name-to-id correction

Status: accepted · Date: 2026-10-27

Context: research into small-model coding failures (full survey in
`docs/reports/slm-pitfalls.md`) finds the failures are mostly interface,
selection, and anchoring — not reasoning. Two of them el still feeds. First,
selection: tool-selection accuracy falls as a flat catalog grows past
~15–25 tools, and the remedy the literature converges on is grouping tools
by intent and scoping the set per turn; el's skill page lists 24 verbs
flat, and el cannot scope per turn because it keeps no conversation state.
Second, addressing: a small model remembers names, not ids, so it addresses
by name where el requires an id (`show greet`) and gets a refusal that
names no way forward beyond re-listing the whole crate.

Decision 1 — the skill page groups verbs by task: write (new, init, add,
batch, apply, edit, rm, mv), read (ls, tree, show, find, explain, diff,
log), history (undo, redo, revert), verify (run, check, doctor), meta
(version, skill, help). The grouping is the static form of per-turn
scoping: selection is by intent, not by scanning a flat catalog. The
150-line ceiling is unchanged, and the golden file pins the new layout.
Rejected alternative: per-verb help scoping (`el help edit` printing only
that verb) — `help` prints the whole page today, so scoping it is a
second feature, not a refinement of this one.

Decision 2 — a subject that is not an element id gets a correction, not
just a refusal. `change::parse_id` is the funnel for every id-taking verb,
so one change covers show, edit, rm, mv, and anchors: when the subject is
a single whitespace-free token, the message appends the exact resolution
(`resolve a name with `el find <text>``); anything else cannot be one
`find` argument and gets no pointer. The same rule covers the `apply`
script's id check, through one shared helper. The code-level hint is
unchanged (`el ls` for `notfound.element`): the hint stays a valid command
for the general case, while the message carries the specific correction.
Rejected alternative: a subject-aware hint field on the error — every
error constructor in the tree would need the new field for a gain the
message already delivers.

Deferred, with reasons. Repeat-refusal escalation (a second identical
refusal varying its hint, against repair-loop anchoring) needs refusals
recorded across processes; plan refusals write nothing by design, and
gate rejections already journal, so this would split the journal's
meaning — it needs its own decision, not a ride-along. `el check` delta
labels (new vs pre-existing, for check-aware feedback) need baseline
comparison, which belongs to the gate path; `check` stays a plain report.
Compiles-but-wrong code stays out of scope: no tier runs tests, and that
is the safety-line decision, not an agent-surface change.
