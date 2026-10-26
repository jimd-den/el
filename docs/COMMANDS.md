# `el` — command surface

**Contract document.** Every command, flag, exit code and JSON field below is a
compatibility promise (`FR-45`). Adding is allowed; removing or renaming is a breaking change
and needs an ADR.

Grammar: `el <verb> [subject] [flags]`. One level deep. No prompts, ever (`FR-12`).
Reads go to stdout; human chatter goes to stderr; `--json` puts exactly one object on stdout.

---

## 1. Universal flags

| Flag | Meaning |
|---|---|
| `--json` | One JSON object on stdout. All prose on stderr. Implies `--no-color`. |
| `--quiet` | Suppress human notes; errors only. |
| `--dry-run` | Mutating commands: perform the **full** transaction, always restore (`FR-23`). |
| `--yes` | Accepted everywhere for scripting symmetry; `el` never prompts, so it is a no-op. |
| `--no-color` | Disable ANSI. Also automatic when stdout is not a TTY. |
| `--root <path>` | Operate on a crate other than the current one. |
| `--fast` | Skip the compiler/semantic tiers; L0 only (`FR-24`). |
| `--strict` | Gate tier L2: L1 + clippy `-D warnings` + `cargo test --no-run` (`FR-25`). |
| `--semantic` | Gate tier L3: `rust-analyzer` diagnostics instead of `rustc` (`FR-53`). |
| `--verbose` | Stream external tool output (cargo, rust-analyzer) to stderr. |

---

## 2. Exit codes

| Code | Name | Meaning |
|---|---|---|
| 0 | ok | Done. Includes no-ops (`changed: []`). |
| 1 | usage | Bad flags, unknown verb, missing argument. Nothing was read or written. |
| 2 | violation | L0 model/policy rejection. Nothing was written (`FR-18`). |
| 3 | gate | A tier rejected the change (new diagnostics). Tree restored, head unchanged (`FR-21`). |
| 4 | environment | Missing `cargo`/`curl`/`rust-analyzer` where required, drift needing `--adopt`, network miss under `--offline` (`FR-42`). |
| 5 | conflict | Lock held, or a concurrent modification was detected (`FR-39`). |
| 6 | not found | Unknown element id, module, alias or history sequence. |
| 7 | internal | A bug. Includes the JSON `error.code`, a reproduction line, and asks for a report. |

`--json` always emits the envelope, including on failure, as the last line of stdout.

---

## 3. Element addressing

An element is addressed by any of:

| Form | Example | Notes |
|---|---|---|
| Id | `el-0007` | Permanent, assigned at creation (`FR-1`) |
| Alias | `core::plan::apply` | `<module path without src/ and .rs>::<name>` |
| Qualified alias | `fn/core::plan::apply` | Use when a name exists as two kinds |
| Bare name | `apply` | Accepted only if unambiguous; otherwise exit 2 listing candidates |

Statement ids (`el-0101`) work anywhere an id works. A statement's owner is shown by
`el show`.

---

## 4. Verbs

### 4.1 Create

```text
el new <path> [--lib] [--bin] [--edition 2024] [--policy] [--json]
el init [--force]
```

`new` writes a crate with `src/main.rs` (or `src/lib.rs`), an empty manifest, `el.policy`
with the `AGENTS.md` defaults (except `gate = model` until M9 adds L1), `.gitignore`, and
`.el/` seeded with entry seq 1 (`init`) followed by the root module's `add` entry — the two
entries are one command's history, not two commands. `init` adopts an existing directory:
everything under `src/` is imported as opaque modules (`ADR-0009`), so nothing is lost and
nothing is silently rewritten. `init` is not in M6.

### 4.2 Mutate

```text
el add <kind> <name> [--in <module>] [--after <id>] [--vis <v>] [--sig <text>]
                      [--body <file|->] [--doc <text>] [--attr <text>]... [--json]
el add mod <path> [--preamble <file|->]
el add dep <name>@<req> [--allow] [--features a,b]
el add stmt <owner> (--after <id>|--before <id>|--at <n>) --body <file|->

el edit <id> [--rename <name>] [--sig <text>] [--body <file|->] [--doc <text>]
             [--vis <v>] [--attr <text>]... [--body-mode split|opaque] [--json]
el rm <id>... [--cascade] [--json]
el mv <id> <module> [--after <id>] [--json]
el mv <stmt> --after <stmt>            # reorder inside one body (FR-66)
el apply [<file>|-] [--dry-run] [--json]
```

