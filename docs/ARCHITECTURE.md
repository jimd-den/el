# Architecture — boundaries, packages, and the files that will exist

> The dependency rule (§9 of `AGENTS.md`, Appendix A1): source dependencies point inward.
> `core` knows nothing about files, processes, terminals or `rust-analyzer`. `main` is the
> only file that names a concrete adapter.

---

## 1. The four layers

```text
        ┌──────────────────────────────────────────────────────────┐
  main  │ composition root: reads env, builds adapters, dispatches  │  only file that
        │                                                          │  names a concrete
        └───────────────┬──────────────────────────────────────────┘  adapter
                        │
        ┌───────────────▼───────────┐   ┌──────────────────────────┐
        │ adapters/                 │   │ cli/                     │  interface adapters
        │ fs · cargo · ra · curl    │   │ argv → Command           │  (I/O lives here)
        │ out_human · out_json      │   │ help · skill             │
        └───────────────┬───────────┘   └────────────┬─────────────┘
                        │ implements                 │ builds
        ┌───────────────▼────────────────────────────▼─────────────┐
        │ ports/  ── traits the inner layers declare and own ─────── │  dependency
        │ Fs · Tools · Clock · Out · Fetcher                        │  inversion seam
        └───────────────┬──────────────────────────────────────────┘
                        │
        ┌───────────────▼──────────────────────────────────────────┐
        │ app/  use cases: mutate · inspect · verify · history ·    │  orchestration only:
        │       search · doctor · dispatch                          │  no algorithms, no I/O
        └───────────────┬──────────────────────────────────────────┘
                        │
        ┌───────────────▼──────────────────────────────────────────┐
        │ core/  pure. model · plan · render · text · gate · history│  no fs, no process,
        │        · search index                                     │  no clock, no env
        └──────────────────────────────────────────────────────────┘
```

Crossing outward is a call; crossing inward is an interface the inner layer declares. Only
plain data crosses: `Catalog`, `Change`, `Projection`, `Diagnostic`, `Verdict`, `Tree`,
`JournalEntry`. No `PathBuf` reaching into `core` as a filesystem decision — paths are
`String` keys and the `Fs` port decides what they mean.

---

## 2. Enforced, not hoped for

Three meta tests make the picture above a fact (`tests/meta/`):

| Test | Asserts |
|---|---|
| `core_is_pure.rs` | `src/core/**` contains no `std::fs`, `std::process`, `std::net`, `std::env::current_dir`, `std::io::stdin` |
| `app_is_orchestration.rs` | `src/app/**` never names `crate::adapters::*`, and contains no `if` chain longer than the dispatch table |
| `zero_dependencies.rs` | `Cargo.toml` has no `[dependencies]`, no `[build-dependencies]`, and `Cargo.lock` has exactly one package |

A boundary that is not tested is a comment.

---

## 3. The tree

Sizes are the design budget (`AGENTS.md` §4: ≤ 200 per file, healthy band 25–200).
Estimated totals: ~6,000 lines of source across ~70 files, largest ~180.

Measured 2026-10-26 (M6, the files that exist): `main.rs` 122; `app/change` 175 + 88,
`app/transaction` 190 + 18 + 65 + 19, `app/verbs` 191, `app/projection` 123, `app/create`
94, `app/load` 58; `app/inspect` 122 + 77 + 44; `ports/journal` 60; `adapters/store` 188 + 47 + 87;
`core/text/grammar` 148 + 133; `core/model/body` 20; `cli/args` 141, `cli/parse` 136,
`cli/parse_read` 44. The tree below is still
the target for M7–M12; a line that names a file M6 has replaced is marked by the measured
sizes above.

