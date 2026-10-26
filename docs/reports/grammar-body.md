# Body grammar and body tree

Status: implemented. `cargo test -q --lib` is green (317 passed),
`cargo clippy --all-targets -- -D warnings` is green, and
`python3 tools/check_line_budget.py` prints `line budget: ok`.

## What changed

| File | Lines | Role |
|---|---:|---|
| `src/core/model/body.rs` | 20 | `Body` / `BodyStmt`, derives `Debug, Clone, PartialEq, Eq, Default` |
| `src/core/text/grammar.rs` | 148 | `parse`, `Error`, `Display`, block builder, pending-statement loop |
| `src/core/text/grammar/line.rs` | 133 | one logical statement -> one text: delimiter state, repair, semicolons |
| `src/core/text/grammar_tests.rs` | 191 | 23 single-line/tree tests |
| `src/core/text/grammar_multiline_tests.rs` | 83 | 6 multi-line tests, declared from `grammar.rs` via `#[path]` |

Also: `src/core/model/mod.rs` gained exactly `pub mod body;`; `src/core/text/mod.rs`
gained exactly `pub mod grammar;`. No existing test, scanner, balance, split,
terminator, CLI, adapter or app file was touched.

## Seam (a deviation from the brief's CREATE ONLY list)

The first complete single-file `grammar.rs` was 278 lines. After removing one
duplicated pass it was still 220, over the 200 ceiling. Rather than line roulette
(compressing formatting or deleting reasoning to fit), I split on responsibility:

- `grammar.rs` owns lines -> tree: `Error`, `parse`, the block builder, and the
  pending-statement loop.
- `grammar/line.rs` owns one logical statement -> one text: delimiter state,
  unterminated/wrong-closer refusals, repair, and the semicolon policy.

This adds `src/core/text/grammar/line.rs`, one file beyond the brief's list. All
files stay inside the 25-200 band. If the extra file is not acceptable, the only
alternatives are to merge and exceed 200, or to move the line policy to an owner
you name. The seam is real, not a line-range split: nesting and per-line
punctuation change for different reasons. The multi-line tests live in their own
file for the same reason, so no test file crosses the ceiling.

## Contract: single line and multi-line

A logical statement is accumulated from physical lines:

- Each line is trimmed; blank lines are skipped. Non-blank lines are joined with
  `
`, so the newlines survive in `BodyStmt.text`.
- A line ending in `{` opens a block only when everything before the `{` is
  balanced. `if x {` opens a block; `take(Foo {` has an open `(`, so it continues
  the expression instead.
- A line that is exactly `}` closes a block only when no logical statement is
  pending. Inside a pending expression, `}` is part of the expression.
- While the accumulated text has an open `(` or `[`, the next physical line is
  appended. When the brackets close, the statement is complete.
- At completion, or at end of body, missing `)` and `]` are appended at the end of
  the logical statement. The semicolon policy runs exactly once on that completed
  text. A missing final `}` for a block is still the renderer's job.

## RED (test-first, before implementation)

First pass, single-line grammar. The tests were written against an interface stub
whose `parse` returned `Ok(Body::default())` (the documented interface-first
exception); every behavioural test failed on an assertion, not a missing name:

```text
assertion `left == right` failed
  left: Body { statements: [] }
 right: Body { statements: [BodyStmt { text: "if x {", children: [BodyStmt { text: "a()", children: [] }] }] }

assertion `left == right` failed
  left: Ok(Body { statements: [] })
 right: Err(Unterminated { line: 1 })

test result: FAILED. 2 passed; 21 failed; 0 ignored; 0 measured; 283 filtered out
```

Multi-line pass, after adding the tests and before the accumulator:

```text
running 28 tests
... a_call_spans_lines_until_its_parenthesis_closes ... FAILED
... an_array_of_structs_spans_lines_until_its_brackets_close ... FAILED
... a_multiline_expression_inside_a_block_keeps_its_newlines ... FAILED
... a_multiline_statement_gets_one_semicolon_after_it_completes ... FAILED
... a_brace_inside_an_open_call_does_not_open_a_block ... FAILED
... a_parenthesis_closes_at_the_end_of_its_logical_statement ... FAILED

thread '...a_call_spans...' panicked: body should parse: Unbalanced { line: 4 }
thread '...a_parenthesis_closes...' panicked: body should parse: Unbalanced { line: 2 }

test result: FAILED. 22 passed; 6 failed; 0 ignored; 0 measured; 288 filtered out
```

