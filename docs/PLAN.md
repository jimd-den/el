# Plan — from an empty directory to a tool that holds itself to its own rules

> Ordering principle (`AGENTS.md` §2.3): one acceptance test at the outermost honest
> boundary names the use case; then descend, inside-out, red→green→refactor, until it flips.
> Each milestone below **is** an acceptance test and the units it forces.

---

## 1. Current state

```text
~/el               (this repository root)
├── AGENTS.md              the governing standard (§0 filled for `el`)
├── Cargo.toml             zero dependencies, edition 2024
├── Makefile, tools/       `make check`: budget, test, lint, typecheck
├── docs/                  this design set
├── src/core/model/        id, kind, element, stmt, catalog, change (types only)
├── src/core/text/         split: scanner, boundary policy, lossless pieces
└── src/{app,cli,adapters,ports}/   empty stubs — nothing behind them yet
```

State as of 2026-10-26, and the only state a reader may trust: `make check` is green.
M0–M6 are real: the harness bites on planted violations, the model round-trips, the text
toolkit and renderer are property-tested, `plan` refuses with nothing written, the store
verifies every read, and UC-1 builds and runs a five-command crate through the real binary.
M7 is real: the read surface, statement insert/edit/rm/reorder, anchors, the split
policy, and comment attachment. M8 is real: history verbs, the attic, per-sequence
catalog snapshots, diff, and doctor recovery. M9 is real on the gate branch: the
compiler delta refuses a change that adds an error. M12 is real on this branch:
`apply`, `skill`, `explain`, `check`, `init`, the refusal hint and the first
actionable diagnostic. M10 and M11 are still designs, not claims. M13 is the
release harness: footprint and the SLM benchmark.

The M6 handoff branch was intentionally red for one turn: two files were over the line
ceiling and three parser tests failed. That is now corrected — `cli::args` and
`adapters::store::journal` are the two splits, the parser reads `words[1]` and `mv`'s
three words, and the one test argv that was wrong is written as argv. Every line of
implementation is demanded by a test written first.

---

## 2. Milestones

| # | Milestone | Flips | State 2026-10-25 |
|---|---|---|---|
| M0 | Harness and project constants | `make check` exists and fails loudly on a planted violation | ✅ green, violations proven |
| M1 | The model | a catalog can be built, addressed and round-tripped | ✅ |
| M2 | Text toolkit | bodies split losslessly, JSON is read and written | ✅ |
| M3 | Rendering | a catalog becomes Rust files and a span map | ✅ |
| M4 | Plan and L0 | an impossible change is refused with nothing written | ✅ |
| M5 | The store | every projection is content-addressed and restorable | ✅ |
| M6 | Transaction and first verbs | **UC-1**: a program built from five commands | ✅ |
| M7 | Statements and the read surface | **UC-11/UC-12**: one line changes, nothing else moves | ✅ |
| M8 | History verbs | **UC-3**: review and rollback | ✅ |
| M9 | Compiler gate | **UC-2**: told before commit that it would not compile | ✅ |
| M10 | Semantic layer | **UC-9/UC-10**: RA rename, refs, quick fixes | ☐ stub |
| M11 | Search and doc | **UC-5**: std lookup without leaving the terminal | ☐ stub |
| M12 | Agent surface, dogfood, polish | **UC-6/UC-7**: batch, skill, self-compliance | ☐ stub (batch + find shipped as `el batch`/`el find`, ADR-0027; `apply` script form and skill still open) |

A milestone is ✅ only when its acceptance test exists, was seen red, and now passes. A module
that exists but decides nothing is ☐, whatever the commit message says.

Anything before M9 ships with `--fast` semantics and says so; the headline promise is not
claimed until M9's gate is green.

---

### M0 — Harness and project constants

**Goal.** A repository where a red test is the cheapest thing in the world, and where the
tool's own rules are already executable.

**Deliverables.**

- `Cargo.toml` with **no** `[dependencies]`, `rust-version = "1.99.1"`, `edition = "2024"`.
- `Makefile`: `budget`, `test`, `test-slow`, `lint`, `typecheck`, `check`.
- `tools/check_line_budget.py` (≤ 200 lines, counts itself), with
  `tools/line-budget-baseline.txt` present and **empty**.
- `.pre-commit-config.yaml` wiring the budget check.
- `tests/meta/line_budget.rs`, `tests/meta/zero_dependencies.rs`,
  `tests/meta/core_is_pure.rs` (initially vacuous, then load-bearing from M1).
- `el.policy` with the `AGENTS.md` defaults (`ceiling = 200`, `deps = none`, `forbid = unsafe`,
  `require_preamble = true`, `gate = check`).
- **`AGENTS.md` §0 refilled for this repository** (open decision `D-0`): `PROJECT` becomes the
  one-line description of `el`; every command stays the same so the gate is unchanged; ỌKỌ's
  original §0 is preserved verbatim in `docs/adr/0003-project-constants.md` so nothing is lost.
- `docs/reports/prior-art.md` committed (the research log this design rests on).

**RED first.** `tests/meta/line_budget.rs` must fail while a deliberately 210-line fixture
exists in `tests/fixtures/`, then pass once the fixture is replaced by the real checker output.

