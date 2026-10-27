# The gate — verifying a change before it is committed

> The headline promise (`P1`): a change either compiles or is not applied.
> This document says exactly how that is decided, how it fails, and what it cannot know.

---

## 1. Why a gate and not a linter

A linter runs after the fact and prints an opinion. A gate runs **inside** the mutation and
has the authority to refuse it. The difference is not strictness; it is that refusing is
cheap only if the previous state is still intact — which is why the gate is inseparable
from the transaction and from history (`HISTORY.md`).

The unit of the verdict is the **diagnostic delta**: errors this change introduced, not the
errors the crate already had. A crate with 40 pre-existing warnings must not make every
`el add fn` look like a disaster.

---

## 2. Tiers

| Tier | Name | Runs | Answers | Cost | Default |
|---|---|---|---|---|---|
| L0 | `model` | pure core, no I/O | Is the catalog coherent? Ids unique, refs resolve, body balanced, policy honoured, budget respected | ≤ 20 ms | always |
| L1 | `check` | `cargo check --all-targets --message-format=json` | Would `rustc` accept this? | 0.3–2 s warm | yes |
| L2 | `strict` | L1 + `cargo clippy -- -D warnings` + `cargo test --no-run` | Would CI accept this? | 2–10 s | no |
| L3 | `semantic` | `rust-analyzer` LSP diagnostics | Would the IDE accept this, and what does it resolve to? | 1–4 s cold, ≤ 1.5 s warm | no |

Selection: default from `el.policy` `gate = model | check | strict | semantic`; overridden by
`--fast` (L0), `--strict` (L2), `--semantic` (L3). `--strict --semantic` runs both and reports
disagreements — the honest way to find out that RA and `rustc` see different programs.

Each tier is recorded in history as its own string, so `el log` never claims more verification
than happened (`FR-24`).

---

## 3. The transaction

```text
1  lock          acquire .el/lock (O_EXCL, pid+start time); stale after 10 min if pid is dead
2  recover       if .el/inflight exists → restore head's tree first (FR-38); no other work
3  snapshot      current projection → tree object; verify every blob digest
4  parse         argv/stdin → Change                                     (pure)
5  plan          apply(Change, catalog) → Projection | Violations        (pure)
                 Violations → exit 2. Nothing has been written yet.
6  inflight      write .el/inflight {seq, from, started}
7  stage         write new tree object + journal entry with gate=pending (fsync)
8  materialize   for each path in the new tree: tmp file → fsync → rename
                 for each path dropped: unlink
9  gate          run the selected tier against the materialized tree
10a pass         head := seq; rewrite journal entry gate=ok; remove inflight; release lock; exit 0
10b reject       restore from ‹from› tree (rewrite every path, unlink the rest)
                 rewrite journal entry gate=rejected:<n>; head unchanged; remove inflight; exit 3
10c dry-run      restore from ‹from› tree; journal entry gate=preview:<n>; head unchanged
```

Properties this buys, each with a test:

| Property | Why it holds | Test |
|---|---|---|
| A rejected change leaves the tree byte-identical | restore writes from content-addressed blobs, not from a diff | `acceptance/rejected_change_restores_tree.rs` |
| A crash at any point is recoverable | step 6 precedes every write that changes the projection; `head` never advances before the gate passes | `acceptance/crash_leaves_tree_restorable.rs` |
| `--dry-run` is full fidelity | it is the same pipeline with step 10c instead of 10a; no second code path | `acceptance/dry_run_is_full_fidelity.rs` |
| Concurrent invocations cannot interleave | step 1, and restoration is idempotent | `acceptance/lock_is_exclusive.rs` |
| A model violation touches nothing | step 5 precedes step 6 | `acceptance/model_violation_writes_nothing.rs` |

**Why render in place instead of in a scratch copy.** A scratch directory changes source
paths, which invalidates every `cargo` fingerprint: the gate would cost a full rebuild on
every keystroke. In-place-plus-restore reuses the incremental cache, and the restore path is
exercised by every rejected change, so it is the *most* tested code in the tool, not the
least. The cost is a brief window where the tree is the new version; the lock and the
`inflight` marker make that window observable and recoverable, and only `el` writes there.

---

## 4. Diagnostic delta

### 4.1 Fingerprinting

A diagnostic is fingerprinted by what identifies it, not by where it happened:

```text
fingerprint = (code, innermost_el_id, normalized_message)
```

