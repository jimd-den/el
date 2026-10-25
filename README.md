# `el`

A Unix-way tool for building Rust programs **by element** — modules, items, and the statements
inside a function body — where every change is checked by the compiler *before* it is applied,
every change can be undone exactly, and the binary depends on nothing.

```console
$ el new hello && cd hello
$ el add fn main --sig 'fn main()' --body - <<<'println!("hello");'
el-0002  added to src/main.rs
el: gate ok (L1 check, 0 new diagnostics, 412 ms)
$ el run
hello
$ el undo && el log
```

**Design status: draft, first modules implemented.** The model types and the text splitter
exist and are tested; the verbs, the store, the gate and the history are specified but not
implemented. `make check` is green. The console transcript above is the target, not a claim.

---

## Documents

| File | What it is |
|---|---|
| `docs/SRD.md` | The requirements document: problem, actors, use cases, 73 functional and 15 non-functional requirements, each with the test that will prove it, plus the honesty limits |
| `docs/COMMANDS.md` | The command surface: every verb, flag, exit code and JSON field — the compatibility contract |
| `docs/GATE.md` | How a change is verified before commit: four tiers, the transaction, the diagnostic delta, what it cannot know |
| `docs/HISTORY.md` | The catalog, the journal, content-addressed snapshots, undo/redo/revert, crash recovery |
| `docs/ARCHITECTURE.md` | The clean-architecture layout: layers, the dependency rule, the ~41 files that will exist, and how the boundaries are enforced by tests |
| `docs/PLAN.md` | Thirteen milestones from an empty directory to a self-hosting tool, with the red test that starts each one, a cut list, and risks |
| `docs/adr/` | Fourteen decision records: zero dependencies, catalog-as-truth, stable ids, transactional apply, error delta, history store, network-as-subprocess, hand-rolled parsing, projection ownership, project constants, `rust-analyzer` as an LSP subprocess, statements as elements, the lexical splitter, symbol-tree import |
| `docs/reports/prior-art.md` | What already exists (cargo-edit, rust-script, ast-grep, rust-analyzer, cargo-nextest, agent-CLI writing), what we take, what we reject |

---

## The three promises

1. **Verified before committed.** A change either compiles or is not applied. `--dry-run` is a
   true preview because it is the same transaction, restored.
2. **Exactly reversible.** Content-addressed snapshots; `el undo` restores bytes, not intent
   (and says so).
3. **Nothing to install but one file.** Zero crates, `std` only. `cargo` is required;
   `rust-analyzer` and `curl` are optional and degrade by name, never silently.

## What it refuses to do

Parse Rust. Replace your editor. Be a build system. Guess at a statement boundary it cannot
prove. Claim that "compiles" means "correct". Each refusal is written down with its escape hatch
in `docs/SRD.md` §3.2 and §12.

---

## Working on this

Governing standard: `AGENTS.md`. The gate:

```bash
make check                          # budget + test + lint + typecheck
cargo test -q -- --test-threads=1   # fast suite, < 10 s
python3 tools/check_line_budget.py  # 200 lines, no exceptions
```

Nothing in `src/` may exist without a failing test that demanded it (`AGENTS.md` §2.1).
