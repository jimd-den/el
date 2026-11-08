# 0034 — Trust repairs and test execution as evidence

Status: accepted · Date: 2026-10-28

## Context

The Reliable SLM Loop Plan's Phase 0 audited four places where
el's surface lied to its caller, each verified hands-on against
the `feat/skill-examples` tree before the fix:

1. An element past 9,999 was unaddressable. `ElId::parse`
   required exactly four digits while `Display` pads to a
   *minimum* of four, so element 10,000 displays as `el-10000`
   and cannot be parsed back. The allocation counter is an
   uncapped u32, so this was a matter of time, not of scale.
2. An apply failure after blank or comment lines named the
   wrong line. `script::parse` dropped blank/comment lines
   and `build` re-enumerated the stripped list, so a failure
   on physical line 3 was reported as line 1. Parse errors
   already carried physical lines; build-stage failures did
   not.
3. The `test` verb was specified but unbuilt while the test
   *kind* existed, so the TDD loop's red and green steps had
   no el verb: the loop ran `cargo test` directly, and a
   failing test was indistinguishable in el's surface from a
   compile error.
4. `--cascade` was documented as working (skill page flag
   line, `COMMANDS.md` §4.1) while the runtime refuses it.
   `--semantic` was already honestly marked "(not built)".

## Decision

1. **Variable-width ids.** `ElId::parse` accepts any
   non-empty digit run after `el-`; `Display` still pads to
   a minimum of four, so the canonical spelling is unchanged
   and short spellings (`el-11`) are the same id, not a
   different one. The old test pinning `el-11` as malformed
   changed with the rule: its intent (malformed ids are
   refused) is preserved by the empty, non-digit, and
   prefix-less cases.
2. **Physical apply lines.** `script::parse` returns each
   kept line tagged with its physical line number, and
   `build` reports that number, so origins — and therefore
   gate-failure attribution — are file lines, not positions
   among the changes.
3. **`el test` is on-demand evidence, not a gate tier.**
   It runs `cargo test` (full suite by default or `--all`,
   one test element by id with `--target`) and reports
   passed/failed counts plus the executed tests, in prose
   and in `--json` `data`. It never refuses a commit and
   never changes what the gate checks, so the per-commit
   "no new errors" scope is untouched.
   - A failing test exits 3 with code `test.failed`:
     the verification bucket `check` already uses, with a
     distinct code so a script can tell it from a gate
     rejection. The exit table is not extended in this
     change; a dedicated test-failure exit is deferred to
     the envelope work.
   - A suite that does not compile exits 3 with code
     `test.compile_failed` and next `el check`, so a
     compile error never masquerades as behavior evidence.
   - Missing cargo is `EL_ENV_UNAVAILABLE` (exit 4),
     never a test verdict.
   - `--target` filters by the test's bare name (a
     substring filter, so it matches in any module);
     `--target` with `--all` is a usage conflict.
   - Like `run`, a drifted catalog is refused first, so
     executed tests are never silently misattributed to
     catalog elements they no longer match.
4. **Cascade stays unbuilt and now says so.** The skill
   page flag line and `COMMANDS.md` mark `--cascade`
   "(not built)". Implementing cascade removal is a
   feature, not a trust repair, and is not claimed here.

## Consequences

- Every id el prints parses back, at any scale.
- Every apply failure names the line the human sees.
- The TDD loop has an el verb for both its steps, and
  test evidence is typed (`passed`, `failed`, executed
  names) rather than scraped from `cargo test` output.
- The documented surface and the dispatch agree on
  `--cascade` and `--semantic` alike.
