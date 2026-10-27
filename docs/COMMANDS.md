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
el new <path> [--lib] [--edition 2024] [--policy] [--json]
el init [--force]
```

`new` writes a crate with `src/main.rs` (or `src/lib.rs`), an empty manifest, `el.policy`
with the `AGENTS.md` defaults, `.gitignore`, and
`.el/` seeded with entry seq 1 (`init`) followed by the root module's `add` entry — the two
entries are one command's history, not two commands. `init` adopts an existing directory: everything under `src/` is
imported as one opaque module per file (`ADR-0009`), so nothing is lost and nothing is
silently rewritten. It writes no projection: it verifies the render is byte-for-byte what is
on disk before recording the tree. `el.policy` and `.gitignore` are written only if absent, and
the manifest is never rewritten. Element-level `add`/`mv` into an adopted file is refused
(`model.opaque`); edit the whole file with `el edit <module> --body <file> --body-mode opaque`.
`--force` and `--dry-run` are refused.

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

Kinds: `mod`, `fn`, `struct`, `enum`, `trait`, `impl`, `const`, `static`, `type`,
`test`, `stmt`.

`--body -` reads the body from stdin to EOF; `--body <file>` reads a file. Bodies are parsed
by `el` into one statement per line tree: a line ending `{` owns its block, the renderer
supplies the closing brace, non-tail statements gain `;`, a tail expression does not, and
unclosed `(`/`[` are closed, and a logical statement continues across lines while `(`/`[`
is open. `el` still does not parse Rust expressions; `else` branches and `};` are refused
as documented in ADR-0019.
`el add fn` without `--body` writes `todo!()` and says so. `el add test` without `--body`
writes the same `todo!()`, which panics under `cargo test`: red by default. `add mod` without `--preamble`
writes one generated comment, because `require_preamble = true` would otherwise refuse the
module it just created.

`--at <n>` is zero-based among the current siblings. `add stmt` takes exactly one
anchor; `mv <stmt> --after <stmt>` reorders inside one body and refuses a cross-body
anchor. `add` prints the new id on stdout (`FR-6`). `edit` prints the ids it changed. `rm` prints
the ids it removed **and every remaining reference** (`FR-9`); dangling references are an
L0 violation unless `--cascade`, which removes them in the same transaction.

`--rename` rewrites the catalog's own references: whole identifiers only, in statement
text and element heads, with strings, comments, char literals and lifetimes untouched. It is
a token rewrite, not name resolution — when the renamed element is not a module, the
qualifier in `greet::call()` and the member in `plan.apply()` are left alone; when it is a
module, only qualifier sites are rewritten. A rename the rewrite could not complete is
refused by the compiler gate, which is why an incomplete one is visible rather than silent.
`rust-analyzer`-backed `textDocument/rename` (`FR-51`, `FR-52`) is scheduled for M10 as
`ADR-0011` describes it and is **not built**: the binary has no LSP adapter.

`apply` reads a batch script, one change per line, comments with `#`:

```text
add mod src/core/gate.rs
add fn run --in src/core/gate.rs --sig 'fn run(p: &Plan) -> Verdict' --body body.snippet
mv el-0004 src/core/gate.rs
edit el-0007 --rename apply_change
rm el-0011
```

The whole script is one transaction: one journal entry (op `apply`), one gate, all-or-nothing
(`FR-10`). Every line goes through the same parser as argv, so a script and a command line
cannot disagree about a flag. A line may carry `add`, `edit`, `mv` or `rm`; global flags belong
to `apply` itself, `--body -` is refused (the script owns stdin), and an id must exist when the
script starts — an element added on an earlier line has no id until planning, so edit it in a
later `apply` rather than guessing. A plan refusal names the line: `apply line 3: change 2 …`.
A gate rejection restores the tree and journals one `rejected:<n>` attempt, as `batch` does.

`batch` is `apply`'s JSON sibling, built: it reads a JSON array from stdin, one
object per `add` (`kind`, `name`, `in`, `vis`, `sig`, `body`; a module item's
`body` becomes its preamble), and commits all items under one sequence —
or nothing. A plan failure names its change (`change {i} ({kind} {name} in
{module})`); a gate rejection restores the tree and journals one
`rejected:<n>` attempt. Statements are refused (`usage.unsupported`):
they need ids that do not exist until planning. `--dry-run` previews the
whole batch; `--fast` and `--strict` pass through as for `add`.

### 4.3 Inspect

```text
el ls [--module <m>] [--kind <k>] [--opaque] [--json]
el tree [--statements] [--json]
el show <id> [--statements] [--source] [--history] [--terse] [--json]
el find <query> [--json]
el explain <id> [--json]
el refs <id> [--json]
el catalog [--json]
```

| Verb | Answers |
|---|---|
| `ls` | What elements exist, with id, kind, alias, module, line count, body mode |
| `tree` | The three-level outline: crate → modules → items → statements |
| `show` | Everything about one element: id, alias, visibility, signature, doc, body mode, statement ids, owning sequence. A statement id shows its owner, parent, index and text instead. `--terse` collapses the header to one line (`<id> <kind> <alias> <module> <signature>`, or `<id> stmt owner <owner> parent <parent> <text>` for a statement) and lists statement bodies under it; `--source` / `--history` sections and the JSON payload are unchanged. |
| `find` | The ids for a name, without the whole-crate dump: a query matches a named element when it equals its bare name, its alias, or its `module::name` path. Outline lines only (`<id> <alias>`), no bodies. No match is `notfound.element`, exit 6. |
| `explain` | Why it exists: module preamble, doc text, RA hover when available (`FR-55`), callers, tests that name it, last 5 history entries |
| `refs` | Who refers to it, via RA (`"source":"ra"`) or the catalog (`"source":"catalog"`) (`FR-50`) |
| `catalog` | The complete machine model: every element, module, dep, policy key (`FR-4`) |

`explain` is built and catalog-only: module preamble, doc text, whole-word callers, functions
named like tests, and the last five entries. It prints `source  catalog (rust-analyzer not
built)` because it has not looked at the semantics.
`refs` and `catalog` are specified but unbuilt: the CLI has no such verbs yet. `find` covers
the "which ids answer to this name" question until `refs` lands.

### 4.4 Verify

```text
el check [--strict|--semantic|--fast] [--json]
el test  [-- <cargo test args>] [--json]
el build [--release] [--json]
el run   [-- <program args>] [--json]
el fix   <id> (--list | --apply <code>:<n>) [--json]
```

`check` is built: it runs the tier the policy selects (or `--fast`/`--strict`), reports every
diagnostic the compiler gave, and writes nothing — reporting, not refusal (ADR-0026). It exits
3 when an error is present, 0 otherwise. With no catalog (a plain Cargo crate) diagnostics are
un-attributed and the page says so; with a catalog the same span map the gate uses attributes
them. `gate.new` and `gate.fixed` are 0 because there is no baseline comparison yet; the set is
in `gate.diagnostics` and counted in `gate.existing`. Baseline writing after `check` is
deferred.
`build` and `fix` are specified but unbuilt: the CLI has no such verbs yet. `test` exists
as a kind (see §4.2); only the `el test` verb is still unbuilt. The loop's red and green
steps are `cargo test`, not `el check`: the gate runs `cargo check`, which does not run
tests, so a failing test commits. `run` does
**not** gate first — after drift refusal, disk equals the head projection, so a gate would
judge an empty delta; ambient files outside the projection (such as `build.rs`) are surfaced
by `cargo run` itself.

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
miss with the exact command that would warm the cache. `search` and `doc` are specified but
unbuilt: the CLI has no such verbs yet.

### 4.7 Self

```text
el doctor [--repair] [--rebuild] [--adopt <path>] [--json]
el skill
el version
```

`doctor` reports: crate root, store version, head seq, journal and attic size, whether a
transaction was interrupted, drift between the catalog and the projection (read-only — it
never re-renders), and how many blobs the store holds. It does not report lock state,
because it holds the lock while it runs, and it does not yet report external tool paths and
versions (`FR-49`, M10). Orphaned-object reporting is specified and unbuilt.
`--repair` rolls back an interrupted transaction. `--rebuild` regenerates the catalog from
the journal and must reproduce it byte for byte (`FR-34`).
`--adopt` records a drifted file as an opaque module (`FR-37`).

`skill` prints a self-contained usage guide (≤ 150 lines) designed to be pasted into an
agent's context (`FR-47`), verbs grouped by task (write, read, history, verify, meta)
so selection is by intent; `--help` prints the same table. An unknown verb names the
verbs that exist rather than pointing at a command that does not.

`version` prints `el <package version>` from `Cargo.toml`. The milestone is not in the
binary: it lives in `README.md` and `docs/PLAN.md`, and a number kept in two places is a
number that disagrees.

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
  "notes": ["el: rewrote 1 file"]
}
```

Failure:

```json
{
  "el": "1",
  "cmd": "add",
  "ok": false,
  "error": {"code": "gate.rejected", "exit": 3, "message": "1 new error", "hint": "el diff"},
  "gate": {
    "tier": "check", "new": 1, "fixed": 0, "existing": 4, "verdict": "rejected",
    "diagnostics": [{
      "code": "E0308", "level": "error",
      "el": "el-0102", "owner": "el-0007",
      "path": "src/core/plan.rs", "line": 41, "col": 9,
      "message": "mismatched types: expected u32, found &u32", "exact": true,
      "first_actionable": true
    }]
  }
}
```

Stable fields (contract): `el`, `cmd`, `ok`, `error.code`, `error.exit`, `error.hint`, `seq`,
`changed[].op`, `changed[].id`, `gate.*`,
`diagnostics[].{code,level,el,owner,path,line,col,message,exact,first_actionable}`.
A diagnostic the compiler gives no code for — a parse error — reports `uncoded`, never an
empty string, because `code` is part of the diagnostic's identity.

Error codes are namespaced and stable:
`usage.unknown_verb`, `usage.unknown_flag`, `usage.unknown_kind`, `usage.missing_verb`,
`usage.missing_argument`, `usage.missing_value`, `usage.repeated_flag`, `usage.bad_value`,
`usage.unexpected_argument`, `usage.unsupported`, `usage.no_change`, `usage.bad_batch`,
`usage.bad_script`,
`usage.exists`, `model.duplicate_alias`, `model.unknown_ref`, `model.cross_body_move`,
`model.unbalanced`, `model.bad_ident`, `model.ceiling`, `model.fn_budget`, `model.preamble`,
`model.forbidden`, `model.dep_denied`, `model.dangling_ref`, `model.bad_stmt`, `model.opaque`,
`gate.rejected`,
`gate.would_reject`, `gate.failed`, `env.not_a_crate`, `env.not_a_project`, `env.drift`, `env.bad_policy`, `env.io`, `env.corrupt`,
`env.missing_tool`, `env.tool_failed`, `env.timeout`, `env.run_failed`, `env.offline_miss`,
`conflict.locked`, `notfound.element`, `notfound.sequence`, `ra.timeout`, `internal.panic`.

Every refusal names a next command. Human output prints it on a `next: el …` line; the
envelope carries it as `error.hint`. The hint is checked by a test that parses it with the
same parser the user's next command will use, so a hint cannot name a command `el` refuses.

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
find  data = [{id, kind, alias|null, module}]
explain data = {id, kind, alias|null, module, signature, preamble|null, doc|null,
              callers: [{id, alias, text}], tests: [{id, alias}],
              history: [{seq, op, gate, msg}], source}
```

`statement_tree` is `[{id, text, children: [...]}]` and appears only with
`--statements`. `source` appears only with `--source`; `history` only with
`--history`. `ls` orders by module path, then id; `tree` walks the renderer's
module tree; both are the order the files are written in. `find` orders by
module path, then id, like `ls`.

---

### 5.2 History payloads

```text
log    data = [{seq, ts, actor, op, target, from, to, gate, msg}]
diff   data = {from, to, elements?: [{op, id, kind|owner, alias?, lines: [
               {op, text}]}], files: [{path, lines: [{op, text}]}]}
undo/redo/revert  data = the new or restored entry
```

`from` and `to` are content digests. `diff` compares the older sequence to the
newer: no arguments is parent → head, one argument is that sequence → head, two
are `a → b`. `elements` is present only when both sequences have a catalog
snapshot; `files` is always present.

---

## 6. Output conventions

- Ids are printed raw (`el-0007`), so `$(el add fn …)` composes.
- Tables use one space-padded header row; no box drawing, no ANSI when not a TTY.
- Every line `el` prints about a change names the element id, so a human and a log agree.
- Nothing is truncated silently: `el ls` on 300 elements prints all of them; paginate with
  `head`/`jq`, not with a built-in pager.
- `el` never writes to stdout in human mode anything a script would need to parse. If a
  value matters, it is in `--json`.
