# 0011 — `rust-analyzer` as an LSP subprocess, out of the box

Status: accepted · Date: 2026-10-24
Context: `el` needs semantic answers a compiler cannot give cheaply: who references this
element, what does a rename touch, what does this type actually mean, what quick fix does the
compiler suggest. The Rust ecosystem already has exactly one good answer, and it is already
installed on most Rust machines (`rustup component add rust-analyzer`).

Decision: treat `rust-analyzer` as a **system tool**, like `cargo`: a child process spoken to
over LSP on stdio, with one short-lived session per command that needs it. Auto-discovered from
`$EL_RA`, then `PATH`, then `rustup which rust-analyzer`. Zero configuration, zero flags
required. Absence is a supported state, never an error.

Capabilities gained, each mapped to a requirement: `textDocument/references` (FR-50),
`textDocument/rename` (FR-51), `textDocument/diagnostic` (FR-53, tier L3),
`textDocument/codeAction` (FR-54), `textDocument/hover` (FR-55),
`textDocument/documentSymbol` (FR-61 / ADR-0014).

Alternatives rejected:
- **Link `ra_ap_*` or `lsp-types`.** ~40 crates and tens of megabytes; violates ADR-0001 and
  would make the binary an order of magnitude larger than its own feature set.
- **Write our own resolver.** Years of work, and wrong for macros and `cfg`.
- **Require RA.** Breaks the bare-machine promise; CI containers often lack it.
- **A persistent daemon with a socket.** Adds a second lifecycle, stale-pid handling and a
  port; RA's disk cache makes a fresh session cheap enough. Deferred, not rejected (D-6).

Consequences: we implement Content-Length framing, request correlation, timeouts and edit
application (~600 lines across four files plus a session policy), tested against a scripted fake
LSP server so the fast suite never spawns the real binary. RA's stderr is quarantined, its
output is sorted for determinism, and it is spawned by no command that does not need it
(FR-60). L1 stays the default gate because RA is a second opinion, not the arbiter — and when
the two disagree, `--strict --semantic` shows the disagreement instead of hiding it.

Revisit if: a session costs more than ~2 s warm on a typical crate (then D-6's daemon), or RA
gains a stable batch-mode CLI that returns diagnostics for a tree without an LSP handshake.