Kinds: `mod`, `fn`, `struct`, `enum`, `trait`, `impl`, `const`, `static`, `type`, `use`,
`test`, `stmt`.

`--body -` reads the body from stdin to EOF; `--body <file>` reads a file. Bodies are parsed
by `el` into one statement per line tree: a line ending `{` owns its block, the renderer
supplies the closing brace, non-tail statements gain `;`, a tail expression does not, and
unclosed `(`/`[` are closed, and a logical statement continues across lines while `(`/`[`
is open. `el` still does not parse Rust expressions; `else` branches and `};` are refused
as documented in ADR-0019.
`el add fn` without `--body` writes `todo!()` and says so. `add mod` without `--preamble`
writes one generated comment, because `require_preamble = true` would otherwise refuse the
module it just created.

`--at <n>` is zero-based among the current siblings. `add stmt` takes exactly one
anchor; `mv <stmt> --after <stmt>` reorders inside one body and refuses a cross-body
anchor. `add` prints the new id on stdout (`FR-6`). `edit` prints the ids it changed. `rm` prints
the ids it removed **and every remaining reference** (`FR-9`); dangling references are an
L0 violation unless `--cascade`, which removes them in the same transaction.

`--rename` on a function, struct, enum, trait or const uses `rust-analyzer`'s
`textDocument/rename` when available (`FR-51`); `--fast` uses the catalog's own reference
rewriting. The output states which one ran.

`apply` reads a batch script, one change per line, comments with `#`:

```text
add mod src/core/gate.rs
add fn run --in src/core/gate.rs --sig 'fn run(p: &Plan) -> Verdict' --body body.snippet
mv el-0004 src/core/gate.rs
edit el-0007 --rename apply_change
rm el-0011
```

The whole script is one transaction: one journal entry, one gate, all-or-nothing (`FR-10`).

### 4.3 Inspect

```text
el ls [--module <m>] [--kind <k>] [--opaque] [--json]
el tree [--statements] [--json]
el show <id> [--statements] [--source] [--history] [--json]
el explain <id> [--json]
el refs <id> [--json]
el catalog [--json]
```

| Verb | Answers |
|---|---|
| `ls` | What elements exist, with id, kind, alias, module, line count, body mode |
| `tree` | The three-level outline: crate → modules → items → statements |
| `show` | Everything about one element: id, alias, visibility, signature, doc, body mode, statement ids, owning sequence |
| `explain` | Why it exists: module preamble, doc text, RA hover when available (`FR-55`), callers, tests that name it, last 5 history entries |
| `refs` | Who refers to it, via RA (`"source":"ra"`) or the catalog (`"source":"catalog"`) (`FR-50`) |
| `catalog` | The complete machine model: every element, module, dep, policy key (`FR-4`) |

### 4.4 Verify

```text
el check [--strict|--semantic|--fast] [--json]
el test  [-- <cargo test args>] [--json]
el build [--release] [--json]
el run   [-- <program args>] [--json]
el fix   <id> (--list | --apply <code>:<n>) [--json]
```

`check` runs the gate against the current tree with no change and reports the delta against
the last committed entry. `test`, `build` and `run` run the gate **first** and refuse to
proceed if the current tree has new diagnostics relative to head (`--fast` skips).

`fix` lists or applies `rust-analyzer` code actions for diagnostics in that element
(`FR-54`). Applying is a normal transaction: snapshot, edit, gate, commit or restore.

### 4.5 History

```text
el log [-n <count>] [--attic] [--json]
el diff [<seqA>] [<seqB>] [--json]
el undo [<n>] [--json]
el redo [--json]
el revert <seq> [--json]
```

`log` lists entries newest first with seq, age, actor, op, target, gate verdict and message
(`FR-28`). `--attic` includes truncated future entries (`FR-30`). `diff` with no arguments
compares head with its parent.

