# 0005 — Report the diagnostic delta, not the error count

Status: accepted · Date: 2026-10-24
Context: a crate mid-refactor has errors. If the gate printed "your change produced 7 errors",
it would block work that is making progress, and users would learn to pass the escape flag.
The question a user actually asks is: "did I make it worse?"

Decision: the verdict compares the candidate tree's diagnostics against a baseline recorded
for `head`, and blocks only on **new** errors. Always three numbers: new, fixed, pre-existing.
Fingerprint = `(code, innermost element id, normalized message)` — never a line number, because
a line number changes when anything above it changes.

Why this needs the span map: because `el` renders the file, it knows which element owns each
line. That is the only reason a compiler position can become an element id. A line-based delta
would report a phantom new error whenever a statement was inserted at the top of a body, which
would make the gate noisy enough to be ignored.

Consequences: the baseline must live beside the tree (`GATE.md` §4.2) and must be reproducible
when missing (run the previous tree once). Warning-only changes are reported as notes, blocking
only under `--strict`. An opaque module degrades to a line-sensitive fingerprint, and the
output says `"exact": false` rather than pretending.

Alternatives rejected: absolute count (blocks progress); "exit code of cargo" (101 tells you
nothing about *this* change); line-based delta (phantom results).

Revisit if: cargo ever exposes stable diagnostic ids — we would adopt them as an additional
fingerprint component, not a replacement.