**Done when.** `make check` exits 0 on an empty `src/`, and exits non-zero when `src/probe.rs`
is created at 201 lines. Both runs shown.

---

### M1 — The model

**Goal.** Everything later depends on this being right and small. Pure data, no I/O.

**RED first.** `tests/acceptance/add_returns_stable_id.rs` — cannot pass yet, because there is
no command. Descend immediately to units:

| Unit test (red first) | Behaviour |
|---|---|
| `id_assigned_and_never_reused` | counter survives delete; `el-0011` is not reissued |
| `id_survives_rename_and_move` | FR-1 |
| `alias_is_unique_across_kinds` | FR-2 |
| `catalog_round_trips` | property: encode(decode(x)) == x (FR-3) |
| `kind_rules_are_total` | every `Kind` answers: needs name? needs body? can nest? |
| `statement_tree_nests` | FR-62 |

**Files.** `core/model/{id,kind,element,stmt,change}.rs`, `core/model/catalog/{mod,edit,identity,codec}.rs`,
and `core/text/tsv.rs` (the escaping the catalog form needs, ahead of M2).

**Done when.** `cargo test -q -- --test-threads=1` green in < 2 s; no file over the 200-line
ceiling. Amended 2026-10-25: this milestone asked for 120 lines, which forced a fourth split of
`catalog` into fragments too small to hold their own tests. The ceiling is 200 (AGENTS.md §4.1);
120 was an estimate made before the shape was known.

---

### M2 — Text toolkit

**Goal.** Two lexical utilities and one JSON reader, each with a property that makes the rest
of the system honest.

**RED first.** `tests/properties/split_round_trips.rs` — for the fixture corpus of 40 bodies,
`join(split(b)) == b`, and the splitter must **refuse** (`None`) on the adversarial cases:
raw strings containing `}`, nested macros with unbalanced-looking tokens, labelled loops,
`'a` lifetimes vs `'a'` chars, block comments containing `*/` inside a string.
This test is red before `split.rs` exists; the refusal cases are what make it hard.

| Unit | Property |
|---|---|
| `text::scan` | every delimiter index returned is outside strings, chars, raw strings, comments |
| `text::split` | lossless + idempotent (FR-65) + refuses ambiguity (FR-64) |
| `text::json` | round-trip on fixtures; rejects trailing commas and unquoted keys loudly |
| `text::tsv` | escaping round-trip; no field may contain a tab or newline after encode |

**Done when.** `make check` green; the splitter's refusal list is a checked-in fixture so a
future "improvement" that starts guessing is caught.

**Status 2026-10-25: ✅ — with one honesty note.** The splitter already existed when this
milestone was opened, so `split_round_trips.rs` was **green on first run**: it is a pin on
behaviour that was already there, not a test that drove it. It earns its place by covering what
the splitter was never checked against — 40 adversarial bodies (raw strings holding `}`, macros
with unbalanced-looking tokens, labelled loops, lifetimes beside char literals, `*/` inside a
delimited string) and 9 refusals, each file named for why it cannot be split. If the scanner is
ever "simplified", this is what fails first.

The new production code here is `text::json`, written test-first and proved by mutation rather
than by a claim: removing the trailing-text check fails `trailing_text_is_refused` and
`a_bad_number_is_refused`; decoding `\n` as anything else fails `a_string_round_trips`.
`lines.rs` is deliberately not implemented — nothing calls it yet, and the string helpers it
would hold are M3's first need, not M2's.

---

### M3 — Rendering

**Goal.** `render(catalog) → files + span map`, deterministic, byte-stable.

**RED first.** `tests/properties/render_is_deterministic.rs`: two catalogs built by inserting
the same elements in different orders render to identical bytes (FR-13). Then
`tests/acceptance/every_rendered_module_compiles.rs`: for five catalogs, the rendered crate
passes `cargo check` (this one is `#[ignore]`-free but lives in `integration/`).

**Files.** `core/render/{mod,rust,body,manifest,spans,style,tree}.rs`.

**Done when.** The rendered `src/main.rs` for a two-module catalog is byte-identical to the
checked-in golden file, and changing one statement changes exactly one line.

**Status 2026-10-25: ✅ — with two corrections the plan did not anticipate.**

*The model had nowhere to put a parameter list.* `COMMANDS.md` offers
`--sig 'fn run(p: &Plan) -> Verdict'` and `Element` could not hold it, so "rendered files
compile" would have been a claim about `fn foo() {}`. An element now carries `head` (the text
after its identifier), which is `21f38fc`, and the catalog's `E` record carries it too
(ADR-0015 amendment). Text, not a typed signature: `el` remembers characters, the compiler
judges them (ADR-0008).

*Five catalogs do not compile on their own.* Three real defects came out of the compiler, not
out of reading the code:

| Symptom | Cause | Rule it produced |
|---|---|---|
| `enum Color;` lost its variants | `needs_body()` was the wrong question | `Kind::takes_block()`: brace or semicolon is a separate question from "may own children" |
| `E0603: module plan is private` | a generated `mod` line hid a `pub fn` | a `mod` line is as visible as the widest thing beneath it (ADR-0016) |
| a block statement never closed | the model records children, not punctuation | the renderer emits the `}`, because `text::split` already decided where the statement ended |

