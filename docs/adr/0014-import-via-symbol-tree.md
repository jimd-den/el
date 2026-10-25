# 0014 — Importing an existing file: symbol tree first, parser never

Status: deferred (design accepted) · Date: 2026-10-24
Context: `el init` and `doctor --adopt` bring a file in as one opaque module (ADR-0009). That is
honest but coarse: an adopted 300-line module cannot be edited at element level. A user who
arrives with a working crate should be able to move toward modelled source without rewriting it.

Decision (when it is built): `el import <path> --from-symbols` asks `rust-analyzer` for
`textDocument/documentSymbol`, which returns a symbol tree with exact ranges and kinds. `el`
slices the file text by those ranges to build items, and keeps each body as opaque text — the
same conservative rule as ADR-0013. Statements are *not* inferred from symbols; a body stays
opaque until `el split <owner>` is asked for explicitly (D-8).

This is the only route to modelling existing code, and it deliberately stops short of writing a
Rust parser. RA supplies syntax knowledge; `el` supplies identity, history, gating and policy.

Alternatives rejected: writing a parser (ADR-0002); regex-shaped heuristics (silently wrong);
importing as anonymous items with no ids (breaks history and attribution).

Consequences: adoption becomes a spectrum — opaque, symbols, statements — and every step is
observable and reversible. The risk is that `documentSymbol` gives a different tree than a
compiler would (macro-expanded items, `cfg`-gated items); the mitigation is that the import is
gated by `cargo check` like any other change, so a bad tree is refused rather than committed.

Revisit if: RA grows a stable "syntax tree for a file" request that returns a CST, at which
point the slice-by-symbol step becomes a slice-by-node step.