```text
src/
  main.rs                      45   composition root; the only file that names adapters
  cli/
    mod.rs                     20   package preamble
    table.rs                   95   the single command/flag table: parse, help, skill, completion
    parse.rs                  150   argv + stdin → core::model::Change
    help.rs                    90   help text generated from table.rs
    skill.rs                   70   `el skill`, generated from table.rs (FR-47)
  core/
    mod.rs                     12
    model/
      mod.rs                   10   "what an element is; nothing here knows about text"
      id.rs                    55   ElId, alias, parse/format, ordering
      kind.rs                  70   Kind + per-kind rules (needs body? name? children?)
      element.rs              110   Element, Module, Visibility, BodyMode
      stmt.rs                 100   Stmt, nesting, binding, comment attachment
      catalog/mod.rs          170   Catalog: elements, statements, next_id, queries
      catalog/edit.rs         175   creation: id issue, alias uniqueness, statements
      catalog/identity.rs     160   removal, rename, move — the id-preserving edits
      catalog/codec.rs        165   the deterministic text form (ADR-0015)
      change.rs                85   Change enum: the complete mutation vocabulary
    plan/
      mod.rs                  127   plan(Catalog, Request) → Outcome | Violations (ADR-0017)
      apply_element.rs         73   add/edit/mv/rm/rename for items and modules
      apply_stmt.rs            94   add/edit/mv for statements; a cross-body move is a refusal
      validate.rs             165   L0 rules that read only the catalog
      rendered.rs             159   L0 rules that need the text: ceiling, fn budget, preamble
      request.rs               96   L0 rules about the ask: cross-body, dep, dangling ref
      violation.rs            161   Code (one per rule), Violation, non-empty Violations
      refs.rs                 162   reference index; rename rewrites, delete reports
      policy.rs               153   el.policy parsing, defaults, error with its line
    render/
      mod.rs                   86   Projection, `render`, ordering contract (FR-13)
      rust.rs                 137   one module's file: marker, preamble, mod lines, items
      rust_tests.rs            60
      body.rs                  92   statements back to text; opaque passthrough
      body_tests.rs           150
      tree.rs                 165   the module tree and its `mod` lines (FR-15)
      tree_paths.rs            74   path ↔ module name, and which file a child lives in
      tree_visibility.rs       42   widest visibility below a module (ADR-0016)
      tree_tests.rs           102
      spans.rs                124   line ranges → element ids, innermost wins (FR-17)
      style.rs                141   marker, indentation, and the line-counting `Out`
      manifest.rs              44   Cargo.toml, no dependency table unless declared (FR-16)
    text/
      mod.rs                    6
      scan.rs                 180   string/char/raw-string/comment/delimiter scanner
      ident.rs                144   where the identifiers that are *code* are (not in strings)
      balance.rs               92   whether a body's delimiters close, ignoring literals
      terminator.rs           140   where a statement may end: `;`, `}`, operator
                                   continuation (`else`, `as`, `.`)
      split.rs                110   lexical statement splitter (I6, FR-64, FR-65)
      json/mod.rs             145   Json tree, canonical writer, accessors
      json/parse.rs           170   strict reader: containers, dispatch, trailing text
      json/scalar.rs           90   literals; numbers validated and kept as source text
      json/string.rs          150   escapes, `\uXXXX`, surrogate pairs
      tsv.rs                  140   line-oriented record encode/decode with escaping
      lines.rs                  -   deferred: no call site yet; M3's renderer is the first
    gate/
      mod.rs                   40   tier selection, orchestration through ports
      diagnostics.rs          150   cargo JSON + LSP diagnostics → one Diagnostic type
      delta.rs                120   fingerprinting, baseline compare, three counts
      verdict.rs               80   Commit | Reject | Note
      budget.rs                90   file/function/statement budgets (FR-26, FR-73)
    history/
      mod.rs                   21   the package: blobs, trees, entries; pure
      sha256.rs               144   FIPS 180-4, NIST vectors, strict hex
      object.rs                84   digest, path = digest, `verify`, Corrupt
      tree.rs                 137   sorted `path⇥digest`, decode, symmetric diff
      entry.rs                174   ten fields, `el1` tag, sanitize, truncate
    search/
      index.rs                120   parse std docs index page → entries
      result.rs                70   Result type, ranking, dedup, limit
  ports/
    mod.rs                     20
    fs.rs                     109   read/write/remove/list/exists; crash-atomic write
    tools.rs                   70   run(argv, stdin, timeout) → Exit{code, stdout, stderr}
    clock.rs                   20   now_unix() — injected, never `SystemTime` in core
    out.rs                     70   Out trait: human / json implementations
    fetcher.rs                 30   fetch(url) → bytes, or a named "no fetcher" error
  adapters/
    fs_local.rs               155   tmp→fsync→rename, sorted listing, path containment
                                       (lock is adapters::lock; drift is app::projection)
    tools_proc.rs             150   std::process::Command, timeout kill, stderr quarantine
    cargo.rs                  120   check/clippy/test/build/run argv construction + capture
    ra/
      mod.rs                   30   "RA is a subprocess we speak LSP to; no ra_ap_* crates"
      frame.rs                110   Content-Length framing, headers, read/write
      client.rs               180   spawn, initialize, request/response correlation, timeouts
      edit.rs                 140   WorkspaceEdit → text edits, all-or-nothing validation
      session.rs              170   open files, pull diagnostics, refs, rename, code actions
    curl.rs                    90   $EL_FETCH or `curl -fsSL`, cache read/write, offline mode
    cache.rs                  120   ~/.cache/el: URL digest, TTL, atomic write
    clock_system.rs            25
    out_human.rs              150   tables, colours only on a TTY, notes to stderr
    out_json.rs               130   the stable envelope from COMMANDS.md §5
  app/
    mod.rs                     20
    dispatch.rs               100   Command → use case; universal flag handling
    mutate.rs                 160   the transaction (GATE.md §3), the only place that writes
    inspect.rs                110   ls/tree/show/explain/refs/catalog
    verify.rs                 120   check/test/build/run/fix
    history.rs                110   log/diff/undo/redo/revert
    search.rs                  90   search/doc
    doctor.rs                 120   health, repair, rebuild, adopt
tests/
  acceptance/                 one file per use case (SRD §6)
  integration/                real cargo / real RA / real fs, temp dirs
  properties/                 render determinism, split round-trip, catalog round-trip
                              (a subdirectory needs a main.rs to be a cargo target:
                              tests/properties/main.rs declares its modules)
  meta/                       zero_dependencies, core_is_pure, app_is_orchestration,
                              line_budget, preambles, no_telemetry
  fixtures/                   cargo-json/*.jsonl, ra/*.lsp, bodies/*.rs
                              (bodies/refused/*.rs are the splitter's refusal corpus:
                              each file is named for why it cannot be split)
tools/
  check_line_budget.py       ≤ 200 (it counts itself)
  footprint.sh
Makefile
el.policy
docs/
  SRD.md COMMANDS.md GATE.md HISTORY.md ARCHITECTURE.md PLAN.md
  adr/ reports/
```

