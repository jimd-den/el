# What the SLM bench missed for token savings

Analysis of the 2026-10-27 bench (pi 0.73.1 + `laguna-xs-2.1:free`,
PR #13 binary, `tools/slm-bench.sh` attempt 2: 9/10 pass, 3.94M tokens
over 10 toy tasks, 117 error-stops). Ranked by cost, with what this
branch (ADR-0030) does about each.

## 1. No test idiom — the dominant tax (fixed here)

52 test-add attempts across attempt 2, zero test functions landed.
Task 02 had its deliverable in place and still failed: the agent
chased test scaffolding until the 900 s harness timeout killed it.
Attempt 1 was worse — a blunt test-first line appended to every
prompt killed two tasks outright, both burning their whole budget on
`add test`, batch JSON and `add mod tests` shapes el could not
express. Fix: the `test` kind (ADR-0030) plus a test-loop block on
the skill page stating the red/green steps. If the loop closes, the
chasing stops, and the turns it consumed disappear with it.

## 2. The skill page advertised things that do not exist (fixed here)

`test` and `use` sat in the kinds line; neither was an accepted
kind. Each attempt cost a refusal, and each refusal spawned
invented variants (batch objects shaped `{"tests": [...]}`, `add
mod tests`, `add stmt` with a whole test module as its body).
Fix: `test` is now real, `use` is removed from the line, and the
not-built line drops `test`. A docs-vs-code gap is a token tax:
every false line is a turn spent discovering it is false.

## 3. No loop-close instruction (partly fixed here)

Task 02 did not know it was done. The skill page named verbs but
never the loop's end condition. Fix: the test-loop block ends the
loop explicitly — cargo test passes means done. This only partly
closes the gap: a model can still wander after any task, but the
observed wandering was test-chasing, which fix 1 removes.

## 4. Invented batch/apply shapes (watch, not fixed directly)

Batch errors already name the item index and the problem, and the
top-level error already says the input must be a JSON array. The
observed inventions were attempts to express tests, not ignorance
of the array shape, so the kind should collapse them. If a bench
re-run still shows shape flailing, the next fix is one example
line on the skill page — not guessed now, because an example with
a wrong field would add a confusion instead of removing one.

## 5. Failure-output bloat (harness-side, partly addressed)

Repo-wide commands (`make check`, `cargo test` in the el repo)
dumped whole suites into context in the pilot; every later turn
then re-sent ~30K tokens of that history. The stay-in-crate rule
in the bench harness addresses the repo-wide half. el's own
outputs stay terse (`show --terse`, named refusals); what el
cannot do is stop a model re-listing after every refusal.

## 6. Free-tier 429 retries (not el's)

117 error-stops, nearly all upstream rate limits, dominate wall
time (tasks 01/02 took 801 s / 902 s). Retries cost time, not
tokens (error messages carry zero usage), but every retry is a
turn that re-sends context. Fewer turns is the only lever el
owns here, and it works through fixes 1–3.

## 7. Context re-send per turn (model-side, not el's)

Every turn re-sends the transcript, so transcript size is token
cost. el's lever is keeping its own outputs small, which it
already does; the transcript grows from the model's own
exploration, which fixes 1–3 shrink.

## What el already does right (do not break)

Named refusals that correct in one turn (visibility-in-signature,
twice in the bench), `first_actionable`, byte-exact undo repairing
a mis-targeted add, the 15-line fn budget holding structurally
(answer 1 line, shout 3 lines), the grouped skill page and the
name-to-id correction from PR #13. The token work is additive:
give the model the missing idiom and the true surface, and the
turns it currently burns on fighting disappear.