- `code` — `E0308`, `unused_variables`, `mismatched-types` (RA). Exact.
- `innermost_el_id` — from the **span map** (`FR-17`): the renderer knows which element owns
  each line of the file it just produced. A position maps to a statement id, or an item id,
  or, in an opaque module, to the file.
- `normalized_message` — the message with whitespace collapsed and absolute
  paths replaced by `<path>`, so it is stable across machines and runs.
  Type-parameter names are deliberately NOT erased: erasure is
  position-stable, and over-erasing merges distinct errors (ADR-0026).

Line numbers are deliberately **not** in the fingerprint. This is the whole point: because
`el` renders the file, it can attribute a diagnostic to an id that survives every edit above
it. A line-based delta would report a phantom "new error" whenever a statement was inserted
at the top of a body.

E0601 is never attributed: it names a crate condition ("main function not
found"), not an element, so it always falls back to (code, path,
normalized_message) (ADR-0027).

A diagnostic whose span maps to no element falls back to
`(code, path, normalized_message)` and the output says `"exact": false`.
The line is display, not identity: a structural edit above a crate-level
diagnostic moves its span line, and treating that move as a new error
refused the first `add mod` after `el new` (the scaffold's E0601,
re-pointed — ADR-0026).

### 4.2 Baseline

The baseline is the diagnostic set of `head`, stored beside its tree:

```text
.el/diags/<treeDigest>   el1 <tier> <code> <level> <el|-> <owner|-> <exact 1|0> <path> <line> <col> <message-sanitized>

One tab-separated line per diagnostic; other tiers' lines for the same tree
are preserved when one tier's set is rewritten. A file that exists records
a set, even when empty; a missing file is computed once from the previous
tree (restore, run, record, re-materialize). Corrupt lines are skipped.
```

Written after every successful gate and after `el check`. If the baseline is missing (first
run, or a tier changed), the gate runs the previous tree through the same tier once and
records it — the user never sees an inflated "41 new errors" on the first command.

### 4.3 Report

```text
el: change rejected — 1 new error (0 fixed, 4 pre-existing)
  error[E0308] src/core/plan.rs:41  el-0102  (in el-0007 core::plan::apply)  (first actionable)
    expected `u32`, found `&u32`
  error[E0308] src/core/plan.rs:42  el-0103  (in el-0007 core::plan::apply)
    expected `u32`, found `&u32`
el: no files changed; history unchanged (attempt recorded as seq 42, gate=rejected:1)
```

Three counts, always: new, fixed, pre-existing. A user who fixes ten errors and breaks one
sees both facts. Exactly one new diagnostic is marked `first_actionable`: the first one
attributed to an element the change touched, else the first exact one, else the first in the
compiler's order. The rule is deterministic, so an agent that retries the same change is sent
to the same line.

### 4.4 What counts as a rejection

| Verdict | Rule |
|---|---|
| reject | ≥ 1 new `error` |
| ok | 0 new errors, regardless of pre-existing state |
| note | new `warning`s are reported but do not block, unless `--strict` |
| reject (strict) | any new warning, because `-D warnings` is what CI will run |

`--force` and `--no-gate` are deliberately absent (`--no-gate` is
unimplemented). To land a change that does not compile, use `--fast`
(recorded as `gate=model`). It is visible in `el log`, which is the point:
an unverified change must be a *recorded* decision, not a silent one.

---

## 5. L0 — the model tier

Pure functions over the catalog. It runs first because it is free and because a model error
means the compiler's answer would be noise.

| Check | Violation code |
|---|---|
| Duplicate alias across elements | `model.duplicate_alias` |
| Unknown owner, module or element reference | `model.unknown_ref` |
| Statement moved across bodies | `model.cross_body_move` |
| Unbalanced delimiters in a body or signature | `model.unbalanced` |
| Reserved keyword or invalid identifier | `model.bad_ident` |
| File over `ceiling` after the change | `model.ceiling` |
| Function over `max_fn_lines` | `model.fn_budget` |
| Module without a preamble when `require_preamble` | `model.preamble` |
| Banned construct (`forbid = unsafe`) | `model.forbidden` |
| Dependency added without `--allow` | `model.dep_denied` |
| Removal leaving a reference | `model.dangling_ref` |

Each message names the element id, the computed size, and the smallest action that would
satisfy the rule — the ceiling violation suggests the statement ids that could be extracted
(`FR-73`), because "your file is too long" without a seam is not advice.

---

## 6. L1/L2 — the compiler tier

