# 0038 — Stepwise harness in tools/

Status: accepted · Date: 2026-10-31

## Context

The bench moved from a transcript-re-sending loop to a stepwise
runner: each API request carries a fresh context (system prompt +
task + the last 5 steps), never the transcript, so per-step tokens
stay flat no matter how long the task runs. The runner lived only
as workspace scratch while it piloted task 08 and the todo project.
This ADR lands it in the repo so bench runs are reproducible from
a checkout, and records the three decisions its shape forced.

## Decisions

### 1. It lives in tools/, beside slm-bench.sh

Bench tooling, not el source and not a Rust test: it drives el
from outside, through the CLI surface PR2 built (status, discover,
revision-bound read, evidence verbs). The reason the window works
at all is el as external memory — the catalog re-reads state
cheaply, so the model can re-discover anything older than the
window instead of carrying it in context.

### 2. The split seam is the network boundary

`stepwise-agent.py` was 229 lines, 29 over the ceiling. The seam
is responsibility, not length: `stepwise_api.py` owns the
OpenRouter request, retries, and backoff; `stepwise-agent.py`
keeps the step loop, prompt building, and state persistence.
Sizes after the split: 169 + 83, both inside the 25–200 band.
`test_stepwise.py` loads the entry by filename, so the entry
keeps its name and re-exports `backoff`/`MAX_DELAY`; the CTQ
tests are unchanged. Two structure checks pin the split itself:
entry within budget, client module present.

### 3. Its tests run standalone, not in make check

`python3 tools/test_stepwise.py` needs no network and no key
and runs in under a second, but it is not wired into `make
check` — same as `slm-bench.sh`, which no gate step executes
either. Wiring tooling tests into the Rust gate is a separate
decision; the PR body reports the run instead.

### 4. Number 0038, not 0036

0036 (renderer empty blocks) and 0037 (fork TDD cycle) exist on
sibling branches off the same base. This branch takes the next
free number so the lines rejoin without a collision.

Secrets: the runner takes `OPENROUTER_API_KEY` from the
environment only. It logs token usage, never the key, and the
key never appears in code, logs, or the state file.

## Consequences

Bench runs are reproducible from the repo at a pinned harness
version — the split version, verified by the same 13 CTQ checks
plus the 2 structure checks, and by the no-key smoke run that
exits 1 before any network use. Future harness changes go
through the same PR process as el changes.

Revisit if: the runner gains a second tool, or the handoff
contract changes — then the handoff doc and the CTQs update
together, in the same PR.
