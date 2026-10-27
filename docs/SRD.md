# SRD — `el`, a Unix-way editor for Rust programs

**Status:** draft 1, for review · **Date:** 2026-10-24 · **Owner:** prime agent
**Governing standard:** `AGENTS.md` (§2 test-first, §3 small functions, §4 200-line ceiling, §5 literate code)
**Companion documents:** `COMMANDS.md` · `GATE.md` · `HISTORY.md` · `ARCHITECTURE.md` · `PLAN.md` · `docs/adr/`

---

## 0. How to read this document

| Section | Answers |
|---|---|
| §1–§3 | Why the tool exists, what it refuses to do, what its words mean |
| §4–§6 | Who uses it, what they are trying to do, the one model underneath |
| §7–§11 | What it must do, precisely, with the test that proves it |
| §12–§16 | Non-functional budgets, formats, honesty limits, open decisions, traceability, deferrals |

Every requirement has an id (`FR-n`, `NFR-n`) and a **verified by** column naming the
test that will be seen red before it is seen green (§2.1 of `AGENTS.md`).
A requirement without a test id is not a requirement; it is a wish.

---

## 1. Problem

An agent or a user who wants a small Rust program today must:

1. remember `cargo new`, the manifest grammar, module paths, `mod` declarations and visibility;
2. open an editor, write text, save, switch to a terminal, `cargo check`, read errors,
   switch back, fix, repeat;
3. guess whether a change is safe *before* making it;
4. use `git` (or nothing) to get the change back;
5. accept that every helper tool they install pulls 80 crates and a 9 MB binary.

Steps 1, 2 and 4 are mechanical. Step 3 is impossible with a text editor: a text editor
cannot tell you that renaming a function breaks three call sites until after you saved.
Step 5 is a choice the ecosystem made and this tool rejects.

**The gap:** there is no tool whose unit of work is a *Rust element* rather than a *file*,
that can predict the compiler's verdict before committing the change, and that costs the
user nothing to install.

---

## 2. Purpose and promise

> `el` lets a person or a program build a complete Rust program by naming **elements**
> — modules, functions, structs, traits, tests — instead of editing files, and refuses
> to commit a change that would not compile.

Three promises, in order of importance:

| # | Promise | Consequence if broken |
|---|---|---|
| P1 | **Every mutation is verified before it is committed.** A change either compiles or is not applied. | The tool is worse than an editor. Fatal. |
| P2 | **Every mutation is reversible, exactly.** One word undoes any change, byte for byte. | Users stop trusting the tool. Fatal. |
| P3 | **The binary depends on nothing.** Zero crates, one file, `std` only. | The tool is not installable in the environments it targets. Fatal. |

Anything that threatens P1–P3 is cut, not negotiated. See `PLAN.md` §6 for the
cut list.

---

## 3. Scope, non-goals, glossary

### 3.1 In scope

- Create, inspect, edit, move and delete Rust **elements** via one command — at every level:
  modules, items, **and the statements inside a function body**.
- Render a complete, compilable Cargo crate from a catalog of elements.
- Predict and report the compiler's verdict on a change before it is committed.
- **Use the user's `rust-analyzer` out of the box** — zero configuration, auto-detected — for
  semantic find-references, exhaustive rename, hover/documentation, and quick fixes, over
  LSP on stdio. It is a *system tool we drive*, never a crate we link.
- Keep exact, content-addressed history; undo, redo, diff, revert.
- Read Rust documentation from the internet, cached, without a network stack in the binary.
- Serve both humans (tables, colour when a TTY) and programs (`--json`, exit codes, no prompts).

### 3.2 Out of scope (v1) — each with where it would live instead

| Not doing | Why | Where it lives |
|---|---|---|
| Parsing existing Rust source | A real Rust parser is either a 100k-line dependency (`syn`) or a project of its own. **`rust-analyzer` already has one**; we ask it. | `el import --adopt` stores a file as an **opaque module**; a symbol-tree import via LSP `documentSymbol` is deferred (ADR-0009, ADR-0014). |
| Our own semantic analysis, resolver or LSP server | Same reason, one level up. `ra_ap_*` is 40 crates. | We are an LSP **client** over stdio. `rust-analyzer` is a system tool, like `cargo` and `curl`. ADR-0011. |
| A TUI or editor | One binary, composable, scriptable. Editors already exist. | `el` outputs text; pipe it. |
| Formatting as a service | `rustfmt` is a system tool and is not always the right one. | `el` renders canonically; `cargo fmt` on managed modules is detected as whitespace drift and re-rendered (ADR-0009). |
| Dependency resolution | `cargo` is the resolver. | `el` writes the manifest; `cargo` does the rest. |
| Language server, watch mode, plugins | Not needed to satisfy P1–P3. | — |
| Branches / merge in history | Undo/redo plus revert covers the intent; merges are a second problem. | `HISTORY.md` §7 (deferred, kept open). |
| Windows-first behaviour | Design is `std`-only and portable; testing targets Linux first. | `NFR-2`. |

### 3.3 Glossary

