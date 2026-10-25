# 0001 — Zero third-party dependencies

Status: accepted · Date: 2026-10-24
Context: the tool must be installable on a bare machine, compile in seconds, and be auditable
in an afternoon. The Rust CLI ecosystem's default (`clap`, `serde`, `anyhow`, `regex`) is
roughly 20 crates and a multi-megabyte binary before any feature exists. The target
environment is minimal: a developer or CI container with `cargo`, possibly no network.

Decision: `el` has an empty dependency graph. `std` only. Arg parsing, JSON, SHA-256,
TSV, the statement scanner and the LSP client are implemented here, each small and tested.
Everything else the tool needs from the outside world is a **process**: `cargo`,
`rust-analyzer`, `curl`.

Alternatives rejected:
- `clap` — 100 KB of binary and ~10 crates for a surface we generate from a table anyway.
- `serde_json` — we need one reader and one writer for four fixed shapes; `core/text/json.rs`
  is ~170 lines and does not need derive macros.
- `sha2` — 110 lines of well-specified code with NIST vectors, versus a dependency the whole
  audit story then has to account for.
- `ra_ap_*` — 40 crates; see ADR-0011, we speak LSP to the binary instead.
- `libc` — not needed: `std::io::IsTerminal` (1.70) covers TTY detection, `std::process`
  covers spawning, `std::fs` covers atomic rename.

Consequences: we own and test a small amount of "boring" code; we cannot use ecosystem
conveniences; a new dependency becomes a reviewed, ADR'd decision rather than a reflex.
Payoff: `cargo install` in seconds, a 1.5 MB binary, a reproducible build with one package in
`Cargo.lock`, and no supply-chain surface at all.

Revisit if: the JSON or LSP work exceeds ~400 lines each, or a required capability (signals,
`io_uring`, TLS) becomes load-bearing rather than optional.
