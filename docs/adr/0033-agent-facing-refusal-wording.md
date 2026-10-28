# 0033 — Agent-facing refusal wording for stmt subjects and body paths

Status: accepted · Date: 2026-10-27

## Context

The SLM bench, attempt 3, tasks 05 and 06: the model could
not learn what `add stmt`'s subject is. It tried an invented
`--text` flag, an invented `--owner` flag, then some fifteen
invented subject names (`let_underscore_x`, `print_42`, …),
then invented batch JSON shapes and YAML/JSON apply scripts
— about 45 failed calls in 05 — before stumbling on the
working form, `add stmt <fn-id> --body -`. Two of el's own
messages looped it: a missing subject says "add stmt needs
a name", so it invented names; a non-id subject says "X is
not an element id; resolve a name with `el find X`", so it
hunted for an id for its invented name — a hunt that cannot
succeed, since a new statement has no element to find. The
working rule, that the subject is the owning function's id,
appears in `docs/COMMANDS.md` (`add stmt <owner>`) but
nowhere the model reads: the skill page has no `add stmt`
example at all.

Task 10 added a second wording gap: `--body "1"` (a literal
where el wants a file) is refused as "1 does not exist",
which never teaches the file-versus-text distinction, so
the model repeated the mistake with `"x * 2"` and `"0"`.

## Decision

1. **Stmt subjects name the owner rule.** A missing
   `add stmt` subject says "add stmt needs the owning
   function", not "needs a name". A non-id subject for
   `add stmt` says the subject is the owning function's id
   (with an id-shaped example) and that a statement has no
   name — instead of the generic `el find` hint, which is
   false for this verb. The generic hint is unchanged for
   every other verb; the stmt branch lives at the `add`
   subject seam, not in `parse_id` itself.
2. **The skill page shows the stmt form.** One example
   line beside the test-loop and plain-words blocks:
   `add stmt <fn-id> --body - --at 0`, with the rule that
   the subject is the owning function and a statement has
   no name. The golden page is regenerated; it stays under
   the 150-line ceiling.
3. **Body paths name the file rule.** A nonexistent
   `--body`/`--doc`/`--preamble` path says "no file
   `<path>`; give a file path whose contents are read, or
   `-` for stdin" instead of "`<path>` does not exist".
   The same wording flows through apply lines, which
   prefix it with the line number. The apply-script stdin
   refusal ("a script cannot read stdin; use `--body
   <file>`") is unchanged: with the file rule taught, it
   is sufficient, and letting scripts read stdin would need
   a grammar change (a line's body versus the next command
   is ambiguous) — deferred with cause.

## Alternatives rejected

- **Resolving bare names in the stmt subject** (letting
  `add stmt double` find `double`): it would make the old
  hint true, but it adds surface to the one verb whose
  subject must already exist, and the wording plus the
  example fix the loop without it. Revisit if the loop
  survives this change on the next bench run.
- **Accepting literal `--body` text**: bodies are
  multi-line and argv-fragile; the file-or-stdin contract
  stands, and the message now teaches it.

## Consequences

- Refusal messages are part of the agent interface; these
  wordings are pinned by acceptance tests, as the generic
  hint already was.
- `COMMANDS.md` needs no stmt change (it already says
  `<owner>`); the skill page was the gap, and it is closed
  there.
