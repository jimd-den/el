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

State as of 2026-10-25, and the only state a reader may trust: `make check` is green.
M0 is real (the budget checker fails on a planted 201-line file; the zero-dependency and
core-purity tests fail on planted violations). M1 is *types only* — no round-trip property,
no acceptance test. M2 is *partly* real — `split` exists and is property-tested, `json` and
`tsv` do not exist. M3–M12 do not exist: the commits that name them add empty modules, so
their milestone labels are claims, not evidence.

Every line of implementation is demanded by a test written first.

---

## 2. Milestones

| # | Milestone | Flips | State 2026-10-25 |
|---|---|---|---|
| M0 | Harness and project constants | `make check` exists and fails loudly on a planted violation | ✅ green, violations proven |
| M1 | The model | a catalog can be built, addressed and round-tripped | ◐ types only |
| M2 | Text toolkit | bodies split losslessly, JSON is read and written | ◐ splitter only |
| M3 | Rendering | a catalog becomes Rust files and a span map | ☐ stub |
| M4 | Plan and L0 | an impossible change is refused with nothing written | ☐ stub |
| M5 | The store | every projection is content-addressed and restorable | ☐ stub |
| M6 | Transaction and first verbs | **UC-1**: a program built from five commands | ☐ stub |
| M7 | Statements | **UC-11/UC-12**: one line changes, nothing else moves | ☐ stub |
| M8 | History verbs | **UC-3**: review and rollback | ☐ stub |
| M9 | Compiler gate | **UC-2**: told before commit that it would not compile | ☐ stub |
| M10 | Semantic layer | **UC-9/UC-10**: RA rename, refs, quick fixes | ☐ stub |
| M11 | Search and doc | **UC-5**: std lookup without leaving the terminal | ☐ stub |
| M12 | Agent surface, dogfood, polish | **UC-6/UC-7**: batch, skill, self-compliance | ☐ stub |

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

**Files.** `core/model/{id,kind,element,stmt,catalog,change}.rs`.

**Done when.** `cargo test -q -- --test-threads=1` green in < 2 s; no file over 120 lines.

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

---

### M3 — Rendering

**Goal.** `render(catalog) → files + span map`, deterministic, byte-stable.

**RED first.** `tests/properties/render_is_deterministic.rs`: two catalogs built by inserting
the same elements in different orders render to identical bytes (FR-13). Then
`tests/acceptance/every_rendered_module_compiles.rs`: for five catalogs, the rendered crate
passes `cargo check` (this one is `#[ignore]`-free but lives in `integration/`).

**Files.** `core/render/{rust,body,manifest,spans,style}.rs`.

**Done when.** The rendered `src/main.rs` for a two-module catalog is byte-identical to the
checked-in golden file, and changing one statement changes exactly one line.

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

---

### M6 — Transaction and first verbs — *first usable release*

**Goal.** `el new`, `el init`, `el add`, `el edit`, `el rm`, `el mv` with L0 gating, the lock,
the inflight marker, drift detection, adoption, and the JSON envelope.

**RED first.** `tests/acceptance/author_builds_program_from_nothing.rs` (UC-1). It runs the
real binary in a temp dir. It is red from M0. On the way down, these units:

| Unit | Behaviour |
|---|---|
| `app::mutate` | step order is observable through a fake `Fs` that records calls (GATE.md §3) |
| `adapters::fs_local` | lock is exclusive; stale lock reclaimed; inflight triggers restore |
| drift | hand edit → exit 4 + `--adopt`; whitespace edit → re-render + one stderr line |
| `out_json` | envelope matches `COMMANDS.md` §5 exactly (golden) |
| exit codes | 0, 1, 2, 4, 5, 6 each reachable by one test |

**Done when.** UC-1 passes end to end: five commands, `cargo run` prints `hello, world`,
`el log` shows five entries all `gate=model`.

**Cut if needed.** `init --force` and dependency verbs slip to M12; nothing else.

---

### M7 — Statements

**Goal.** Statement ids everywhere, and the budget that makes the ceiling actionable.

**RED first.** `tests/acceptance/edit_replaces_one_statement_only.rs` (UC-11): capture all
statement bytes, edit one, assert the others are unchanged **and** that no line numbers moved
for the statements above.

| Unit | Behaviour |
|---|---|
| `plan::apply_stmt` | add by anchor, rm, reorder; ids stable across reorder |
| `text::split` | wired into body creation; `split = auto/always/never` honoured |
| `gate::budget` | per-file and per-function budgets; ceiling message lists extractable statement ids |
| `model::stmt` | comments and attributes attach and move with their statement |

**Done when.** UC-11 and UC-12 pass; `el tree --statements` prints the three-level tree.

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

| Unit | Behaviour |
|---|---|
| `el apply` | batch parser, one transaction, all-or-nothing |
| `el skill` | generated from `cli::table`, ≤ 150 lines, pinned by a golden test |
| `el explain` | preamble + docs + callers + tests + history |
| dogfood | `el check` on `el` is part of `make check`; `el.policy` applies to this repo |
| footprint | `tools/footprint.sh` reports a stripped binary ≤ 1.5 MB |

**Done when.** `make check` green with `el` checking `el`, and `docs/reports/m12-dogfood.md`
records the first session where `el` edits its own source.

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