| Term | Meaning in this document |
|---|---|
| **Element** | A named, addressable piece of a Rust crate: `mod`, `fn`, `struct`, `enum`, `trait`, `impl`, `const`, `static`, `type`, `use`, `test`, **and `stmt`** (a statement inside a function or impl body). Has a permanent id. |
| **Statement (stmt)** | One addressable line-or-block inside a body: `let x = f();`, an `if`/`match`/`for` block, a call. Splitting is lexical and conservative; see §8.10. |
| **Body mode** | `split` (body is a list of statement elements) or `opaque` (body is one blob). Set per element; downgrade is always available, upgrade requires a clean split. |
| **Catalog** | The ordered set of elements plus modules, deps and policy. The source of truth. Plain data. |
| **Projection** | The files on disk (`src/**`, `Cargo.toml`) rendered from the catalog. Derived, disposable, owned by `el`. |
| **Gate** | The verification step that runs between "change computed" and "change committed". Tiered L0/L1/L2/L3. |
| **Diagnostic delta** | New compiler errors *introduced by this change*, computed against the pre-change baseline. Not the absolute error count. |
| **Snapshot / tree** | A content-addressed record of the full projection at one history entry. |
| **Transaction** | One mutating command: snapshot → render → gate → commit or restore. |
| **Opaque module** | A projection file the tool stores but does not model at element level. |
| **Policy** | A repository file (`el.policy`) of enforced rules: line ceiling, banned constructs, dependency allowlist. |
| **RA** | `rust-analyzer`, the user's installed binary, driven as an LSP server over stdio. Optional, auto-detected, never required. |
| **LSP session** | One short-lived `rust-analyzer` process: spawn → `initialize` → 1..n requests → `shutdown` → reap. Per command, not per keystroke. |
| **Span map** | The per-projection `line → element id` table emitted by the renderer (FR-17). It is how compiler and LSP positions become element identities. |
| **Semantic tier (L3)** | The gate tier that asks RA instead of `rustc`: faster on a warm index, and able to answer reference and resolution questions `cargo check` cannot. |

---

## 4. Actors

| Actor | Intent | Environment | What it needs from `el` |
|---|---|---|---|
| **Author** (human, interactive) | Get a small program working, without remembering Rust's file grammar | Terminal, has `cargo` | Short verbs, readable errors, undo, no ceremony |
| **Agent** (LLM or script) | Make a specific change, learn whether it worked, recover from failure | No TTY, pipes, retries | Stable ids, `--json`, exit codes, idempotence, dry-run, no prompts |
| **Reviewer** | Understand what changed and why, revert if wrong | Terminal or CI | `el log`, `el diff`, `el explain`, gate verdict per entry |
| **Maintainer** (of `el` itself) | Keep the tool small and honest | CI | Zero-dependency gate, line ceiling, port/adapter boundaries |

Two actors, one surface. Every capability is reachable by both; nothing is TUI-only,
nothing is JSON-only, and no command ever blocks on input.

---

## 5. The one model

```
  intent (argv, stdin)          pure, ~1 ms
        │
        ▼
    ┌────────┐   Change    ┌──────┐   Projection   ┌────────────┐
    │  parse │────────────▶│ plan │───────────────▶│  materialize│
    └────────┘             └──────┘                └──────┬─────┘
                                │                         │
                          Violations                 ┌────▼────┐
                                │                    │  gate   │  cargo check
                                ▼                    └────┬────┘
                          exit 2 (no writes)               │
                                                    new diagnostics?
                                                     ┌────┴────┐
                                                    yes        no
                                                     │          │
                                                 restore     commit
                                                 exit 3      exit 0
```

The catalog is a **tree of three levels**, and every node in it is an element with an id:

```
crate ── module (src/core/plan.rs)
          ├── item  (fn apply, struct Plan, use …)
          ├── item  (fn run)
          └── stmt  (body of apply)
                ├── stmt  let change = self.clamp();
                ├── stmt  if change.is_empty() { … }
                └── stmt  Ok(outcome)
```

Addressability goes all the way down: an agent can replace one statement without
regenerating a function, and a diagnostic can be attributed to that statement by id
rather than to a line number that the next edit invalidates. Statements exist because
**the unit of a change should be the unit of a mistake** — and a mistake is usually one
line.

Invariants hold at every moment:

- **I1** The catalog is the only source of truth. The projection can always be rebuilt from it.
- **I2** The projection is byte-identical to what the catalog renders. Drift is detected, never silently absorbed.
- **I3** `head` always names a tree that is materialized on disk, or is restorable from the store.
- **I4** Every state the tool has ever been in is reachable by `el log` and restorable by `el revert`.
- **I5** Everything outside the projection — `cargo`, `rust-analyzer`, `curl` — is a **tool we
  invoke and parse**, never a library we link. If a tool is absent, the capability degrades
  by name and exit code, and the rest of the pipeline still works.
- **I6** Splitting a body into statements is **lossless or refused**: either the rendered
  statements concatenate to the original body byte for byte, or the body is stored
  `opaque`. There is no third outcome, and no heuristic is allowed to guess.

Invariant I5 is what makes the "small, dependency-free" promise compatible with "support
`rust-analyzer` out of the box": RA is a second opinion, not a load-bearing wall. Every
command that uses it states, in one line, what it did instead when RA was unavailable.

This is why the tool is small: it is one pipeline with four verbs of work
(parse, plan, render, verify) and a store, not a framework.

---

## 6. Use cases

Each use case names an **actor achieving an intent** (§2.3 of `AGENTS.md`), not a function.

### UC-1 Author creates a program that prints a word, from nothing, in five commands

```console
$ el new hello && cd hello
$ el add mod src/greet.rs
$ el add fn greet --in src/greet.rs --vis pub --sig 'fn greet(name: &str) -> String' --body -
pub fn greet(name: &str) -> String { format!("hello, {name}") }
^D
$ el add fn main --sig 'fn main()' --body -
fn main() { println!("{}", greet::greet("world")); }
^D
$ el run
hello, world
```

**Acceptance:** `tests/acceptance/author_builds_program_from_nothing.rs` — exits 0, stdout
contains `hello, world`. Gate ran on every mutation; `el log` shows five entries (the `init` and root-module entries from `new`, then three adds), all `gate=model` until M9 adds L1.

### UC-2 Agent adds a function and is told, before the commit, that it would not compile

```console
$ el add fn total --sig 'fn total(xs: &[u32]) -> u32' --body - <<<'xs.iter().sum()'
el: change rejected — 1 new error (0 fixed, 4 pre-existing)
  error[E0308]: mismatched types
    src/core/total.rs::total  (line 12)
    expected `u32`, found `&u32`
el: no files changed; history unchanged (attempt recorded as seq 42, gate=rejected)
$ echo $?
3
```

