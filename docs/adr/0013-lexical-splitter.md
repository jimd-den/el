# 0013 — The statement splitter is lexical and refuses to guess

Status: accepted · Date: 2026-10-24
Context: ADR-0012 needs statement boundaries. Parsing Rust to find them violates ADR-0001 and
ADR-0002. A heuristic that is right 98% of the time is worse than no split at all, because a
wrong boundary is silent: the tool would then edit a "statement" that is really half of an
expression, and the compiler would blame the user.

Decision: split using **delimiter depth plus a scanner** that skips string literals, raw
strings, byte strings, char literals, line comments and block comments. A statement is the text
between top-level `;` boundaries, or one balanced block (`if`/`match`/`for`/`while`/`loop`/
`unsafe`/labelled block) including its header and trailing modifiers. If the scanner cannot
classify something with certainty, the splitter returns **"cannot split"** and the body is
stored as a single opaque statement. There is no third outcome (invariant I6).

The two properties that make it trustworthy:

1. `join(split(b)) == b` — lossless, byte for byte, for every body ever written or read.
2. `split(join(split(b))) == split(b)` — idempotent, so re-rendering never reshuffles.

Both are property tests over a corpus; the refusal cases (raw strings containing braces, nested
macros, `'a` lifetimes next to `'a'` chars, block comments containing `*/`) are checked-in
fixtures, so a future "improvement" that starts guessing fails a test.

Alternatives rejected: a `syn`-based parser (ADR-0001); indentation heuristics (wrong for
macro-generated and multi-line expressions); "best effort, warn on failure" (a warning is not a
gate — the body would be wrong and the user would find out from rustc, three phases later).

Consequences: some bodies will be opaque forever, and that is acceptable: opaque bodies still
get the full gate, history, references and rename. `split = always` in `el.policy` turns the
fallback into a hard error for teams that would rather fix the body than lose the granularity.

Revisit if: `el import --from-symbols` (ADR-0014) makes RA available as a splitting oracle — the
splitter could then *confirm* its boundaries instead of only refusing them.
