# 0031 — Plain-words flags for `add`

Status: accepted · Date: 2026-10-27

## Context

Small models generate sentences fluently and stumble on sig syntax:
keyword order, parentheses, the `->` arrow, where visibility goes.
The bench showed the vis-in-signature refusal correcting in one
turn, but every sig is still a small assembly task the model must
get exactly right, and nothing tells it when a return type is
needed at all. The order behind this branch: the command language
should let the model not think about sig syntax, and should say
when a return type is needed.

## Decision

1. **Word flags on `add`, assembled mechanically.** `--takes` and
   `--returns` for `fn` and `test`; `--type` and `--value` for
   `const` and `static`; `--type` for `type`. Assembly is string
   joining at the one seam every add path already funnels through
   (`head::signature`), so the CLI, `batch` and `apply` all get it:
   `fn greet(name: &str) -> String`, `const LIMIT: usize = 10`,
   `type Id = u32`. Nothing is parsed and nothing is validated
   beyond emptiness; legality stays the gate's verdict, as ever.
2. **Either `--sig` or the plain words, not both.** Mixing them is
   a usage refusal naming the rule. `--sig` remains the escape
   hatch for everything the flags cannot say.
3. **Defaults.** `fn` and `test` without `--sig` and without words
   are `fn name()` — as before for `fn`, and a fix for `test`,
   which previously fell through to `test name` and died in a
   `BadSignature` refusal. `const`, `static` and `type` without a
   type still refuse, but the message now names `--type`
   ("add const needs --type (or --sig); its type is not
   inferable") instead of only `--sig`.
4. **The return-type note.** On an add of `fn` or `test` without
   `--sig` and without `--returns`, when a body was given and its
   last statement does not end with `;`, the add deserves a note:
   "no --returns given, and the body ends in a value; if that
   value is the result, declare a return type". It is a note,
   not a refusal, deliberately: a `()`-valued tail such as
   `println!` is legal without a return type, and a body ending
   in `return` is left to the gate. It names the need, not a
   command, because the fix after the fact is `edit --sig` while
   the fix beforehand is `--returns`. The note speaks only
   where it adds to the diagnosis: beside a gate rejection whose
   diagnostics include E0308, or when the compiler was skipped
   (`--fast`). On a checked success it would only fire for
   `()`-valued tails, which is noise, so it stays silent there.
   Correction to ADR-0030, learned while building this: `cargo
   check` typechecks test bodies — a test with a non-`()` tail
   is E0308-rejected at add time, not silently kept. Only
   assertion-failure red commits; type-error red refuses. The
   note's E0308 path therefore covers tests too. The note is
   suppressed by `--quiet` like every other human note, and it
   fires for the CLI `add` and for `batch` items; `apply` lines
   do not carry it yet (their changes are built without the
   request beside them). For a batch the E0308 check is
   crate-wide, not per item, which is an accepted approximation:
   the note is conditional, and the batch case is rare.
5. **Surface.** The four flags join the skill page's flag lines
   plus a plain-words block with examples, and COMMANDS §4.2
   documents the flags, the either/or rule and the note.

## Alternatives rejected

- Verb or kind synonyms (`create`, `function`): synonyms add
  surface without removing thought. The thought is in the sig,
  and the flags remove it; the verb `add` is already plain.
- A sentence parser (`add function answer that returns u32`):
  English parsing is strictly harder than sig parsing, and a
  misread sentence would fail in ways no refusal could name.
  Word flags keep the grammar exact and the words familiar.
- Refusing a missing return type: el cannot typecheck, so it
  cannot know a tail value needs a type. A conditional note says
  what the model needs to hear without ever blocking a valid
  program.
- Word flags on `edit`: deferred. `edit --sig` stays the way to
  redeclare; the same flags can extend it later.

## Deferred, with cause

`struct`, `enum`, `trait` and `impl` get no word flags here.
Their heads are structural (fields, variants, methods), and no
existing test pins how a struct's fields are even expressed, so
their flag shapes need their own design instead of a guess.
`--fields` and `--variants` are the obvious next shapes once
that design exists.

## Consequences

`Add` gains four optional fields; batch items gain the same four
JSON fields; `head::signature` is the single assembly point.
The skill page grows about seven lines and stays under its
150-line ceiling. The test default fix (decision 3) is a
behaviour change to `add test` without `--sig`: it now succeeds
instead of refusing.
