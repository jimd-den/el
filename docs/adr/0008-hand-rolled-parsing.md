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

## Amendment — 2026-10-25, after implementing M2

Two things the decision above could not have known:

1. **JSON is a package, not one file.** `json/{mod,parse,scalar,string}.rs`: a strict reader is
   ~225 lines before it is finished, which trips the 200 ceiling (§4.1). The seam was already
   there — containers, scalar tokens, and string-literal escapes are three jobs with three sets
   of failure modes, and each got its own mutation check.

2. **A number is kept as its source text**, never parsed to `f64`. `el` does no arithmetic on a
   number it reads, and cargo emits `u64` ids; `f64` rounds them silently and the symptom would
   be a wrong span three layers away from the cause. The reader validates the JSON number
   grammar (`-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?`) and stores the text, so the writer
   reproduces what was read rather than an opinion about it. `1e3` stays `1e3`; `-0` stays `-0`.

The reader is strict and non-streaming. Both callers emit whole documents, and anything malformed
means the stream was cut or the flag is wrong — a lenient reader would hide that behind a
plausible value. Rejected alternatives: `from_str::<f64>` for numbers with a "real" number type
later (rejected: "later" is where the rounding bug lives); a resumable state machine (rejected:
harder to prove correct than a cursor over bytes, and nothing needs it).

Still true from the decision above: the reader rejects malformed input loudly rather than
recovering, and the writer's key order is insertion order, not sorted — canonical for `el`
because `el` is the only producer of its own envelope.