**Acceptance:** `tests/acceptance/agent_is_told_before_commit.rs` — projected files unchanged
byte for byte, exit 3, `--json` envelope carries `error.code = "gate.rejected"` and the
diagnostic's element id.

### UC-3 Reviewer finds when a rule was broken and returns to before it

```console
$ el log --json | jq '.[] | select(.gate != "ok")'
$ el revert 41 && el test
```

**Acceptance:** `tests/acceptance/reviewer_restores_a_known_good_tree.rs` — the tree digest
after revert equals the recorded digest of seq 41; `el test` green.

### UC-4 Author splits a module that reached the ceiling

```console
$ el check
error: src/core/plan.rs is 203 lines, ceiling is 200 (policy: ceiling=200)
  hint: `el mv el-0012 src/core/apply.rs` moves an element into a new module
$ el mv el-0012 src/core/apply.rs
moved el-0012 apply → src/core/apply.rs; 3 references rewritten
plan.rs 118 · apply.rs 96 · gate ok
```

**Acceptance:** `tests/acceptance/author_splits_module_at_ceiling.rs`.

### UC-5 Person looks up std without leaving the terminal

```console
$ el search BTreeMap
std::collections::BTreeMap   ordered map based on a B-tree
  https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
$ el doc std::collections::BTreeMap | head -20
```

**Acceptance:** `tests/integration/search_uses_cache_without_network.rs` — second call performs
zero subprocesses; `--offline` on a cold cache exits 4 with an actionable message.

### UC-6 Agent composes five edits into one atomic change

```console
$ el apply - --dry-run <<'EOF'
add mod src/core/gate.rs
add fn run --in src/core/gate.rs --sig 'fn run(p: &Plan) -> Verdict'
mv el-0004 src/core/gate.rs
edit el-0007 --rename apply_change
rm el-0011
EOF
el: 5 changes, 1 transaction, gate would pass (0 new diagnostics)
```

**Acceptance:** `tests/acceptance/agent_composes_edits_atomically.rs` — either all five land as
one journal entry, or none do.

### UC-7 Maintainer proves the tool still has no dependencies

```console
$ make check
... 47 tests passed
zero_dependencies ... ok
core_is_pure ... ok
line budget ... 31 files, max 178, all within 200
```

**Acceptance:** `tests/meta/zero_dependencies.rs`, `tests/meta/core_is_pure.rs`,
`tools/check_line_budget.py`.

### UC-8 Author recovers from a killed process

```console
$ el add fn x --body - &   # SIGKILL mid-transaction
$ el ls
el: recovered from interrupted change (seq 44); projection restored to seq 43
...
```

**Acceptance:** `tests/integration/crash_leaves_tree_restorable.rs`.

### UC-9 Author renames a function and every call site follows, semantically

```console
$ el refs el-0007
el-0007  fn  core::plan::apply
  5 references
    src/core/plan.rs:41        el-0021  fn  core::plan::run        (call)
    src/core/validate.rs:88    el-0033  fn  core::validate::check  (call)
    src/core/render/mod.rs:12  el-0040  use core::plan::apply
    ...
$ el edit el-0007 --rename apply_change
el: rust-analyzer 2026-10-13 · 5 references · 6 edits across 3 files
edit el-0007 apply → apply_change; 5 references rewritten
gate ok (L3 semantic, 0 new diagnostics, 812 ms)
```

**Acceptance:** `tests/acceptance/rename_uses_rust_analyzer.rs` — the renamed tree compiles with
`cargo check`, the catalog round-trips, and with `rust-analyzer` missing, `el refs` falls back to
catalog-level references and says so on stderr (exit 0, `"source":"catalog"` in `--json`).

### UC-10 Agent asks rust-analyzer to fix what it broke

```console
$ el check --json | jq -r '.gate.diagnostics[] | "\(.el) \(.code)"'
el-0051 E0308
el-0051 E0425
$ el fix el-0051 --list
E0425  cannot find value `limt` in this scope       quickfixes: 2
E0308  mismatched types                            quickfixes: 1
$ el fix el-0051 --apply E0425:0
applied 1 edit via rust-analyzer; gate ok
```

**Acceptance:** `tests/acceptance/fix_applies_quickfix_and_gates.rs`.

### UC-11 Author replaces one statement and nothing else moves

```console
$ el show el-0007 --statements
el-0007  fn  core::plan::apply   body: split (9 statements)
  el-0101   let change = self.clamp();
  el-0102   if change.is_empty() { return Ok(Outcome::Noop); }
  el-0103   let mut out = Outcome::new();
  ...

$ el edit el-0102 --body - <<'EOF'
if change.is_empty() { return Err(Error::Empty); }
EOF
el-0102  rewritten; 1 statement changed; gate ok

$ el diff
el-0102  (in el-0007 core::plan::apply)
-  if change.is_empty() { return Ok(Outcome::Noop); }
+  if change.is_empty() { return Err(Error::Empty); }
1 statement changed; 8 untouched (el-0101, el-0103 … byte-identical)
```

**Acceptance:** `tests/acceptance/edit_replaces_one_statement_only.rs` — every other statement's
bytes and id are unchanged; `el diff` names the statement id, not a line range; the gate
attributes any error to the statement id.

### UC-12 Agent inserts a statement into a body it has never read in full

```console
$ el add stmt el-0007 --after el-0105 --body - <<<'debug_assert!(!out.plan.is_empty());'
el-0117  added to el-0007 after el-0105; gate ok (L1, 0 new diagnostics)
```

**Acceptance:** `tests/acceptance/add_statement_by_anchor.rs`.

---

## 7. Command surface (summary)

Full grammar, flags, JSON shapes and exit codes in `COMMANDS.md`. The rule behind the
surface: **verbs, one level deep, reads and writes separated, no prompts.**

