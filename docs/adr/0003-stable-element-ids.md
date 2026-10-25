# 0003 — Stable element ids, decoupled from names and locations

Status: accepted · Date: 2026-10-24
Context: every feature that matters — history, references, diagnostics attribution, tags —
needs to name an element across a rename, a move, and a rewrite. A name-based key
(`core::plan::apply`) breaks on every rename; a path-and-offset key breaks on every edit above
it.

Decision: identity is a monotonic, never-reused id `el-NNNN`, assigned at creation and stored
in the catalog. Names and modules are *attributes*. A human alias (`core::plan::apply`) is
derived and unique, and is accepted anywhere an id is.

Consequences: ids are less readable than names, so every command prints both one line apart
and accepts both on input. Ids leak into `--json` and logs, which is intended: it is what makes
`el log` readable a year later. Reusing an id would make history lie, so deleted ids are burnt.

Alternatives rejected: content hashing the element (id changes on every edit, so history is
unreadable); ordinal position (changes on insert, which is exactly when we need it stable);
full path plus name (breaks on `mv`, which is a supported daily operation).

Revisit if: users need cross-crate ids (workspace, ADR beyond scope) — the id would gain a
crate prefix, additively.
