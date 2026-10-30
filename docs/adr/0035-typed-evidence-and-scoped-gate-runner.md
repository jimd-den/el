# 0035 — Typed evidence and scoped gate runner

Status: accepted · Date: 2026-10-28

## Context

The Reliable SLM Loop Plan's Phases 1–2 ask for tool state that
does not depend on agent prose: a versioned JSON envelope, bounded
discovery, revision-bound reads, and runner verbs whose verdicts a
script can trust. This ADR records the five decisions where the
plan's shape met el's existing contracts and had to bend one way
or the other.

## Decisions

### 1. The envelope grows additively; old keys stay

`COMMANDS.md` §5 already promised `el`, `cmd`, `ok`, `error`,
`seq`, `changed`, `gate`, `notes`, `data`. The plan's required
fields (`schema_version`, `request_id`, `action`, `status`,
`refs`, `diagnostics`, `next`) are added alongside them, and
`data` is now always present (an empty object when the verb has
no payload) instead of omitted. Nothing old moves or renames, so
existing parsers keep working; the byte-golden tests pin the new
order, with `request_id` shape-checked rather than byte-pinned
because it carries the process id and wall-clock time.

`status` separates four things the old boolean blurred: `refused`
(validation, unknown target, model violation — nothing was
done), `blocked` (state or environment prevents the run:
`env.*`, `conflict.*`, `EL_ENV_UNAVAILABLE`, `EL_STALE_REF`),
`failed` (verification ran and failed: `gate.*`, `test.*`), and
`restored` (a successful history move). A script that treated
every non-ok response as "the code is wrong" was wrong about
refusals and blocks; the vocabulary says which.

### 2. `read` is a new verb; ADR-0020 stands

ADR-0020 deliberately chose drift-tolerant reads: `show`, `ls`
and `find` answer from the catalog, so a hand edit on disk does
not change the answer. The plan's `EL_STALE_REF` does not
retrofit onto those verbs — that would reverse a deliberate
decision under the cover of a feature. Instead `read` is a new
verb with opt-in pinning: an id or alias, an optional `--rev`
digest of the committed source slice, and `--lines` for a
window. The digest reuses the existing SHA-256 content
addressing, over the same `source_of` slice `show --source`
renders, so a digest can never drift from what `read` returns.
A stale pin exits 5 (`Conflict` — the tree changed under the
reader), not 6: the target is known, it is not unknown.

### 3. Scope is recorded, not used to filter a verdict — except for test

`--scope` names a selector the caller wants recorded in
`evidence.scope` (manifest, packages, targets, features,
toolchain, triple, requested scope, command). For `check` and
`lint` the command runs crate-wide whatever the scope names,
because cargo cannot limit rustc to one module; filtering the
reported diagnostics while keeping the crate-wide verdict would
make the two disagree, so the scope is recorded and the verdict
stays whole. For `test` the scope is cargo's own substring
filter, so it genuinely narrows execution — and it conflicts
with `--target`, which selects a different filter. `fmt` takes
no scope: formatting is file-oriented and el's module-to-file
mapping for a fmt scope is deferred, not guessed.

### 4. `lint` uses the project's own lints; `fmt` never writes

`lint` runs `cargo clippy --all-targets --message-format=json`
with no injected flags. The L2 gate's `-D warnings` is a gate
property, not a lint-verb property: a verb that silently enabled
restriction lints would report failures the project never asked
for. A lint run whose warnings exit zero passes with a warnings
count; errors fail. `fmt` runs `cargo fmt --all -- --check`
only; bare `fmt` is refused, and there is no fix mode, because
formatting writes require a distinct, previewed action and this
verb is evidence, not action.

### 5. Cargo output is parsed into diagnostics plus preserved lines

`parse_cargo_json` now returns a `Parsed` struct: the
diagnostics, the lines that were not JSON, and the run's exit
code. The old signature silently dropped non-JSON lines; the
new one carries them to `data.non_json_lines` (capped at 20),
so build-script chatter is visible beside the verdict instead
of vanishing. The `build-finished` rule is unchanged: only
`test result:` lines are test evidence, so a finished build is
never a passed suite. The `Parsed` shape flows through
`run_tier` to `check`; the commit path takes `.diagnostics`
and is otherwise untouched, and the gate keeps its own
`env.missing_tool` code — only the reporting verbs remap a
missing tool to `EL_ENV_UNAVAILABLE`, so a missing cargo is
never read as a verdict about the code.

## Deferred

- `EL_UNSUPPORTED_SYNTAX`, `EL_TDD_PHASE`, `EL_INVALID_RED`,
  `EL_TEST_LOCKED`, `EL_BUDGET_TRUNCATED`: named in the taxonomy
  and mapped in `status()` where they belong, but with no
  producer until Phases 3 and 5 build the verbs that raise them.
- Workspace scope: the package list is the one crate, because
  workspaces are outside el's model; a workspace-aware scope
  needs cargo metadata, not a guess.
- A `fmt` scope and a `fmt` write action: specified as
  distinct, previewed actions, not smuggled into `--check`.