---

## 4. Why these seams (and not others)

| Seam | Actor / rate of change | Cost if wrong |
|---|---|---|
| `core` vs `adapters` | The compiler-facing rules change rarely; the tools change constantly | A rule buried in an adapter cannot be tested without that tool — this is the seam that makes the fast suite possible |
| `ports/tools.rs` | Every external tool is the same shape: argv in, exit/stdout/stderr out | One fake drives cargo, RA and curl in tests (`GATE.md` §9) |
| `render/tree.rs` owns `mod` lines, not `rust.rs` | A `mod` line is a fact about the module tree, and the tree is derived from paths, not from the file being written | If they merge, the visibility rule (ADR-0016) has nowhere to live |
| `text/split.rs` separate from `render/body.rs` | The splitter is a lexical utility with its own invariant (I6); the renderer consumes its output | If they merge, the lossless property test has nowhere to live |
| `gate/delta.rs` separate from `gate/diagnostics.rs` | Parsing an external format changes when `cargo` changes; fingerprinting is ours | A cargo format change should break one file's fixtures, not the verdict logic |
| `history` as four files | Encode/decode, hashing, trees and entries are four different reasons to change | One `store.rs` would cross 200 lines and mix concerns |
| `ra/` as a package | LSP framing, request lifecycle, edit application and session policy are four things | A single `ra.rs` is 600 lines of three concerns |

Deliberately **not** seams: no `utils`, no `common`, no `traits/` grab bag, no `types.rs`
(the model is the contract module and is named after its concept), no `error.rs` holding
every error in the program (each layer returns its own result type; `app` maps to exit codes).

---

## 5. Module preambles

Every file opens with 3–8 lines in the domain's language (`AGENTS.md` §5.3). The preamble is
the seam test: if you cannot write it, the module has no single purpose yet.

```rust
//! Lexical statement splitting.
//!
//! A function body is stored as an ordered list of statements so a change can name one
//! line instead of a file. This module finds those boundaries using delimiter depth plus a
//! scanner that skips strings, raw strings, chars and comments — never Rust grammar.
//!
//! Deliberately not here: deciding whether a boundary is *meaningful*. If the split cannot
//! be proven lossless the caller stores the body opaque (`I6`); rendering lives in
//! `render::body`, and the scanner lives in `text::scan`.
```

---

## 6. Data flow of one command

```text
main        parse env, build adapters (Fs, Tools, Clock, Out, Fetcher)
cli::parse  argv → Command, or Usage error → exit 1
app::dispatch  acquire lock, recover inflight, load catalog, select use case
app::mutate    core::plan → Projection | Violations
               core::render → files + span map
               ports::Fs    → materialize
               ports::Tools → cargo / rust-analyzer
               core::gate   → Verdict
               ports::Fs    → commit or restore; journal entry
               ports::Out   → human or json envelope
```

`app::mutate` is the only module in the program that writes to the projection. Everything
else computes, reads or renders. That is what makes "a change either compiles or is not
applied" checkable by reading one file.

---

## 7. Dogfooding

From M11, `el.policy` in this repo sets `ceiling = 200`, `require_preamble = true`,
`forbid = unsafe`, `deps = none`, `gate = check`. The tool then holds itself to its own
rules: `el check` on `el` is part of `make check`, and a file that grows past 200 lines fails
the same gate a user's crate would fail. If the tool cannot keep its own source inside its
own policy, the policy is wrong or the source is.
