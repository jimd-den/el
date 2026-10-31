# Stepwise handoff contract and CTQs

Async dev means work proceeds in steps across time and processes. The
handoff is the state passed from step N to step N+1. If the handoff is
wrong, the next step repeats work, loses work, or acts on stale state.
This file states the contract and the Critical to Quality
characteristics (CTQs) — each one measurable, each one tested in
`test_stepwise.py` or measured by a bench run.

## Handoff contract (step N -> step N+1)

Carries:
- the task text, immutable across the run;
- the last WINDOW steps, each as (command -> truncated result);
- the total step count and the omitted-step count.

Does NOT carry:
- the full transcript, older tool outputs verbatim, or model reasoning.

Persistence:
- state is written atomically (temp file + replace) after every step;
- a resumed run loads the state file and its first prompt equals the
  uninterrupted run's next prompt, exactly.

## CTQs

Unit-testable (in `test_stepwise.py`):
1. Per-step token bound: prompt size stays flat as steps grow. A prompt
   built from 100 steps is barely larger than one built from 10 steps
   (only the omitted-count digits differ).
2. No lost steps: every executed action is persisted before the next
   step begins. Save/load round-trips exactly.
3. Resume fidelity: prompt(state reloaded from disk) == prompt(state in
   memory) for the same step.
4. One action per step: only the first tool call in a response is
   executed; the rest are ignored.
5. Retry survival: 429/5xx/network errors are retried with growing
   waits; Retry-After is honoured when present; nothing dies before
   MAX_RETRIES.
6. No-action discipline: a text-only reply that is not DONE records a
   no-action step instead of looping or dying.

Bench-measurable only (reported by a pilot or full run, not unit tests):
7. Pass rate per task.
8. Tokens per success (total tokens / passed tasks).
9. Wall-clock per task and share of it spent waiting on rate limits.
10. Retries per task.

## What is NOT a CTQ here

Pass rate alone. A harness that passes by burning unbounded tokens or
wall-clock time has failed CTQs 1, 8, and 9 even with a green check.