| Group | Verbs |
|---|---|
| Create | `new`, `init` |
| Mutate | `add`, `edit`, `rm`, `mv`, `apply` (batch) |
| Inspect | `ls`, `show`, `tree`, `explain`, `refs`, `catalog` (machine view of everything) |
| Verify | `check`, `test`, `build`, `run`, `fix` |
| History | `log`, `diff`, `undo`, `redo`, `revert` |
| Outside world | `search`, `doc` (network, via system `curl`) |
| Self | `doctor`, `skill`, `version` |

Universal flags on every command: `--json`, `--quiet`, `--dry-run` (writes only),
`--yes`, `--no-color`, `--root <path>`, `--fast` / `--semantic` (where a semantic
alternative exists).

---

## 8. Functional requirements

**Verified by** names the test file. All tests are written red first (§2.2).

### 8.1 Model and catalog

| Id | Requirement | Verified by |
|---|---|---|
| FR-1 | Every element has a permanent id (`el-NNNN`), assigned at creation, never reused, never changed by rename or move. | `core/model/id.rs` unit tests; `acceptance/id_survives_rename_and_move.rs` |
| FR-2 | Every element has a unique human alias `<module>::<kind>/<name>`; collisions are rejected at L0 with the colliding id named. | `core/plan/validate.rs`; `acceptance/duplicate_alias_is_rejected.rs` |
| FR-3 | The catalog encodes to a deterministic, line-oriented text form; encode(decode(x)) == x, and two catalogs with equal content encode to equal bytes. | `history` property test: `tests/properties/catalog_round_trips.rs` |
| FR-4 | `el catalog --json` emits every element, module, dep and policy entry, sorted by id, with no loss. | `acceptance/catalog_is_complete.rs` |
| FR-5 | Unknown kinds, malformed signatures, unbalanced bodies and invalid identifiers are rejected before any write. | `core/plan/validate.rs`; `acceptance/malformed_change_touches_nothing.rs` |

### 8.2 Mutation

| Id | Requirement | Verified by |
|---|---|---|
| FR-6 | `add` creates an element at a well-defined position (end of module, or `--after <id>`) and returns its new id on stdout. | `acceptance/add_returns_stable_id.rs` |
| FR-7 | `edit` changes only the named fields; omitted fields are preserved byte for byte. | `acceptance/edit_preserves_untouched_fields.rs` |
| FR-8 | `mv` relocates an element, creating its module if absent, and rewrites every `use`/path reference the catalog knows about. | `acceptance/move_rewrites_references.rs` |
| FR-9 | `rm` removes an element and reports every remaining reference to it; dangling references are an L0 violation unless `--cascade`. | `acceptance/remove_reports_dangling_references.rs` |
| FR-10 | `apply` executes a batch of changes as **one** transaction and one journal entry, all-or-nothing. | `acceptance/apply_is_atomic.rs` |
| FR-11 | Every mutation is idempotent where the intent is idempotent: re-adding an identical element is a no-op with exit 0 and `changed: []`. | `acceptance/repeated_add_is_noop.rs` |
| FR-12 | No command ever reads from stdin unless it was given `-` or the stdin is a pipe *and* the command documents it. No command prompts. | `acceptance/never_prompts.rs` (runs with stdin closed) |

### 8.3 Rendering

| Id | Requirement | Verified by |
|---|---|---|
| FR-13 | `render(catalog)` is pure and deterministic: same catalog → same bytes, on any machine, in any order of insertion. | `tests/properties/render_is_deterministic.rs` |
| FR-14 | Rendered files are valid Rust that `cargo check` accepts when the catalog is valid. | `integration/rendered_crates_compile.rs` (five catalogs, real `cargo check`, not `#[ignore]`d) |
| FR-15 | `mod` declarations are generated from the module tree; users never write them. A generated `mod` line is as visible as the widest item beneath it, so it never hides what the catalog exposed (ADR-0016). | `acceptance/mod_tree_is_generated.rs` |
| FR-16 | `Cargo.toml` is generated and contains no dependency table unless the catalog declares one. | `acceptance/manifest_has_no_deps_by_default.rs` |
| FR-17 | The tool emits a span map (path → element id per line range) with each projection, used by the diagnostic delta. | `core/render/spans.rs`; `acceptance/diagnostics_map_to_elements.rs` |

### 8.4 Verification (the gate)

| Id | Requirement | Verified by |
|---|---|---|
| FR-18 | L0 (model) validation runs on every mutation, costs ≤ 20 ms, and writes nothing on failure. | `acceptance/model_violation_writes_nothing.rs` |
| FR-19 | L1 (compiler) runs by default on every mutation: `cargo check --all-targets --message-format=json`, parsed, diffed against the pre-change baseline. | `acceptance/gate_runs_cargo_check.rs` |
| FR-20 | The gate reports **new** diagnostics, not total; fixed and pre-existing counts are stated separately. | `core/gate/delta.rs`; `acceptance/gate_reports_only_new_errors.rs` |
| FR-21 | A change introducing a new error is **not committed**: projection restored, head unchanged, attempt recorded with `gate=rejected`, exit 3. | `acceptance/rejected_change_restores_tree.rs` |
| FR-22 | Diagnostics are attributed to element ids where the element is modelled, and to file+line inside opaque modules. | `acceptance/diagnostic_has_element_id.rs` |
| FR-23 | `--dry-run` performs the full transaction (render + gate) and always restores; exit reflects the gate verdict. | `acceptance/dry_run_is_full_fidelity.rs` |
| FR-24 | `--fast` skips L1 and says so; `--no-gate` records `gate=skip` in history. Neither is ever silent. | `acceptance/skipped_gate_is_recorded.rs` |
| FR-25 | `--strict` adds clippy `-D warnings` and `cargo test --no-run` to the gate. | `acceptance/strict_gate_catches_warnings.rs` |
| FR-26 | Policy rules (line ceiling, banned constructs, dependency allowlist, required preamble) are enforced at L0 with the offending element and its size named. | `acceptance/policy_ceiling_blocks_growth.rs` |
| FR-27 | The gate's latency on a warm 30-module crate is ≤ 2 s at p95 (`NFR-4`); `el check --fast` ≤ 50 ms. | `tests/integration/gate_latency.rs` (advisory, not a failure gate) |

