# 0010 — Project constants: refilling `AGENTS.md` §0 for `el`

Status: accepted · Date: 2026-10-24
Context: the governing `AGENTS.md` describes a different project (ỌKỌ, a display-list console)
in §0. This repository's `PROJECT` line, and only that line, would be false if left as-is.
Every command, path, budget and rule in §0–§10 still applies.

Decision: M0 replaces §0's `PROJECT` value with the one-line description of `el` and keeps
every command identical, so the gate does not change:

| Slot | Value for `el` |
|---|---|
| `PROJECT` | `el` is a Unix filter that builds Rust programs by name — modules, items and statements — verifies every change with the compiler before committing it, and can undo any of it, with zero dependencies. |
| `LANGUAGE` / `STACK` | Rust 1.99.1, edition 2024 / `std` only; no crates, no async runtime, no GUI; external tools invoked as processes (`cargo`, `rust-analyzer`, `curl`) |
| commands, paths, budgets | unchanged from `AGENTS.md` §0 |

The original ỌKỌ text is preserved verbatim in `docs/reports/oko-constants-archive.md`, so
nothing is destroyed and the change is reversible in one commit.

Alternatives rejected: keeping both projects' constants in one file (a §0 with two truths is
no §0); a second `AGENTS.el.md` (splits the standard, which is the thing that must not split).

Consequences: `PROJECT` is the only line that differs; the gate, the ceiling and the test
commands are untouched. If `el` later lives beside ỌKỌ in one repo, §0 gains a nested table —
which is the moment this ADR should be revisited.
