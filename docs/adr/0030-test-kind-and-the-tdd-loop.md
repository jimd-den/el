# 0030 — The `test` kind and the TDD loop

Status: accepted · Date: 2026-10-27

## Context

The 2026-10-27 SLM bench (pi + `laguna-xs-2.1:free`, PR #13 binary) landed
9/10, but the model tried to work test-first 52 times and never landed a
test: `el add test` was refused (test is not among the accepted kinds),
while the skill page lists `test` among kinds and COMMANDS §4.2 lists it
and §4.4 specifies an `el test` verb as unbuilt. One task (02) had its
deliverable in place and still failed: the agent spent its whole budget
chasing test scaffolding until the harness timeout killed it. A blunt
test-first line appended to every task prompt killed two more tasks in
the earlier attempt the same way.

## Decision

1. **`test` is a kind.** Token `test`, alias `module::test/<name>`. It
   needs a name and a body, takes a block, cannot nest — the same
   answers as `fn`. It renders as a `#[test]` line over an `fn`
   declaration, in the module it was added to (`--in`), so a test for
   `util::answer` lives in `src/util.rs` next to it. No new module
   concept is needed, and the 15-line fn budget applies to it through
   `takes_block`, so tests stay small structurally.
2. **The declaration keyword is `fn`, not `test`.** `Kind::decl_keyword`
   returns `fn` for Test and `as_str()` otherwise, and both the
   signature split and the declaration use it. A sig starting with
   `#[test]` is therefore refused, as it was before: attributes are
   not part of a signature. The `#[test]` line is emitted by the
   renderer only, and `declaration()` stays single-line, so `show`,
   `explain` and `diff` keep working unchanged.
3. **The loop's red and green steps are `cargo test`, not `el check`.**
   Verified by hand: a `#[test]` fn with a failing assertion passes
   `cargo check` silently and fails `cargo test`. The compiler gate
   (L1 `cargo check`) therefore does not refuse a commit that adds a
   failing test — red is committable by design, not a gate rejection.
   The skill page says this outright, because the bench models kept
   reaching for `el check` to see a test fail.
4. **No body means `todo!()`, as for `fn`.** A test added without a
   body panics under `cargo test`: red by default, with no special
   case in the code.
5. **Skill page and COMMANDS are corrected in the same branch.** The
   kinds line drops `use` (never a kind, accepted nowhere) and keeps
   `test` (now real). The not-built line drops `test`. A test-loop
   block states the add-test usage and the cargo-test red/green
   steps. COMMANDS §4.2 drops `use`; §4.4 distinguishes the test
   kind (built) from the `el test` verb (specified, unbuilt).
6. **`el test` the verb is deferred, not built here.** The loop
   completes through `cargo test`; the verb would be a convenience
   wrapper with el-owned reporting, not a missing step. It stays
   specified-but-unbuilt, honestly, until the loop needs attribution
   or `first_actionable` for test failures.

## Alternatives rejected

- `#[cfg(test)] mod tests` as the rendered shape: needs nested
  modules, which el does not model (a module is a file), and it is
  the shape the bench models repeatedly failed to express.
- `#[test]` via `--attr`: the flag is not built, and attribute
  handling is a separate feature; the kind is the discoverable
  surface a small model can actually hit.
- `el test` now: scope. The loop works without it, and building it
  here would delay the kind the bench actually needs.
- A test sig starting with `test` or `#[test]`: no such Rust
  keyword exists, and the sig rule (keyword first) is what makes
  signatures addressable at all.

## Consequences

Test elements participate in `ls`, `tree`, `show`, `find`, `explain`,
`edit`, `rm`, `mv`, `batch` and `apply` through the existing generic
paths; `batch` accepts `"kind": "test"`. The gate is unaffected:
`cargo check` ignores test outcomes, so red stays committable and
green stays a compiler verdict plus a test verdict, each from its
own command. The skill page grows a few lines and stays under its
150-line ceiling. Revisit `el test` when a bench shows the loop
needing el-owned test reporting.