### 8.5 History

| Id | Requirement | Verified by |
|---|---|---|
| FR-28 | Every committed transaction writes one journal entry containing: seq, timestamp, actor, op, target, from-tree, to-tree, gate verdict, message. | `acceptance/journal_entry_is_complete.rs` |
| FR-29 | `undo [n]` moves head back n entries and materializes that tree exactly (byte-for-byte, including deletions). | `acceptance/undo_restores_bytes.rs` |
| FR-30 | `redo` is available until a new mutation occurs; after that the future is moved to `.el/attic/` and is readable by `el log --attic`. | `acceptance/new_change_truncates_future.rs` |
| FR-31 | `revert <seq>` applies a *new* entry whose tree equals seq's tree; history is never rewritten. | `acceptance/revert_appends.rs` |
| FR-32 | `diff <a> [b]` prints a line diff of the projections; `--json` emits per-element changes. | `acceptance/diff_shows_element_changes.rs` |
| FR-33 | The store verifies content digests on read; a corrupted object is reported, never silently used. | `acceptance/corrupt_object_is_detected.rs` |
| FR-34 | `doctor --rebuild` regenerates the catalog from the journal and must reproduce it byte for byte. | `acceptance/rebuild_from_journal.rs` |

### 8.6 Drift, adoption, recovery

| Id | Requirement | Verified by |
|---|---|---|
| FR-35 | A projection file whose content differs from its tree entry in anything but whitespace is a hard error (exit 4) naming the file and offering `--adopt`. | `acceptance/hand_edit_is_detected.rs` |
| FR-36 | Whitespace-only drift (e.g. `cargo fmt`) is re-rendered silently, with a one-line note on stderr. | `acceptance/rustfmt_drift_is_rerendered.rs` |
| FR-37 | `doctor --adopt <path>` records the file as an opaque module; `ls` marks it `opaque`, and element-level edits to it are refused with an explanation. | `acceptance/adopted_module_is_opaque.rs` |
| FR-38 | An interrupted transaction (present `.el/inflight`) is detected at the next invocation and rolled back before any other work. | `acceptance/crash_leaves_tree_restorable.rs` |
| FR-39 | Concurrent invocations are serialized by `.el/lock`; the second exits 5 with the holder's pid and start time. A stale lock (dead pid, > 10 min) is reclaimed with a note. | `acceptance/lock_is_exclusive.rs` |

### 8.7 Outside world

| Id | Requirement | Verified by |
|---|---|---|
| FR-40 | `search` and `doc` fetch over `https://` by invoking a system fetcher (`$EL_FETCH`, default `curl -fsSL`); the binary links no TLS and no HTTP client. | `tests/meta/zero_dependencies.rs`; `acceptance/search_shells_out.rs` |
| FR-41 | Responses are cached by URL digest under `~/.cache/el/` with a 7-day TTL; `--offline` never touches the network and exits 4 on a miss with the exact command that would warm it. | `acceptance/offline_never_fetches.rs` |
| FR-42 | A missing fetcher is exit 4 with one actionable line; it never degrades into a stack trace or a silent empty result. | `acceptance/missing_curl_is_actionable.rs` |
| FR-43 | Search results are ordered, deduplicated, and limited (`--limit`, default 10); every result carries a canonical URL. | `acceptance/search_results_are_stable.rs` |

### 8.8 Agent surface

| Id | Requirement | Verified by |
|---|---|---|
| FR-44 | `--json` is available on every command and emits one object on stdout; all human text goes to stderr. | `acceptance/json_on_stdout_only.rs` |
| FR-45 | The JSON envelope is versioned (`"el":"1"`) and its documented fields are a compatibility contract; tests pin them. | `tests/acceptance/json_schema_is_pinned.rs` |
| FR-46 | Exit codes are stable and documented: 0 ok, 1 usage, 2 model violation, 3 gate rejected, 4 environment, 5 conflict, 6 not found, 7 internal. | `acceptance/exit_codes_are_stable.rs` (one case per code) |
| FR-47 | `el skill` prints a self-contained usage guide (≤ 150 lines) suitable for pasting into an agent's context. | `acceptance/skill_fits_a_context.rs` |
| FR-48 | Every command is safe to retry: a repeated successful mutation returns exit 0 and `changed: []` (see FR-11); a repeated failed mutation returns the same code. | `acceptance/commands_are_retryable.rs` |

### 8.9 Semantic layer — `rust-analyzer`, out of the box

> **Not built yet.** This whole section is M10 (`UC-9`, `UC-10`). The binary has no LSP
> adapter, so `--rename` today is the catalog's own token rewrite, `refs`/`explain`/`fix` do
> not exist, and `--semantic` is refused as unsupported. The requirements below are the
> contract those milestones will be held to; they are not a description of current
> behaviour.

The user installs `rust-analyzer` once (`rustup component add rust-analyzer`, or any
distribution package) and `el` uses it with **no configuration file, no flag, no daemon**.
It is driven as an LSP server over stdio, one short-lived session per command that needs it.

