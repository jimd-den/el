# ADR-0036: The renderer collapses empty blocks to one line

## Context

`el fmt --check` (PR2) reports formatting evidence, and el's own generated
output failed it: the renderer wrote every block-taking item as three
lines (`{declaration} {`, body, `}`), so an empty `fn main()` came out
multi-line while rustfmt wants `fn main() {}`. The loop would then spend
turns "fixing" formatting el itself created.

rustfmt 1.99.1 was checked kind by kind on multi-line empty input: empty
fn, struct, impl, trait, test, enum, and module blocks all collapse to
one line, while a non-empty fn stays multi-line. The collapse rule is
general for empty blocks, not main-specific.

## Decisions

1. `render_item` probes the body before laying it out: it renders the
   body into scratch output, and when nothing was written it emits
   `{declaration} {}` on one line. A body holding only a comment still
   renders lines, so it stays multi-line, exactly as rustfmt keeps it.
   The probe is kind-agnostic, so no kind list can drift from rustfmt.
2. `fn_budgets` no longer assumes a block span covers at least two
   lines. A one-line span has a zero-line body, computed with
   saturating subtraction. The old `lines - 2` underflowed on the new
   one-line form and panicked the `add` transaction; the other span
   consumers were audited and already handle single-line spans
   (`ls` counts `last - first + 1`; `show`/`source_of` slice with
   saturating bounds).

## Consequences

- el-generated crates are rustfmt-clean for empty blocks, so `fmt
  --check` evidence no longer reports el's own output as a failure.
- Non-empty bodies render exactly as before; only the empty case
  changes shape.
- The renderer still never decides whether text is legal Rust; the
  one-line form is a layout rule, verified against rustfmt, not a
  parse.
