# Prior art — what already exists, and the gap this design aims at

Research log for the design in `docs/SRD.md`. Each entry: what it does, what we take, what we
reject and why. Sources are the projects' own documentation and the CLI-design writing listed
at the end.

---

## 1. Structural / element-level editing

| Tool | What it is | Take | Reject |
|---|---|---|---|
| **ast-grep** ([docs](https://ast-grep.github.io), [rewrite notes](https://ast-grep.github.io/blog/tree-sitter-rust-rewrite)) | Tree-sitter-based structural search and rewrite; patterns over syntax trees | The insight that agents operate on **whole file snapshots**, and that a fresh parse per operation is the normal case, not a degraded fallback | We cannot take tree-sitter: a grammar + C core is a dependency ADR-0001 forbids. Also it *matches* structure; it does not give identity, history or a gate |
| **tree-sitter** ([docs](https://tree-sitter.github.io)) | Incremental, error-tolerant parser | Error tolerance: a half-written body should not crash the tool | Requires a vendored grammar and a C build; `el` refuses to guess instead (ADR-0013) |
| **difftastic** | Syntax-aware diff | The idea that a line diff is the wrong diff for code | Out of scope; `el diff` diffs element ids, which is cheaper and more precise for a modelled tree |

**Conclusion.** Structural editing tools know syntax but not *identity*. `el`'s contribution is
that every node has a stable id, so a change, a diagnostic, and a history entry can all name the
same thing.

---

## 2. Rust project tooling

| Tool | What it is | Take | Reject |
|---|---|---|---|
| **cargo-edit** ([repo](https://github.com/killercup/cargo-edit), now `cargo add`) | Edits `Cargo.toml` from the CLI | The ergonomic win of "name a thing, not a file"; the `name@req` argument shape | Manifest-only; no elements, no gate, no history |
| **rust-script / cargo-script** ([RFC 3424](https://rust-lang.github.io/rfcs/3424-cargo-script.html), [write-up](https://davidhamann.de/2026/01/30/writing-scripts-in-rust)) | Single-file packages with an embedded manifest; run a `.rs` as a script | Fast start for throwaway programs; sharing a `CARGO_TARGET_DIR` to reuse builds | A single file is a different product; `el` targets multi-module programs with structure. We do borrow the "no scaffolding ceremony" goal |
| **evcxr** | Rust REPL over a Jupyter kernel | The REPL-ability ideal (`AGENTS.md` §3.3) is the same instinct: make the unit small enough to try | A REPL needs incremental compilation; out of scope |
| **cargo-nextest** ([machine-readable output](https://nexte.st/docs/machine-readable/list)) | Test runner with stable JSON output | The discipline of a *documented, versioned* machine format; the reason `el` pins its envelope (FR-45) | — |
| **cargo_metadata** ([docs.rs](https://docs.rs/cargo_metadata)) | Library wrapping `cargo --message-format=json` | Confirms the message shapes we must parse; a reference for the field names | It is a crate (ADR-0001); we implement the subset we need |
| **rust-analyzer** | The Rust IDE engine, LSP server | Everything semantic: references, rename, hover, code actions, diagnostics (ADR-0011). It already contains the parser we refuse to write | Linking `ra_ap_*` (≈40 crates); requiring it to be installed |
| **cargo check `--message-format=json`** ([Cargo book](https://doc.rust-lang.org/cargo/commands/cargo-check.html)) | Machine-readable diagnostics, `reason`-tagged frames | The authoritative gate (L1) and the fixture format for our parser tests | Relying on exit code alone (101 tells you nothing about *this* change) |

**Conclusion.** The Rust ecosystem has excellent *builders* (`cargo`), excellent
*analysers* (`rust-analyzer`), and no tool whose unit of work is an element with identity and
history. That is the gap.

---

## 3. CLI design for agents

Sources: [The CLI Spec v0.2](https://clispec.dev/spec/v0.2), [agent CLI field notes](https://dockup.ai/blog/cli-design-for-ai-agents),
[practical checklist](https://dev.to/uenyioha/writing-cli-tools-that-ai-agents-actually-want-to-use-39no).

Nine rules we adopt verbatim, each mapped to a requirement:

| Rule | Where it lands |
|---|---|
| JSON on stdout, prose on stderr | FR-44 |
| Stable, documented exit codes as data states | FR-46 |
| `--dry-run` that emits a structured diff, not a sentence | FR-23 |
| Never prompt; a non-TTY stdin must not hang | FR-12 |
| Meaningful error codes for branching | COMMANDS.md §5 |
| Idempotent / retry-safe mutations | FR-11, FR-48 |
| Stable field names; removal is a breaking change | FR-45 |
| An audit trail of every mutation | FR-28, HISTORY.md |
| A companion `skill` doc so an agent does not brute-force `--help` | FR-47 |

Two rules we deliberately do **not** copy: "JSON by default" (humans are the primary user of a
terminal tool; `--json` is one flag), and "interactive REPL when invoked bare" (a REPL is a
second program).

---

## 4. Unix philosophy, applied

The relevant tradition is not "small programs" as an aesthetic but composition as a
correctness strategy. Applied here:

- **One thing well**: mutate-and-verify Rust programs. Not a build system, not an editor, not a
  package manager.
- **Text streams as the interface**: line-oriented TSV for state, one object per command for
  machines, stdout/stderr split, `|`-composable ids.
- **Compose, don't reimplement**: `cargo` builds, `rust-analyzer` analyses, `curl` fetches.
- **Fail loudly, write nothing**: exit codes are the contract; a partial write is a bug.
- **Mechanism, not policy, where possible**: `el.policy` is the user's policy file, not our
  opinion — which is also how `el` ends up able to enforce `AGENTS.md`'s own 200-line ceiling.

The tension worth naming: Unix tools are stateless filters, and `el` keeps state (a catalog, a
journal). The resolution is that the state is *content-addressed and rebuildable* — `doctor
--rebuild` regenerates the catalog from the journal, and a test asserts the bytes match. State
that can be recomputed is not a hidden global; it is a cache with a proof.

---

## 5. What we did not find, and therefore had to design

1. **A change-verification transaction** that renders, gates and restores, with the gate's
   failure path as the *tested* path (ADR-0004, ADR-0005).
2. **Diagnostic deltas keyed by element identity** rather than line numbers — possible only
   because the tool renders the file and therefore owns a span map.
3. **Statement-level identity** with a conservative lexical splitter (ADR-0012, ADR-0013).
4. **A history store that is not git and does not need git**, small enough to audit in one
   sitting (ADR-0006).
5. **An RA client with no RA crates**, degrading by name and exit code (ADR-0011).

Each of those is a small mechanism; none is novel in isolation. The design bet is that the
combination — identity + gate + history + zero dependencies — is a different tool from any of
the parts.