| Id | Requirement | Verified by |
|---|---|---|
| FR-49 | RA is located, in order: `$EL_RA`, `rust-analyzer` on `$PATH`, `rustup which rust-analyzer`. Absence is a normal state, not an error. `el doctor` reports path, version, and a one-line verdict. | `acceptance/ra_is_autodetected.rs` |
| FR-50 | `el refs <id>` answers with `textDocument/references`, grouped by file, each reference attributed to an element id via the span map, sorted by (path, line, id). With RA absent it falls back to catalog-level references and marks `"source":"catalog"` in `--json` plus one stderr line. | `acceptance/refs_uses_rust_analyzer.rs`, `acceptance/refs_degrades_without_ra.rs` |
| FR-51 | `el edit <id> --rename <name>` uses `textDocument/rename` → `WorkspaceEdit`, applies the edits to the projection, re-imports the result through the span map, and then runs the normal gate as one transaction. `--fast` uses the catalog-only rename instead. | `acceptance/rename_uses_rust_analyzer.rs` |
| FR-52 | A `WorkspaceEdit` is applied all-or-nothing. If any edit lands outside a modelled element span and outside an opaque file, the rename is refused before writing, naming `file:line`. There is no partial application, ever. | `acceptance/workspace_edit_is_atomic.rs` |
| FR-53 | `el check --semantic` (tier **L3**) collects RA diagnostics (`textDocument/diagnostic` pull model, falling back to `publishDiagnostics` push) and diffs them against the baseline exactly as L1 does: new errors block the commit and restore the tree. | `acceptance/semantic_gate_blocks_commit.rs` |
| FR-54 | `el fix <id> --list` lists RA code actions for diagnostics in that element; `el fix <id> --apply <code>:<n>` applies one as a normal transaction (one journal entry, gate re-runs). | `acceptance/fix_applies_quickfix_and_gates.rs` |
| FR-55 | `el explain <id>` includes RA hover content when available; without RA it renders the catalog's own docs and says which source it used. | `acceptance/explain_uses_hover_when_present.rs` |
| FR-56 | Every RA session is bounded: per-request timeout (default 10 s, `$EL_RA_TIMEOUT`), session budget 30 s. On timeout the child is killed, `error.code = "ra.timeout"`, exit 4. | `acceptance/ra_timeout_is_bounded.rs` |
| FR-57 | RA crash, timeout or malformed LSP frame leaves the tree byte-identical to `head` and records the attempt as `gate=rejected:ra`. Corruption is impossible by construction, and the test proves it by killing RA mid-session. | `integration/ra_crash_leaves_state_clean.rs` |
| FR-58 | RA's stderr is captured to `.el/log/ra.log` (capped at 256 KB, rotated once) and never mixed into stdout. `--verbose` streams it. | `acceptance/ra_stderr_is_quarantined.rs` |
| FR-59 | RA-derived output is sorted and deduplicated, so two identical runs produce identical bytes (`NFR-11`). | `acceptance/ra_output_is_deterministic.rs` |
| FR-60 | RA is never started by a command that does not need it: `add`, `rm`, `mv`, `log`, `undo`, `redo`, `diff`, `revert`, `search` spawn it zero times. | `acceptance/ra_spawns_only_when_needed.rs` (counts child processes through the `Tools` fake) |
| FR-61 | `el import <path> --from-symbols` (deferred, ADR-0014) builds a module's element list from `textDocument/documentSymbol`, with bodies kept as opaque text; it is the supported route to model an existing file without writing a parser. | `acceptance/import_from_symbol_tree.rs` |

### 8.10 Statement-level elements

A function body is a list of statements, each with its own id, unless the splitter cannot
prove the split is lossless — in which case the body is one opaque element (I6).

| Id | Requirement | Verified by |
|---|---|---|
| FR-62 | `stmt` is an element kind. Statements nest: a statement containing a block owns child statements, to any depth, and every one has an id. | `core/model/stmt.rs`; `acceptance/statement_tree_nests.rs` |
| FR-63 | A new body is stored `split` when the split is clean and `opaque` otherwise; the mode is shown by `el show` and stored in the catalog. | `acceptance/body_mode_is_reported.rs` |
| FR-64 | The splitter is **lexical only** — delimiter depth plus a string/comment/char-literal scanner. It never infers Rust grammar. On anything it cannot classify it returns "cannot split". | `core/text/split.rs`; `acceptance/splitter_refuses_ambiguity.rs` (raw strings, nested macros, labelled loops, lifetimes vs char literals) |
| FR-65 | Splitting is lossless and idempotent: `join(split(b)) == b` and `split(join(split(b))) == split(b)`, for every body in the corpus and for every body `el` has ever written. | `tests/properties/split_round_trips.rs` |
| FR-66 | `el add stmt <owner> (--after <id>|--before <id>|--at <n>) --body -`, `el edit <stmt> --body -`, `el rm <stmt>`, `el mv <stmt> --after <stmt>` operate on one statement; cross-body moves are an L0 violation. | `acceptance/statement_verbs_are_scoped.rs` |
| FR-67 | The tool never reformats the interior of a statement it was not asked to change; only structural indentation of a moved statement is normalized. Diff noise is a defect. | `acceptance/untouched_statements_are_byte_identical.rs` |
| FR-68 | Compiler diagnostics and RA references attribute to the **innermost** statement containing the position; `--json` reports both `el` (statement) and `owner` (function). | `acceptance/diagnostic_maps_to_statement.rs` |
| FR-69 | A `let` statement records its binding name and optional annotated type, so fallback `el refs` can name it and `el explain` can list its uses. | `acceptance/let_bindings_are_indexed.rs` |
| FR-70 | Preceding comments and attributes belong to the statement that follows them and move with it; a statement is never orphaned from its comment. | `acceptance/comments_move_with_statements.rs` |
| FR-71 | `el tree --statements` prints the three-level tree; `el ls --kind stmt` lists statements; `el catalog --json` includes them with owners and order. | `acceptance/tree_shows_statements.rs` |
| FR-72 | Granularity is policy-controlled (`split = auto | always | never`); `never` makes every body opaque and is the supported escape hatch for a body the splitter keeps refusing. | `acceptance/split_policy_is_honoured.rs` |
| FR-73 | The line budget (`FR-26`) is also computed per statement, so a change that would push a body past `max_fn_lines` is rejected with the statement ids that could be extracted. | `acceptance/statement_budget_rejects_growth.rs` |