**What drove what.** `render_matches_golden` and `mod_tree_is_generated` were red first —
the latter's new case failed with *"root does not re-export core"* before the visibility rule
existed. `render_is_deterministic` was **green on first run**: the renderer sorted by content
from the start, so it is a pin, and it was proved by mutation instead of by claim — deleting
`node.items.sort()` fails two of its cases and `tree_tests::items_are_in_id_order`, which was
strengthened for exactly that reason. Deleting `spans::Collector::finish`'s sort fails
`spans_sort_by_path_then_start`; making `opens_block` return `false` fails two body tests and
the integration crate.

The golden file changed once, and not because the renderer moved: `greet.rs` holds a `pub fn`,
so its `mod` line had to become `pub mod greet;` or the crate would not compile. A golden
recorded from output pins bugs; this one was changed because `cargo check` refused the
alternative, and `tests/fixtures/golden/two_modules/README.md` says so line by line.

---

### M4 — Plan and L0

**Goal.** An impossible change is refused before anything is written.

**RED first.** `tests/acceptance/model_violation_writes_nothing.rs`: add a statement that
pushes a module to 201 lines; assert exit 2, the error names `el-NNNN` and the line count,
and the file mtime is unchanged.

| Unit | Rule |
|---|---|
| `plan::apply_element` | add/edit/rm/mv are total functions over `Catalog` |
| `plan::apply_stmt` | cross-body move is a violation, not a panic |
| `plan::validate` | the eleven L0 rules of `GATE.md` §5 |
| `plan::refs` | reference index; rename rewrites, delete reports |
| `plan::policy` | `el.policy` parse, defaults, a policy file that is wrong is an error with its line |

**Done when.** `make check` green; every violation has a distinct code and one test.

**Status 2026-10-25: ✅ — with one boundary correction and two splits the plan did not name.**

*The RED test could not be written as specified, and pretending otherwise would have been the
dishonest move.* The milestone asks `model_violation_writes_nothing.rs` to assert **exit 2** and
an **unchanged mtime**. There is no binary and no store until M5 (`src/main.rs` is
`fn main() {}`), so neither exists to assert. The test therefore proves the property at the only
boundary that is real: the catalog that goes in is byte-identical to the one that comes back
(`encode()` before and after), and the refusal names `el-NNNN` and the computed size. `plan`
takes `&Catalog`, so "nothing was written" is a property of the signature (ADR-0017) and the
test is the witness. **Exit 2, the untouched mtime, and the ≤ 20 ms budget are M5's half of
FR-18 and are not claimed here.**

*The eleven rules needed three files, not one.* `validate.rs` was 279 lines with every rule in
it. The seam was not length but what can answer the question:

| File | Answers | Rules |
|---|---|---|
| `request.rs` | what was *asked for* | cross-body move, denied dependency, removal leaving a reference |
| `validate.rs` | what the catalog *is* | duplicate alias, unknown ref, unbalanced, bad ident, forbidden |
| `rendered.rs` | what the text *would say* | ceiling, function budget, preamble |

`request` runs first because it is free and because its message names ids the user still
recognises. `rendered` was the second split: the ceiling rule needs the two largest extractable
statements to be advice rather than a wall (FR-73), which is a projection question.

*The eleven rules, each with a code and a test.* `Code::all()` is exhaustive by test, and every
code is spelled `model.*` in one `match`, so a rule cannot ship without a name. Along the way
`text::balance` (a body whose delimiters close, ignoring literals) and `text::ident` (where the
identifiers that are *code* are) became reusable, and `plan::refs` is built on the second: a
rename rewrites `apply(p)` and leaves `"apply"`, `// apply` and `applier` alone.

**What was pinned, and how it was proved.** Eight test files went green on their first run after
a stub, which is a pin, not a red. Each was proved by mutation instead:

| Mutation | What failed |
|---|---|
| `unbalanced` fires for every body | 6 tests, including the clean-catalog case |
| the ceiling check is skipped | 4 tests, including both size assertions |
| the function budget counts braces | `an_element_over_the_function_budget_names_its_body_size` |
| a cross-body move is allowed | `a_statement_moved_into_another_body_is_refused` |
| a module always has a preamble | `a_module_without_a_preamble_is_refused_when_the_policy_requires_one` |
| `rewrite` uses `match_indices` | 2 tests: a comment and a string got rewritten, and unscannable text stopped being refused |
| `references` uses `match_indices` | 4 tests, including "a name in a string is not a reference" |

**Two findings the plan did not anticipate.** The preamble rule caught a design question: the
crate root is a file like any other, so `require_preamble` applies to `src/main.rs` too. The
first version of the test passed by asserting a special case; the rule has none, and the test
now declares a root preamble instead. And `plan::refs::occurrences` was first asserted at byte
17 for `"apply; applier; apply"` — arithmetic I got wrong in the test I had just written. The
third occurrence is at 16, which is provable by counting; the expectation was corrected before
the implementation was trusted with it, and the counting is in the test's own line.

