# 0025 — A change messages how many files it rewrote

Status: accepted · Date: 2026-10-26

Context: the token story for element editing is "one statement, not the
file", but nothing in the tool's output ever said so — an agent had to
take it on trust or read the files back. Separately, a quiet run was
proposed as the next token saving, but on inspection it already exists:
`--quiet` is a global flag (COMMANDS.md §5, "Suppress human notes;
errors only"), and cargo's build noise reaches the user only through
`run`'s notes, so `el run --quiet` already prints program output only.
No run change was needed; this record covers the message that was
actually missing.

Decision:
- `materialize` returns how many files it wrote, split into added and
  changed, and a committed transaction appends one note: `el: rewrote 1
  file` for a pure edit, `el: wrote 1 file` for a pure addition, `el:
  wrote 2 files (1 new, 1 rewritten)` for a mixed change such as `add
  mod`, which adds the module file and rewrites the crate root's
  declaration.
- The note appears only on the commit path. A refusal, a rejection, an
  abort, and a dry-run preview write nothing persistent, so they message
  nothing. History moves (undo/redo/revert) restore a whole tree rather
  than materializing a diff, so they do not carry the note.
- The note is a human note: it prints to stderr with the other notes,
  carries no promise (report.rs), and `--quiet` suppresses it along with
  every other note, by that flag's contract.

Alternatives: count statements instead of files (rejected: the file is
what the agent would otherwise rewrite wholesale, and the statement
count is already visible in `changed`); put the count in the JSON
payload (rejected: mutations leave payload `None` by contract, and the
note is prose for the human/agent reading the run, not machine data);
message the count from `run` (rejected: run renders nothing — verbs
render on commit — so a run-time count would always be zero).

Consequences: the edit loop's footprint is now visible in the output
itself — `el: rewrote 1 file` after a one-statement edit — and a quiet
run costs only the program's own output plus whatever the agent typed.

Revisit if: a consumer wants the count machine-readable, or history
moves want the same treatment for their full-tree restores.