---

## 9. Non-functional requirements

| Id | Requirement | Budget | Verified by |
|---|---|---|---|
| NFR-1 | No third-party crates in the `el` binary's dependency graph, direct or transitive — including `ra_ap_*`, `lsp-server`, `lsp-types`, `syn`, `serde`. | 0 | `tests/meta/zero_dependencies.rs` |
| NFR-2 | Only `std`; `libc` and platform crates must not appear. Portable source, Linux-first testing. | 0 non-std crates | same |
| NFR-3 | Release binary size, stripped, `x86_64`/`aarch64` Linux. | ≤ 1.5 MB | `tools/footprint.sh` (CI, advisory) |
| NFR-4 | `el ls` on a 300-element crate; warm `el check --fast`. | ≤ 15 ms / ≤ 50 ms | `tests/integration/latency.rs` |
| NFR-5 | Warm `el add fn` on a 30-module crate including L1 gate. | ≤ 2 s p95 | same, advisory |
| NFR-6 | Any source file in this repo, tests included. | ≤ 200 physical lines | `tools/check_line_budget.py` |
| NFR-7 | Every module has a 3–8 line preamble stating problem, shape, deliberate non-scope. | present | `tests/meta/preambles.rs` |
| NFR-8 | `core/` never performs I/O: no `std::fs`, `std::process`, `std::net`, `std::env::current_dir`. | 0 occurrences | `tests/meta/core_is_pure.rs` |
| NFR-9 | Generates no crate that depends on a crate unless the user explicitly asked (`el add dep --allow`). | 0 default deps | `acceptance/manifest_has_no_deps_by_default.rs` |
| NFR-10 | Whole-repo gate. | `make check` ≤ 3 min | `PLAN.md` M0 |
| NFR-11 | Byte-identical output across machines for the same catalog and the same gate inputs. | 100% | `tests/properties/render_is_deterministic.rs` |
| NFR-12 | No secrets, no telemetry, no network except an explicit `search`/`doc`. | 0 | `tests/meta/no_telemetry.rs` (grep for `net`/`Command` outside allowed adapters) |
| NFR-13 | `el refs` on a 30-module crate, RA warm. | ≤ 4 s p95 | `tests/integration/ra_latency.rs` |
| NFR-14 | `el check --semantic` on the same crate, RA warm. | ≤ 1.5 s p95 | same |
| NFR-15 | RA is optional: its absence changes no exit code except for `fix --apply` (documented RA-required), and never produces a silent fallback — the source is named in output and in `--json`. | 0 silent fallbacks | `acceptance/ra_degrades_explicitly.rs` |

---

## 10. Data formats (normative summary)

Full grammar in `HISTORY.md` §4. All formats are **line-oriented TSV** with tabs as
separators and no embedded newlines, so `grep`, `cut`, `awk` and `sort` work on them
without a parser. This is a deliberate Unix choice; the cost is that free text is
sanitized on write.

```text
# .el/journal/000042                    one file per entry
el1  42  1761266000  agent  edit  el-0007  <fromTree>  <toTree>  ok:0/1/5  "rename apply to apply_change"

# .el/trees/ab/cdef...                  one file per snapshot (the whole projection)
src/core/plan.rs   <digest>
Cargo.toml         <digest>

# .el/objects/ab/cdef...                one file per blob, digest-addressed, verified on read
<exact bytes of one projection file>

# .el/catalog                           derived cache, rebuildable from the journal
```

Projection files carry an ownership marker on line 1; the marker is the contract that
`el` owns the file:

```rust
// @el-projection — generated from the catalog; change it with `el edit`, not here.
```

---

## 11. Policy

`el.policy` at the crate root makes the repository's rules executable. Defaults match
`AGENTS.md`:

| Key | Default | Effect |
|---|---|---|
| `ceiling` | `200` | A change that pushes any generated file above this is rejected at L0, with the seam suggestion |
| `require_preamble` | `true` | A module without a 3–8 line preamble is a violation |
| `forbid` | `unsafe` | Banned constructs are rejected at L0 |
| `deps` | `none` | `el add dep` without `--allow` is rejected; `--allow` costs one policy line and one journal entry |
| `gate` | `check` | Default tier: `model`, `check`, `strict`, or `semantic` |
| `split` | `auto` | Body granularity: `auto` falls back to opaque, `always` refuses an unprovable split, `never` stores every new body opaque (FR-72) |
| `ra` | `auto` | `auto` uses `rust-analyzer` when present; `off` never spawns it (useful in CI and sandboxes) |
| `ra_timeout_ms` | `10000` | Per-request LSP timeout; `0` means unlimited (never use it in a loop) |
| `max_fn_lines` | `0` (off) | Soft warning when a function body exceeds this |
| `split` | `auto` | Body granularity: `auto` (split when clean), `always` (refuse bodies that cannot split), `never` (all opaque) |

The tool is therefore able to hold *this* repository to *this* repository's rules, which is
the dogfooding target in `PLAN.md` M11.

---

## 12. What `el` refuses to claim

Written down because a false promise here is worse than a missing feature.

1. **It cannot predict all errors statically.** L0 is heuristic; L1 is the compiler, and the
   compiler's answer is the only answer. A change that compiles may still be wrong.
2. **The gate cannot see runtime failures.** `el test` runs tests; it does not prove them.
3. **Diagnostic attribution is exact only for modelled elements.** In opaque modules it
   degrades to file+line, and the delta is then line-sensitive. Said plainly in the output.
4. **`--fast` is not verification.** It is a linter. The flag name says so, the log records it.
5. **Restoring bytes is not restoring intent.** `undo` returns the tree; it cannot un-run
   `el run`. Anything a program did to the world outside the projection is out of scope.