**Files.** `src/core/plan/` in nine modules (127, 73, 94, 165, 159, 96, 161, 162, 153 lines)
plus `text/ident.rs` (144) and `text/balance.rs` (105); largest file 181 (`apply_element_tests`).
Gate: 283 tests, `make check` exit 0.

---

### M5 — The store

**Goal.** Snapshots that can be trusted enough to restore from after a crash.

**RED first.** `tests/acceptance/corrupt_object_is_detected.rs`: flip one byte in an object,
read it back, assert a named error and no silent use.

| Unit | Property |
|---|---|
| `history::sha256` | NIST vectors; empty string; 1 MB |
| `history::object` | write→read round-trip; digest verified on read (FR-33) |
| `history::tree` | encode/decode; set diff is symmetric; sorted output |
| `history::entry` | encode/decode; sanitization of tabs/newlines |
| `adapters::fs_local` | atomic write: a kill between tmp and rename leaves the old file intact |

**Done when.** `doctor --rebuild` is not implemented yet, but the store's own round-trip test
proves every recorded entry can be decoded and re-encoded byte-identically.

**Status 2026-10-25: ✅ — the six units landed, and the crash story is split between two layers.**

*The RED test compiles against the real adapter and the real store root, because "we verify on
read" is a claim about bytes that came back off a filesystem.* A fake `Fs` would have proved that
the fake was honest. `corrupt_object_is_detected.rs` writes a blob, flips one bit on disk with
`std::fs`, reads it back, and asserts `env.corrupt` naming **both** digests — plus a control test
that an intact blob reads back byte-identically, without which a reader that always failed would
have passed. `history` stays pure: it frames, sorts, encodes and verifies; it names no file, and
`tests/meta_core_is_pure.rs` is what keeps that true.

**What was pinned, and how it was proved.** All of these went green on their first run, so they
are pins rather than reds. Each was killed by mutating the code it is supposed to defend:

| Mutation | What failed |
|---|---|
| `object::verify` returns `Ok` for anything | 2 tests: a flipped byte became silence |
| `object::path` drops the two-character fan-out | 2 tests, including the path shape |
| `entry::sanitize` becomes the identity | `free_text_is_sanitized_so_a_record_cannot_forge_the_next_one` |
| the 200-character truncation is removed | `a_long_message_is_truncated_to_two_hundred_characters` |
| `Tree::new` stops sorting | 2 tests: arrival order leaked into the encoding |
| `Tree::diff` stops reporting `changed` | `the_diff_names_what_was_added_removed_and_changed` |
| `Tree::new` accepts a duplicate path | 2 tests, including the hand-edited tree |
| `Fs::write` writes the target directly, no staging | `a_write_that_cannot_stage_leaves_the_previous_bytes_intact` |
| `resolve` stops refusing `..` and absolute paths | `a_path_that_would_leave_the_store_is_refused` |
| `list` stops excluding `tmp/` | `staging_files_are_never_listed_as_store_content` |
| `list` reverses instead of sorting | `a_store_written_in_two_runs_lists_the_same_way_each_time` |
| `remove` of an absent path becomes an error | `listing_a_store_that_was_never_written_is_empty_and_not_an_error` |
| `read` of an absent path becomes an error | the same test |

**The crash contract is simulated where a crash would actually stop.** Killing a process between
`tmp` and `rename` would be a flaky test; making `tmp/` unwritable is not. The old bytes survive,
which is the negative half of `I3`; the positive half — "the new bytes are visible" — is the
ordinary write test. The *ordering* across files (objects, then trees, then the entry) was
not claimed here; M6's transaction now owns it, and the tests in `app::transaction_tests`
prove the refusal and recovery paths.

**One test was wrong in a way the mutation found.** `a_path_that_would_leave_the_store_is_refused`
asserted that `../escape` was never created, but looked for it *outside* the directory the test
cleans. The mutation created the file, the test failed as designed, and the next honest run then
failed too — the assertion was measured against a dirty world. The store root now lives inside the
cleaned directory, so the test owns everything it makes a claim about.

**Files.** `src/core/history/` in four modules (144, 84, 137, 174 lines) plus three test siblings;
`src/ports/fs.rs` (109) and `src/adapters/fs_local.rs` (155). Largest file 174. Gate: 334 tests,
`make check` exit 0.

**Accepted debt, recorded in ADR-0018.** No `lock`, no `inflight`, no drift scan, no `rename` on
the port, and verify-on-read is a convention with one call site — the signal that it rotted is a
second hand-written `verify`, and the fix then is a `Store` in `adapters/`, not another
convention.

---

### M6 — Transaction and first verbs — *first usable release*

**Goal.** `el new`, `el add`, `el edit`, `el rm`, `el mv`, `el run`, `el log` with L0
gating, the lock, the inflight marker, drift detection, and the JSON envelope. Bodies
are parsed by `el`: the author writes intent, the tool supplies closing braces,
semicolons and unbalanced parentheses.

**RED first.** `tests/acceptance/author_builds_program_from_nothing.rs` (UC-1). It runs
the real binary in a temp directory. On the handoff binary it failed at the first
command with `el new failed: Cargo.toml is missing`; both tests in the file are now
green, including a real `cargo clippy -- -D warnings` over the generated crate. On the
way down, these units:

| Unit | Behaviour |
|---|---|
| `core::text::grammar` | a body line tree; braces, semicolons, `(`/`[` repair; `else` refused |
| `app::transaction` | plan before write; inflight; materialize; model gate; restore on refusal |
| `app::projection` | blobs → tree → disk; restore removes what the tree does not name |
| `adapters::store` | `.el/` over `Fs`; verify-on-read; journal rewritten by seq |
| drift | hand edit → exit 4; whitespace edit → re-render + one stderr line |
| `out_json` | envelope matches `COMMANDS.md` §5 exactly (golden) |
| exit codes | 0, 1, 2, 4, 5, 6 each reachable; 3 by the `--strict` lint tier |

**Done when.** UC-1 passes end to end: five commands, `cargo run` prints `hello, world`,
`el log` shows five entries all `gate=model`.

**Status 2026-10-26: ✅ — with the handoff completed and the cuts recorded.**

*The handoff was finished, not hidden.* The two over-budget files were split along the
seams the handoff named: `cli::args` is its own module (141 lines) and `parse` is 166;
`adapters::store::journal` holds the three journal methods (47) and `store` is 188. The
three red parser tests were the two off-by-one reads (`Args::required` returned the verb,
and `mv` was allowed two words when it consumes three) plus one test argv typo: `--sig
fn_greet(--body -` has no space, so `-` became a stray word. The assertion was right; the
argv is now spelled as argv.

