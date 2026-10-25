# 0008 — Hand-rolled argument parsing and JSON

Status: accepted · Date: 2026-10-24
Context: `el` needs an argv parser and a JSON reader/writer. `clap` and `serde_json` are the
reflex choices; ADR-0001 forbids both.

Decision: one declarative table (`cli/table.rs`) describes every command, its flags, arity,
and help text. `parse.rs` interprets the table; `help.rs` and `skill.rs` render it. JSON is a
single `core/text/json.rs` with a value tree, a reader that rejects malformed input loudly, and
a writer with stable key order.

Why the table is load-bearing: `--help`, `el skill`, the usage errors and the completion all
come from one source, so "documented" and "accepted" cannot drift. That is also the reason the
parser is not a `match` on `argv[1]`.

Consequences: roughly 300 lines of parsing we own, with tests for the cases that matter
(missing value, `--flag=value`, `--` terminator, repeated flags, unknown flag → exit 1 with the
offending token). No derive macros, no reflection, no schema. JSON is limited to the shapes we
read (`cargo` frames, LSP frames, crates.io responses) and write (our envelope); it is not a
general-purpose library and does not pretend to be.

Revisit if: the table grows past ~150 rows, or a JSON shape arrives that the value tree cannot
express without contortion.