6. **`rust-analyzer` is a second opinion, not the arbiter.** L3 can finish before `rustc` would,
   and RA occasionally sees a different program than `cargo check` does (feature flags,
   cfg-gated code, proc macros). When the two disagree the compiler wins, and `--strict`
   runs both so the disagreement is visible rather than assumed away.
7. **RA may be absent, and that is a supported state.** Every RA-backed command names its
   fallback. A tool that silently degrades is a tool that lies about what it checked.
8. **A statement boundary is a guess, and the tool refuses to guess.** The splitter reads
   delimiters, strings and comments — not Rust grammar. Where the split is not provably
   lossless, the body is stored opaque and statement commands say so instead of inventing
   boundaries. `split = always` turns this from a fallback into a hard error for teams that
   want it.
9. **Statement ids are not stable across a hand-written rewrite.** If you adopt a file as
   opaque and later split it, the statement ids are new. Element ids are permanent only
   while the catalog is the authority.

---

## 13. Compliance with `AGENTS.md`

| Rule | How this SRD honours it |
|---|---|
| §2 test-first | Every FR/NFR names its test; `PLAN.md` orders work by RED-then-GREEN |
| §3 small functions | §3.1 budgets become `max_fn_lines` policy and a review checklist |
| §4 line ceiling | Enforced on `el`'s own source by `tools/check_line_budget.py` **and** on the user's crate by `el.policy` |
| §5 literate code | `NFR-7` requires a preamble per module; `HISTORY.md` and `ADR`s carry the reasoning |
| §6 delegation | `PLAN.md` §5 partitions milestones into single-writer slices |
| §8 DoD | `PLAN.md` §4 maps each milestone to a gate command |
| §9 boundaries | `ARCHITECTURE.md` states the dependency rule; `core_is_pure` enforces it mechanically |

---

## 14. Open decisions for the user

Each has a recommended default so work can start without an answer; each is cheap to
change now and expensive later.

| # | Question | Default assumed | Cost to change later |
|---|---|---|---|
| D-0 | `AGENTS.md` §0 describes ỌKỌ. Is `el` a second crate in this repo, or does this repo become `el`? | This repo becomes `el`: §0's `PROJECT` line is refilled in M0, ỌKỌ's text is archived verbatim in `docs/reports/oko-constants-archive.md`, and every command stays identical (ADR-0010). | Low now, high after M2 |
| D-1 | Is the name `el` final? (binary `el`, state dir `.el`, cache `~/.cache/el`) | Provisional; a rename is one `sed` over docs plus `Cargo.toml`. | Low until M3 |
| D-2 | History retention: keep every snapshot forever, or prune to the last 200 entries plus tagged ones? | Keep forever (a 30-module crate is < 200 KB per tree, deduplicated). | Low, `el doctor --prune` |
| D-3 | Should `el` write `Cargo.toml` at all, or require the user to bring their own? | Generate it; `--no-manifest` for users who own it. | Low |
| D-4 | Is network search in v1 or v2? | M11, and the first thing cut; everything before it works offline. | Low |
| D-5 | Default gate tier: `check` (compiler) or `model` (fast only)? | `check`. Slower, honest. `--fast` is one flag. | Low |
| D-6 | Persistent `rust-analyzer` session (a daemon with a socket, so `refs` skips the 1–3 s index) or ephemeral per command? | Ephemeral. A daemon is a second lifecycle, a socket and a stale-pid problem; RA's own disk cache makes the second session cheap enough. Flip if `NFR-13` misses in practice. | Medium — `HISTORY.md`-style lifecycle, but contained in one adapter |
| D-7 | Statement granularity on by default (`split = auto`) or opt-in? | On. It is the reason an agent can make a small change cheaply. Bodies the splitter refuses stay opaque automatically, so the downside is bounded. | Low — one policy key |
| D-8 | Should `el` auto-split an adopted opaque module? | No. Adoption preserves what was written; splitting is an explicit `el split <module>` (M12), because a wrong split is silent and a missing one is visible. | Low |

---

## 15. Traceability

| Use case | Requirements | Milestone |
|---|---|---|
| UC-1 | FR-1, 6, 13–16, 18, 44–46 | M6 |
| UC-2 | FR-19–24, 46 | M9 |
| UC-3 | FR-28–31, 34 | M8 |
| UC-4 | FR-8, 26 | M6/M7 |
| UC-5 | FR-40–43 | M11 |
| UC-6 | FR-10, 23 | M12 |
| UC-7 | NFR-1, 2, 6, 8 | M0 |
| UC-8 | FR-38, 39 | M6 |
| UC-9 | FR-49–52, 56–60 | M10 |
| UC-10 | FR-50, 53–54 | M10 |
| UC-11 | FR-62–68, 70 | M7 |
| UC-12 | FR-62, 66, 68 | M7 |

---

## 16. Deferred, and what keeps it cheap

| Deferred | Kept open by |
|---|---|
| Branches and merges | Journal entries are parent-linked; only the UI would change |
| `el import` with real parsing | Opaque modules already model "a file we don't understand"; a parser becomes an adapter, not a rewrite |
| Remote history (push/pull) | Trees are content-addressed; any transport can carry them |
| Multi-crate workspaces | Catalog is a crate; a workspace is a catalog of catalogs |
| `el fmt` via `rustfmt` | Rendering is already a pure function; a formatter is an alternative renderer |
| Watch mode | `el check` is idempotent and fast; a shell `inotifywait` loop is enough |
| `el split <module>` for adopted files | The splitter already exists; splitting is a modelling decision, not a new mechanism |
| Statement-level RA code actions | The LSP surface is already there; only the mapping to statement ids is new |
| Persistent RA daemon (D-6) | The session is one adapter; swapping `spawn-per-call` for `connect-or-spawn` changes no core type |