`cargo check --all-targets --message-format=json`, parsed by our own JSON reader
(`core/text/json.rs`), never by a crate. Only `reason == "compiler-message"` frames with
`message.level` in `{error, warning}` are used; the rest are ignored so that a cargo upgrade
that adds frames cannot break us.

`cargo` exit code 101 is expected when diagnostics exist; the exit code alone is never the
verdict — the delta is.

L2 adds `cargo clippy --all-targets -- -D warnings` and `cargo test --no-run`, whose
diagnostics join the same set. Clippy codes are prefixed (`clippy::needless_return`) so they
cannot collide with `rustc` codes.

Timeout: 120 s per invocation (env `$EL_CARGO_TIMEOUT`), after which the child is killed,
the tree restored, `error.code = "env.timeout"`, exit 4.

---

## 7. L3 — the semantic tier

`rust-analyzer` over LSP on stdio (`ADR-0011`). One session per command:

```text
spawn → initialize (rootUri, capabilities) → initialized
      → textDocument/didOpen for every projection file
      → wait for workspace loading, then textDocument/diagnostic per file
      → shutdown / exit → reap
```

- Diagnostics are pulled per file; `publishDiagnostics` push notifications are consumed as a
  fallback. Both are normalized into the same `Diagnostic` type as L1, so the delta code is
  shared and the fingerprint is identical.
- RA codes are namespaced `ra::<code>` so an L1 and L3 baseline can coexist for comparison
  under `--strict --semantic`.
- The session is bounded (`FR-56`) and its stderr is quarantined to `.el/log/ra.log` (`FR-58`).
- `refs`, `rename` and `fix` reuse the same session type with different requests. They are the
  only commands that start RA besides `check --semantic` (`FR-60`).

**Why not make L3 the default.** Two reasons, both measured: a cold session pays RA's index
cost (1–4 s on a 30-module crate) on every single `el add`, and RA is not the arbiter —
`rustc` is. L1 is the cheapest *authoritative* opinion. L3 earns its place on questions L1
cannot answer at all: references, exhaustive rename, quick fixes.

---

## 8. Latency budget and where it goes

| Step | Budget | How |
|---|---|---|
| process start + lock | ≤ 5 ms | no runtime, no dynamic init, lazily read only the catalog |
| L0 | ≤ 20 ms | pure, over ≤ 300 elements |
| render | ≤ 10 ms | string building into one buffer per file |
| write | ≤ 15 ms | only changed paths; tmp+rename |
| L1 warm | ≤ 2 s | in-place render keeps cargo's fingerprint cache valid |
| L3 warm | ≤ 1.5 s | RA cache in `target/`; session reuse within one command (refs+diagnostics) |

If L1 exceeds budget, the first thing to check is that the gate is not accidentally
changing `Cargo.toml` on every run (a manifest rewrite invalidates the whole graph).

---

## 9. Testing the gate deterministically

The gate is the part most likely to be tested by accident and never on purpose. Rules:

1. **Unit tests never invoke `cargo` or `rust-analyzer`.** `core/gate/*` takes `Vec<Diagnostic>`
   and returns a `Verdict`; the parsers take `&str` fixtures captured from real runs.
2. **Fixtures are checked in** (`tests/fixtures/cargo-json/*.jsonl`, `tests/fixtures/ra/*.lsp`)
   and are the contract with the outside tools. A cargo upgrade that breaks parsing breaks a
   test, not a user.
3. **The transaction is tested through the `Tools` port fake** — a test double that returns
   canned diagnostics, counts spawns (`FR-60`) and can be told to hang (timeout tests) or die
   (crash tests). No network, no clock, no real subprocess in the fast suite.
4. **One integration test per tier** runs the real tool on a throwaway crate in
   `std::env::temp_dir()`; these are the only tests that need `cargo` installed, and they are
   marked `#[ignore]`-free but excluded from the fast suite by `make check`'s split:
   `make test` (unit, < 10 s) and `make test-slow` (integration).

---

## 10. What the gate cannot do

| Claim it must not make | Reality |
|---|---|
| "This change is correct" | It is *compilable*. Semantics are `el test`'s job and the author's. |
| "No new errors" without qualification | True only relative to the recorded baseline and the tier that ran. Both are printed. |
| "The tree is restored" after a crash at step 8 | Restored at the next invocation, not at crash time. `el doctor` reports when it did. |
| "RA agrees with rustc" | It often does not. `--strict --semantic` shows the disagreement; nothing hides it. |
| "Your statement split was right" | The splitter is lexical. Lossless is proven; meaningful is not. A split at the wrong place still compiles or is rejected by the compiler. |
