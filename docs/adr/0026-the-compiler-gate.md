# 0026 — The compiler gate (M9)

Status: accepted · Date: 2026-10-26
Context: M9 is the headline promise — a change that would not compile is
refused before commit (UC-2). The plan specified tiers, fingerprints,
baselines, and a report; building it forced nine decisions the plan left open
or got wrong.
Decision:
1. **The default tier flips for new crates.** `el new` seeds
   `gate = check`, not `gate = model`. `Policy::default()` already said
   `"check"`; the seed was the laggard. Existing crates keep their seeded
   policy until edited — no silent migration.
2. **Mutation-only gating; `run` keeps drift refusal.** The plan's §4.4
   said `check`/`test`/`build`/`run` gate first. In code, `run` has no gate
   call and `check`/`test`/`build` verbs do not exist. Gating `run` was
   rejected: after `check_drift`, disk equals the head projection, so the
   delta is empty by construction for projection files (ambient files like
   `build.rs` are the narrow exception, and `cargo run` surfaces those
   failures itself), while a check on every run costs 0.3–2 s. COMMANDS
   §4.4 is corrected instead of the code: `run` keeps drift refusal;
   `check`/`test`/`build`/`fix` are marked specified-but-unbuilt, and
   `check`'s future job is diagnostics reporting plus baseline writing, not
   refusal.
3. **Journal strings.** A commit records `entry.gate` as the tier string
   (`model` | `check` | `strict`); a rejection records
   `gate=rejected:<new count>`; a dry run records
   `gate=preview:<new count>`. Counts are always new/fixed/pre-existing;
   the rejection message counts new errors primarily.
4. **The scaffold is forced Model-tier.** `el new`'s scaffold renders a
   comment-only `main.rs`, which a real `cargo check` refuses with E0601.
   So the scaffold transaction runs with `fast: true`, and the E0601 enters
   the first real mutation's baseline as pre-existing — where it shows as
   `fixed: 1` when the user adds `fn main`.
5. **Baselines live on the Fs port** at `.el/diags/<hex tree digest>`, one
   tab-separated `el1` line per diagnostic with 11 fields (tier, code,
   level, el, owner, exact, path, line, col, message). A file that exists
   records a set, even when empty; a missing file is computed once from
   the previous tree (restore, run, record, re-materialize). The model
   tier never reads or writes baselines. The new tree's baseline is
   written before commit; a write failure aborts. Corrupt lines are
   skipped: the cache fails soft.
6. **Timeouts are environment errors.** `Tools::run_timed` (default
   delegates to `run`) plus a `timed_out` flag; the gate maps a timeout to
   `env.timeout`, exit 4. `$EL_CARGO_TIMEOUT`, default 120 s, read in
   `main.rs`. A cargo run that fails with zero parsed diagnostics is
   `env.tool_failed`, never a pass; a missing tool is `env.missing_tool`.
   Cargo exit 101 is not a verdict: it is the diagnostics that judge.
7. **Two deviations from GATE §4.1's letter, both recorded here.**
   (a) The fallback fingerprint uses the *normalized* message, not the
   raw message — normalization is what makes two runs comparable at all.
   (b) Type-parameter-name erasure was NOT attempted in `normalize`:
   erasure is position-stable, and over-erasing merges distinct errors.
   Normalization is whitespace collapse plus absolute-path tokens to
   `<path>`.
8. **The fallback fingerprint drops the line.** GATE §4.1's letter said
   `(code, path, line, message)`, but that contradicts the section's own
   principle ("line numbers are deliberately not in the fingerprint") and
   the delta module's invariant ("identity is the fingerprint, never the
   line"). The witness: the scaffold's E0601 re-points when the first
   `add mod` adds a line to `main.rs`, and the gate refused that
   legitimate change as "1 new error (1 fixed, 0 pre-existing)". The
   line remains display; identity is `(code, path, normalized_message)`.
   The test that pinned the old behaviour
   (`a_fallback_fingerprint_keeps_the_line_because_nothing_better_exists`)
   was replaced, with its distinguishing intent preserved in
   `a_fallback_fingerprint_changes_with_the_code_path_or_message`, and a
   delta-level regression test pins the E0601 case.
9. **Clippy codes arrive namespaced.** Fixtures confirm `clippy::needless_return`
   as the code; no manual prefixing is done.
Out of scope, recorded not forgotten: `--no-gate` (unimplemented; `--fast`
is the escape hatch, recorded `gate=model`); L3 / `--semantic` (policy
`gate=semantic` is refused `usage.unsupported`, never silently
downgraded); `check` / `test` / `build` / `fix` verbs.
Consequences: every mutation pays one incremental `cargo check` (0.82 s
measured warm on a 30-module crate, under the 2 s budget); a crate whose
tests build deliberately uncompilable programs must use `--fast` for those
steps — four existing acceptance tests were migrated exactly that way,
and the M6-era log assertion now expects `check` on add entries.
Known artifact, not a defect: `cargo check --all-targets` flags `fn main`
as dead code in test-target compilation, so a fresh crate's first gated
commit reports one dead-code warning; and rustc suppresses lints when
errors are present, so warning counts flicker when an error appears.
Only new-error blocking is load-bearing.
Revisit if: a real `check` verb lands (it takes over baseline writing and
reporting); M10 needs semantic diagnostics (L3 reopens tier selection).