No test was weakened.

## Superseded tests (contract change, not weakening)

Two tests from the first pass asserted the old line-at-a-time behaviour. The new
contract supersedes them, so their expectations changed:

- `an_unclosed_parenthesis_is_closed_at_the_end_of_its_statement` became
  `a_parenthesis_closes_at_the_end_of_its_logical_statement`. Its second assertion
  used to expect two statements from `call(a` + `b()`; it now expects one logical
  statement, `call(a\nb)`, because the open `(` continues. The single-line
  `call(a` -> `call(a)` repair assertion is unchanged.
- `an_opener_closes_its_parenthesis_before_the_block_brace` became
  `a_balanced_prefix_before_a_block_brace_opens_a_block`. The old input
  `if foo(a {` has an unbalanced prefix and is no longer a block opener; the
  replacement tests the positive rule with `if foo(a) {`. The negative case is
  covered by `a_brace_inside_an_open_call_does_not_open_a_block`.

## GREEN

```text
cargo test -q --lib grammar
running 29 tests
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 288 filtered out

cargo test -q --lib body
running 27 tests
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 289 filtered out

cargo test -q --lib
running 317 tests
test result: ok. 317 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The 27 `body` tests are the grammar tests whose names mention a body plus the
existing renderer body tests; nothing was changed there.

## Clippy

```text
cargo clippy --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.46s
exit 0
```

`rustfmt --check` is also clean for the five files above.

## Line budget

The five files are 20, 148, 133, 191 and 83 lines, all inside the 25-200 band.
`python3 tools/check_line_budget.py` prints `line budget: ok`. (During concurrent
work it was briefly red on another agent's `src/app/verbs.rs`; that file is not
part of this change and is green at hand-off.)

## Decisions and limitations

1. A closing line must be exactly `}`. `};`, `},` and `} // note` are treated as
   statements and refused as unbalanced. The contract only legalises the exact
   line.
2. A block opener needs a balanced prefix. `if x {` opens; `take(Foo {` continues
   as an expression because the `(` is open. A top-level `let p = Point {` has a
   balanced prefix, so it opens a block; a multi-line struct literal is supported
   when nested in `(` or `[`, as the array test shows.
3. A line that is exactly `}` inside a pending expression is appended, not treated
   as a block closer. Otherwise an array of blocks would close its enclosing block
   by accident.
4. Each physical line is trimmed and non-blank lines are joined with `
`. Blank
   lines inside a multi-line statement are skipped, consistent with the single-line
   rule.
5. Accumulation is driven by open `(` and `[`, not by `{`. An unclosed `{` with no
   open `(`/`[` is `Unbalanced` unless it is a block opener.
6. An unterminated literal or block comment is an error as soon as the scanner
   sees it, so a string or block comment that spans physical lines is not
   accumulated. Accumulating those would require deferring the unterminated error
   until end of body; the follow-up scoped continuation to open brackets.
7. At end of body, a pending statement with open `(`/`[` is repaired by appending
   the missing closers. This preserves the old single-line behaviour
   (`call(a` -> `call(a)`) while allowing the same text to continue when more
   lines follow.
8. `else` is refused whenever a line's first word is `else`, optionally after a
   leading `}`. Inside a pending multi-line expression the `else` check does not
   apply, because there is no tree block to attach it to; it is part of the
   expression text.
9. `first_word` is the leading identifier-like token, not a whitespace split:
   `return(x)` counts as `return`, `return_value` does not.
10. A statement already ending in `;`, `{`, `}` or `,` is never double-terminated.
11. Purity: no clocks, files, environment or panics on input. The only slice is
    taken after `strip_suffix('{')`; all indexing is guarded.

## Not done

- No ADR was added. This is a new module inside `core::text`, not a new boundary or
  public interface outside the crate; the brief did not list an ADR in scope. If the
  grammar later becomes a boundary, it will need one.
- The unrelated pre-existing failures were not investigated or fixed, as
  instructed. For the record, `cargo test -q --lib` was already green at the start
  of this task (283 passed); the failures the brief mentioned appear to be the
  untracked acceptance test's compile errors, not lib tests. Both were left alone.