### 4.6 Outside world

```text
el search <query> [--limit <n>] [--offline] [--json]
el doc <path> [--offline] [--raw] [--json]
```

Fetches via `$EL_FETCH` (default `curl -fsSL`), caches by URL digest for 7 days under
`~/.cache/el/` (`FR-40`, `FR-41`). `--offline` never spawns the fetcher and exits 4 on a
miss with the exact command that would warm the cache.

### 4.7 Self

```text
el doctor [--repair] [--rebuild] [--adopt <path>] [--json]
el skill
el version
```

`doctor` reports: crate root, `.el` health, head seq, store size, orphaned objects, lock
state, interrupted transaction, drift, and the status of each external tool
(`cargo`, `rust-analyzer`, `curl`) with path and version (`FR-49`).
`--repair` rolls back an interrupted transaction. `--rebuild` regenerates the catalog from
the journal and must reproduce it byte for byte (`FR-34`).
`--adopt` records a drifted file as an opaque module (`FR-37`).

`skill` prints a self-contained usage guide (≤ 150 lines) designed to be pasted into an
agent's context (`FR-47`). It is generated from the same table as `--help`, so it cannot
drift from the code.

---

## 5. JSON envelope

Success:

```json
{
  "el": "1",
  "cmd": "add",
  "ok": true,
  "seq": 42,
  "changed": [{"op": "add", "id": "el-0007", "alias": "core::plan::apply"}],
  "gate": {"tier": "check", "new": 0, "fixed": 1, "existing": 5, "verdict": "ok", "ms": 812},
  "notes": ["rust-analyzer 2026-10-13 used for rename"]
}
```

Failure:

```json
{
  "el": "1",
  "cmd": "add",
  "ok": false,
  "error": {"code": "gate.rejected", "exit": 3, "message": "1 new error"},
  "gate": {
    "tier": "check", "new": 1, "fixed": 0, "existing": 4, "verdict": "rejected",
    "diagnostics": [{
      "code": "E0308", "level": "error",
      "el": "el-0102", "owner": "el-0007",
      "path": "src/core/plan.rs", "line": 41, "col": 9,
      "message": "mismatched types: expected u32, found &u32"
    }]
  }
}
```

Stable fields (contract): `el`, `cmd`, `ok`, `error.code`, `error.exit`, `seq`, `changed[].op`,
`changed[].id`, `gate.*`, `diagnostics[].{code,el,path,line,message}`.

Error codes are namespaced and stable:
`usage.unknown_flag`, `model.duplicate_alias`, `model.ceiling`, `model.dangling_ref`,
`model.cross_body_move`, `gate.rejected`, `env.missing_tool`, `env.drift`, `env.offline_miss`,
`conflict.locked`, `notfound.element`, `ra.timeout`, `internal.panic`.

---

### 5.1 Read payloads

Read verbs put their verb-specific object under `data`; the envelope fields above
do not change. The shapes are stable in the same sense as the envelope.

```text
ls    data = [{id, kind, alias|null, module, lines|null, mode}]
tree  data = {crate, modules: [{path, module, module_id|null, items: [
              {id, kind, alias|null, statement_tree?}]}]}
show  data = {id, kind, alias|null, module, visibility, signature, doc|null,
              mode, sequence|null, statements: [id],
              statement_tree?, source?, history?}
```

`statement_tree` is `[{id, text, children: [...]}]` and appears only with
`--statements`. `source` appears only with `--source`; `history` only with
`--history`. `ls` orders by module path, then id; `tree` walks the renderer's
module tree; both are the order the files are written in.

---

## 6. Output conventions

- Ids are printed raw (`el-0007`), so `$(el add fn …)` composes.
- Tables use one space-padded header row; no box drawing, no ANSI when not a TTY.
- Every line `el` prints about a change names the element id, so a human and a log agree.
- Nothing is truncated silently: `el ls` on 300 elements prints all of them; paginate with
  `head`/`jq`, not with a built-in pager.
- `el` never writes to stdout in human mode anything a script would need to parse. If a
  value matters, it is in `--json`.