*What exists and is tested.* `app::{change,create,load,projection,transaction,verbs}`,
`ports::journal` (the application never names `adapters::Store`), `adapters::store::{journal,port}`,
`core::model::body`, `core::text::grammar`, and a `main.rs` that parses before it touches a
crate, locks before a mutation, and chooses a renderer from the same `Report`. `new` seeds
an `init` entry and then adds the root module as its first ordinary transaction, which is
why UC-1's four mutations produce five entries. `--strict` runs `cargo clippy --all-targets
-- -D warnings`; the default M6 gate is L0 `model`, and L1 arrives in M9.

*Grammar is el's job now.* A `--body` is parsed into a tree before it enters the catalog:
a line ending `{` owns its block, the renderer supplies the closing brace, a non-tail
line gets `;`, a tail expression does not, and unclosed `(`/`[` are closed at the end of
the statement. A logical statement continues across lines while `(`/`[` is still open,
so a multi-line call or struct literal is one statement with its newlines preserved. The
line-oriented grammar deliberately refuses what the model cannot represent: `} else {`, a
trailing comment after `{`, and `};`. This is a real limit, not a claim of a Rust parser.

*The deliberate cuts, now the record.* `init` (and `--force`), `--adopt`, `--cascade`,
`--doc`, `--attr`, `--after`, `add stmt` (M7), `add dep`, `apply`, the inspect verbs,
and the history verbs. Each is refused with `usage.unsupported` when it reaches a verb,
never silently ignored. `--semantic` is M10; `init`/adoption needs the opaque-module
representation ADR-0009 describes, which the catalog does not have yet.

*Evidence.* `make check` green: line budget ok, 319 unit + 31 acceptance + 16 integration
tests, clippy `-D warnings`, `cargo check --all-targets`. UC-1's log lines all contain
`model`. Files: `app/change` 183 + 88, `app/transaction` 185 + 88 + 18 + 19,
`app/verbs` 108 + 105, `app/projection` 123, `grammar` 148 + 133 + 83, `store` 188 + 47 + 87.

---

### M7 — Statements and the read surface

**Goal.** Statement ids everywhere, the budget that makes the ceiling actionable,
and the token-saving read surface: `el ls`, `el tree --statements`, `el show`.
`ls` and `show` had no milestone before this branch; they land here because they
are the same id-and-shape contract as `tree --statements`.

**RED first.** `tests/acceptance/edit_replaces_one_statement_only.rs` (UC-11):
capture all statement bytes, edit one, assert the others are unchanged **and**
that no line numbers moved for the statements above. The read half's RED is
`tests/acceptance/read_verbs_navigate_without_source.rs` and
`read_verbs_filter_and_refuse.rs`; all five tests failed with `"ls"` / `"tree"` /
`"show" is not a verb` before the parser learned them.

| Unit | Behaviour |
|---|---|
| `plan::apply_stmt` | add by anchor, rm, reorder; ids stable across reorder |
| `text::split` | wired into body creation; `split = auto/always/never` honoured |
| `gate::budget` | per-file and per-function budgets; ceiling message lists extractable statement ids |
| `model::stmt` | comments and attributes attach and move with their statement |
| `app::inspect` | `ls` filters, `tree` outline, `show` fields; one `View` for human and JSON |
| ordering | modules by path, items by id, statements by `children_of` (ADR-0020) |

**Done when.** UC-11 and UC-12 pass; `el tree --statements` prints the three-level
tree; `el ls` and `el show` answer from the catalog without opening a source file.

**Status 2026-10-26: ✅ — read surface and statement editing.**

*What landed.* `el ls [--module] [--kind] [--opaque]`, `el tree [--statements]`,
and `el show <id> [--statements] [--source] [--history]`, human and `--json`; statement
ids work in `ls --kind stmt` and `show` too
through the existing envelope's `data` field. `show` is id-only: an alias is
`notfound.element` and exit 6, the same refusal `edit` gives. Ordering is the
renderer's. Read verbs recover an interrupted transaction like `log`; they do not
refuse drift, because they answer from the catalog (ADR-0020).

*Evidence.* `make check` green: 339 unit + 50 acceptance + 16 integration tests,
clippy `-D warnings`, `cargo check --all-targets`. The acceptance tests build a
crate through the real binary, then prove `tree` contains every element id and
`show --source` contains a full body without reading a `.rs` file. The golden
`tree` (with and without `--statements`) and the `ls`/`show` field tests are in
`app::inspect::{tests,show_tests}`.

*Statement editing.* `el add stmt`, `el edit <stmt> --body`, `el rm <stmt>`, and
`el mv <stmt> --after <stmt>` work on one statement, keep every id, and renumber
siblings `0..n`. `split = auto|always|never` is policy: `never` keeps a body opaque,
`always` refuses an unprovable split, `auto` falls back to opaque. Leading comments
and attributes attach to the statement that follows them. `model.bad_stmt` refuses
a body that would become several statements. See ADR-0021.

*Evidence.* `make check` green: 339 unit + 50 acceptance + 16 integration tests,
clippy `-D warnings`, `cargo check --all-targets`. UC-11 edits one statement and
proves the other ids, texts and the line above are unchanged; UC-12 inserts by
anchor and reorders without losing the id; the split-policy test sees `opaque` in
`show` and the `always`/`auto` difference.

---

### M8 — History verbs

**Goal.** Review and rollback, exactly.

**RED first.** `tests/acceptance/undo_restores_bytes.rs` (UC-3): record the digest of every
file at seq 41, make three changes, `el undo 3`, assert byte equality including deletions.

| Unit | Behaviour |
|---|---|
| `app::history` | undo/redo/revert are head moves plus materialize |
| attic | a new change after undo moves the future to `attic/`; `log --attic` shows it |
| `diff` | element-level diff, not line noise (FR-32) |
| `doctor --rebuild` | catalog regenerated from journal equals on-disk bytes (FR-34) |

**Done when.** UC-3 passes; `doctor --repair` recovers a SIGKILLed transaction.

**Status 2026-10-26: ✅.**

*What landed.* `undo [<n>]`, `redo`, `revert <seq>`, `diff [<a>] [<b>]`, `log --attic`,
and `doctor --repair|--rebuild`. Every committed sequence now has a catalog snapshot
under `.el/catalogs/<seq>`; undo/redo install it, revert copies it into a new entry,
and rebuild restores the head's cache byte for byte. A mutation after undo moves the
future to `.el/attic/`; `redo` then disappears, and `log --attic` still reads the old
entries. Diff compares older → newer and names the statement id when both snapshots
exist, falling back to a file line diff otherwise.

*Evidence.* `make check` green: 339 unit + 50 acceptance + 16 integration tests,
clippy `-D warnings`, `cargo check --all-targets`. UC-3 captures every owned file,
deletes `src/tmp.rs` through two mutations, undoes four, and compares bytes; redo
restores the same bytes. Revert's new entry carries the target's `to` digest. Diff
names the edited statement. Rebuild restores a corrupted `.el/catalog` from its
snapshot. Repair is tested with an inflight marker and a half-written file, the state
a SIGKILL leaves.

*Boundary.* A pre-M8 store has no old catalog snapshots; undo to such a sequence is
`env.corrupt` rather than a guessed catalog. `log` still lists journal entries ahead
of `head` until a new mutation moves them, so `head` is the authority. See ADR-0022.

*Real programs (post-M8 fix).* Building a two-module program only with el verbs
found three breaks in what M7/M8 shipped: multi-line blocks refused as
unbalanced, a bare module name becoming a phantom tree node, and `add mod`
with a bare name colliding with itself. Fixed on a branch stacked on M8:
L0 now judges balance over a statement subtree (ADR-0023), and module
references are normalized to one file path at the command-line boundary.
Acceptance: `tests/acceptance/real_program_builds_with_blocks_and_modules.rs`
— two modules, multi-line `if`/`for`, a cross-module call, `el run` green.
M7/M8 stay ✅; this is a fix to them, not a new milestone, and statement
editing of blocks is what it unblocks. Deliberate boundary kept: a childless
block opener still refuses (write an empty block single-line).

---

### M9 — Compiler gate — *the headline promise*

**Goal.** The tree either compiles or is not committed.

**RED first.** `tests/acceptance/gate_reports_only_new_errors.rs` (UC-2): a crate with one
pre-existing error, a change that introduces one more; assert `new: 1, existing: 1`, exit 3,
and that the projection is restored.

| Unit | Behaviour |
|---|---|
| `gate::diagnostics` | cargo JSON → `Diagnostic` from checked-in fixtures; unknown reasons ignored |
| `gate::delta` | fingerprints ignore line numbers; span map attributes to statement ids |
| rejection path | restore is byte-exact; journal records `gate=rejected`; head unchanged |
| `adapters::cargo` | timeout kills and restores; exit 101 is not a verdict |
| `--dry-run` | identical pipeline, always restores |

**Done when.** UC-2 passes; `el add fn` on a 30-module crate runs in ≤ 2 s warm
(`tests/integration/gate_latency.rs`, advisory).

Shipped 2026-10-26 (PR against `feat/rewrite-message`): UC-2 acceptance test
`tests/acceptance/gate_reports_only_new_errors.rs` passes; the timed gated add
measures 0.82 s warm on a 30-module crate. Decisions in ADR-0026.

**This is the milestone that makes the tool worth installing.** Everything before it is a
convenience; everything after it is a bonus.

---

### M10 — Semantic layer

**Goal.** `rust-analyzer`, out of the box, optional and honest.

**RED first.** `tests/acceptance/refs_degrades_without_ra.rs`: with `$EL_RA` pointing at
nothing and `PATH` stripped, `el refs` exits 0, returns catalog references, and prints one
line saying which source it used. Then `rename_uses_rust_analyzer.rs`: with RA present, rename
a function called from two modules and assert both call sites changed and `cargo check` passes.

| Unit | Behaviour |
|---|---|
| `adapters::ra::frame` | Content-Length framing round-trip, partial reads, oversized header |
| `adapters::ra::client` | initialize handshake against a scripted fake LSP server (a test binary) |
| `adapters::ra::edit` | WorkspaceEdit all-or-nothing; an edit outside span → refusal with file:line |
| `adapters::ra::session` | timeout kills; stderr quarantined; spawn count is 0 for non-RA commands (FR-60) |
| L3 | diagnostics normalize to the same `Diagnostic`; delta shares code with L1 |

**Done when.** UC-9 and UC-10 pass, and the fast suite still touches no real process.

---

### M11 — Search and doc *(optional; cut first)*

**Goal.** Std and crate lookup from the terminal, cached, zero network code in the binary.

**RED first.** `tests/acceptance/offline_never_fetches.rs`: warm the cache, empty `$EL_FETCH`
to a script that fails, `el search Vec --offline` still answers; a cold cache exits 4 with the
exact command that would warm it.

| Unit | Behaviour |
|---|---|
| `search::index` | parse the std item index into (path, summary) |
| `search::result` | rank, dedupe, limit; deterministic order |
| `adapters::curl` | `$EL_FETCH` override; missing fetcher → exit 4 |
| `adapters::cache` | URL digest, 7-day TTL, atomic write, corrupt entry re-fetched once |

---

### M12 — Agent surface, dogfood, polish

**Goal.** The tool an agent can drive without reading its source, holding itself to its own
policy.

**RED first.** `tests/acceptance/apply_is_atomic.rs` (UC-6): a five-change batch where the
fourth is invalid; assert none of the five landed, one journal entry (rejected), exit 2.

Shipped ahead of the rest of M12 (ADR-0027): `el batch` (the batch parser, one
transaction, all-or-nothing — JSON input rather than the `apply` script form,
which remains unbuilt) and `el find` (name resolution without the whole-crate
dump). The acceptance tests are `batch_adds_are_atomic.rs` and
`find_resolves_names.rs`; the UC-6 shape is covered by the gate-rejection and
plan-failure cases there.

| Unit | Behaviour |
|---|---|
| `el apply` | batch parser, one transaction, all-or-nothing |
| `el skill` | generated from `cli::table`, ≤ 150 lines, pinned by a golden test |
| `el explain` | preamble + docs + callers + tests + history |
| dogfood | `el check` on `el` is part of `make check`; `el.policy` applies to this repo |
| footprint | `tools/footprint.sh` reports a stripped binary ≤ 1.5 MB |

**Done when.** `make check` green with `el` checking `el`, and `docs/reports/m12-dogfood.md`
records the first session where `el` edits its own source. Evidence: `el init` adopted 171
files, `el check` reported zero diagnostics, and `el edit el-0325` rewrote
`src/core/text/tsv.rs` through the gate. The report lists the three defects the session found.

---

### M13 — Release: the footprint and the measured loop

**Goal.** The two claims M12 leaves open are measurable: the binary is small enough to
install, and an agent can be run against the tool without hand-holding.

**RED first.** `tests/meta_slm_harness.rs`: exactly ten tasks, each with a non-empty prompt, a
check script and a solution script, and a runner that names its model.

**Deliverables.**

- `tools/footprint.sh` — a stripped release binary, `FOOTPRINT_LIMIT` default 1 500 000 bytes.
- `tests/bench/slm/` — ten tasks over one fixture crate; each task has a prompt, a check and a
  solution. `tools/slm-bench.sh --solution` validates the harness without a model; a real run
  names the model and records its result.
- `docs/reports/m13-slm-bench.md` — the harness, the method, and what is deliberately not
  claimed until a model is run.

**Done when.** `tools/footprint.sh` exits 0 (measured 1 184 664 bytes on 2026-10-27), all ten
solutions pass their checks, and the meta test is green. A benchmark *result* is not claimed:
the harness is the deliverable, the model run is the next step. Evidence:
`docs/reports/m13-slm-bench.md`; the self-test is `tools/slm-bench.sh --solution`.


---

## 3. Gate for every milestone

```bash
cargo test -q -- --test-threads=1     # fast, < 10 s
make check                            # budget + test + lint + typecheck
make test-slow                        # real cargo, real rust-analyzer, temp dirs
python3 tools/check_line_budget.py    # explicit, even though check runs it
```

A milestone is not done until the acceptance test is green **and** the fast suite is under ten
seconds. Speed of the loop is the asset everything else is bought with (`AGENTS.md` §1).

---

## 4. Sequencing rationale

- **Model before render**: rendering is a projection *of* the model; getting the model wrong
  twice is the expensive mistake.
- **Render before plan**: the plan layer's output is a projection, and seeing rendered Rust in
  a test at M3 makes every later test readable.
- **Store before transaction**: the transaction's failure path *is* the store. Building the
  restore path before the happy path means it is tested from its first use.
- **L1 after history verbs**: a rejection must be able to restore. Shipping the gate before
  `undo` works would mean the first bad change is unrecoverable.
- **RA after L1**: RA is a second opinion. The first opinion has to exist first.
- **Statements before the compiler gate**: statement granularity is what makes the delta
  fingerprint (and therefore the gate's usefulness) precise.

---

## 5. Delegation plan (`AGENTS.md` §6)

The parent keeps: the model, the transaction, the gate verdict, and every boundary decision.
These are the parts where a plausible-looking mistake is invisible until much later.

| Slice | Owner | Why it is separable | Files |
|---|---|---|---|
| M0 harness | subagent | mechanical, self-verifying | `Makefile`, `tools/*`, `tests/meta/*` |
| M2 text toolkit | subagent | pure functions with properties, no context needed beyond the property | `core/text/*` |
| M3 output renderers | subagent | golden files are the spec | `adapters/out_*` |
| M11 search/cache | subagent | independent of the model | `search/*`, `adapters/{curl,cache}` |
| M10 LSP framing | subagent | the protocol is external and documented; a scripted fake is the spec | `adapters/ra/{frame,client}` |
| Model, plan, transaction, gate, history | **parent** | integration risk | `core/{model,plan,gate,history}`, `app::mutate` |

Rules: one writer per path, no child edits a test the parent is making green, every brief
carries the `STANDARDS` block, and the parent runs `make check` plus the line budget on
everything returned.

---

## 6. Cut list, in order

When time runs short, cut from the top. **P1, P2 and P3 are never cut.**

| # | Cut | What the user loses | Why it is first |
|---|---|---|---|
| 1 | M11 search/doc | std lookup in-terminal | nothing else depends on it |
| 2 | `--strict` (L2) | clippy in the gate | L1 already blocks non-compiling changes |
| 3 | `el fix` | quick-fix application | rename and refs are the high-value RA features |
| 4 | `el apply` batch | multi-edit transactions | single verbs still work |
| 5 | L3 `check --semantic` | fast semantic verdict | L1 is authoritative anyway |
| 6 | Statements (M7) | per-line addressing | the model degrades to opaque bodies, not to broken |
| 7 | `el init` adoption | onboarding an existing tree | `new` still works |

Never cut: the gate's rejection path (M9), undo (M8), the store (M5), zero dependencies.

---

## 7. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Renderer produces ugly or wrong Rust for constructs we did not anticipate | High | Medium | bodies are opaque text; only structure is rendered; `cargo check` is the arbiter; golden files make drift visible |
| The statement splitter guesses wrong on exotic syntax | Medium | High | I6: lossless or refuse; `split = never`; adversarial fixture corpus grows with every bug report |
| `cargo --message-format=json` schema drift | Low | Medium | fixtures checked in; parser ignores unknown reasons; one file to fix |
| RA index cost makes `refs` feel slow | Medium | Low | ephemeral sessions, disk cache, `NFR-13`; daemon is `D-6`, one adapter swap |
| RA sees a different program than `cargo check` (cfg/features) | Medium | Medium | `--strict --semantic` shows disagreements; L1 remains the arbiter; documented (`SRD` §12.6) |
| In-place transaction restores incorrectly after a crash | Low | High | restore path is exercised by every rejection; `doctor --repair`; property test on tree round-trip |
| Scope creep: the tool becomes a build system | High | High | §6 cut list, and the SRD's non-goals are explicit |
| The 200-line ceiling collides with LSP handling | Medium | Medium | the split is already planned (`frame`/`client`/`edit`/`session`); if a file exceeds 200, stop and name the seam |

---

## 8. What "done" means for the whole plan

1. UC-1 … UC-12 pass as written, in `tests/acceptance/`.
2. `make check` is green with `el` checking `el` under `el.policy`.
3. The binary is ≤ 1.5 MB, has no dependencies, and `Cargo.lock` has one package.
4. Every module has a preamble; every public item has a contract; no file over 200 lines.
5. `docs/reports/m12-dogfood.md` records a real session where the tool built itself.
6. The final message states what changed, what proved it, what was deferred, what was overridden.
