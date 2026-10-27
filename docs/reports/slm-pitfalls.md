# SLM coding pitfalls: what the research says, and where el guards

Surveyed 2026-10-27 for the agent-surface work (ADR-0029). The through-line
is the design note's: small-model coding failures are mostly interface,
selection, and anchoring failures, not reasoning failures. Each pitfall
below names el's existing guard, the gap it left, and the disposition.

## 1. Format collapse and tool-as-text

The model never makes a real call: it prints a call-shaped blob as prose,
or fabricates a tool response before calling anything. RETO (Qwen2.5-7B,
ReAct) shows fluent but ungrounded outputs cascading into unrecoverable
states, fixed by restricting each step to a small tool subset. A Rust
coding-agent test of 7 local models found the same two patterns live:
qwen2.5:7b read the file then asked permission instead of editing, and
qwen2.5-coder:14b printed JSON as text with zero actual calls.
ToolFailBench names the class `tool_skip` (no call, text-only call, or
refusal) beside `result_ignore` (the return is disregarded) and
`output_fabrication` (invented structured data in place of the return).

- Sources: https://arxiv.org/pdf/2602.18968 ,
  https://dev.to/kuroko1t/what-happens-when-local-llms-fail-at-tool-calling-testing-7-models-with-a-rust-coding-agent-cep ,
  https://pub-fad10fba280c45dba0823fd585acc08c.r2.dev/pdf/2607.04686.pdf?v=1789264653
- el's guard: a closed CLI grammar with named usage errors, JSON batch
  and `apply` script forms that parse by construction, and refusal-before-
  write so a malformed attempt changes nothing.
- Gap: none at el's layer — the call channel itself belongs to the
  harness. What el owes the harness is outputs a small model can actually
  use, which is pitfalls 2, 4, and 5.

## 2. Selection collapse as the catalog grows

Accuracy falls as the offered toolset grows, and falls off a cliff past
~15–25 tools: one BFCL-derived stress test shows 79–100% degradation
going from ~50 to ~741 tools, with severe lost-in-the-middle position
bias; TSCG reports 4B–14B tool-calling accuracy at 0–49% past 15 tools
under JSON schemas. The prescribed remedy is scoping the toolset per turn
and grouping tools by intent.

- Sources: https://vllm-sr.ai/blog/semantic-tool-selection/ ,
  https://arxiv.org/html/2605.04107 ,
  https://www.channel.tel/blog/function-calling-accuracy-tool-count-decay-curve
- el's guard: `el skill`, one page capped at 150 lines, and `el find`
  for scoped discovery instead of whole-crate dumps.
- Gap, built in ADR-0029: the page listed 24 verbs flat — exactly the
  shape the literature blames. el keeps no conversation state, so it
  cannot scope per turn; the static form is grouping by task (write,
  read, history, verify, meta), so selection is by intent.

## 3. Repair-loop anchoring

Showing a small model its own failed attempt makes it *less* likely to
succeed than asking again fresh: below 7B, blind resampling beats
feedback-informed repair, and retries re-emit a near-identical program
in 33–68% of cases (2–14% under blind resampling); execution feedback
adds nothing measurable over a content-free failure notice. Iteration
also converges (95.4% similarity by the second iteration elsewhere) and
breaks previously-passing checks, with damage tracking how much code was
rewritten.

- Sources: https://paperz.arrangedbits.com/api/papers/44e6eff1-2320-42f9-aabf-d92b429bbda8/pdf ,
  https://andreasbauer.org/esem26/p050-Agyekum.pdf
- el's guard: the loop is built for discard-and-reroll — `--dry-run`
  previews with the verdict exit, `undo` restores bytes exactly, and the
  gate refuses any change that adds an error, so a repair cannot silently
  regress the tree.
- Gap, deferred: varying the hint on a *repeated identical* refusal
  would need refusals recorded across processes, but plan refusals write
  nothing by design (only gate rejections journal). That split in the
  journal's meaning needs its own decision, not a ride-along.

## 4. The feedback-quality bottleneck

Olausson et al. find same-model self-repair gains modest because repair
is bottlenecked by the model's ability to give feedback on its own code;
a stronger critic helps far more, and human feedback more still. The
consequence for a tool: do not ask the model to be its own critic. Be
the critic, mechanically.

- Source: https://ar5iv.labs.arxiv.org/html/2306.09896
- el's guard: the compiler is the critic — attributed diagnostics,
  exactly one `first_actionable`, and every refusal naming a next command.
- Gap, built in ADR-0029: the critic's directions assumed the agent
  addresses by id, but a small model remembers names. `show greet`
  refused with no way forward beyond re-listing the crate. Now a
  single-token subject that is not an id gets the exact resolution
  (`el find <name>`) in the refusal itself, through the one funnel
  (`change::parse_id`) every id-taking verb shares, plus the `apply`
  script's id check.

## 5. Compiles but wrong

Under flawed task descriptions, 60–90% of code that compiles is
semantically incorrect; scale does not fix it. The same gap appears as
"plausible code": it looks right, runs on the happy path, and hides the
error. No compiler gate closes this one — it is an intent gap, not a
syntax gap.

- Source: https://arxiv.org/pdf/2507.20439
- el's guard: honest scope. The gate guarantees no *new* compiler
  errors per commit, byte-exact restore, and element-level diffs so a
  reviewer (human or stronger model) sees exactly what changed. It does
  not claim the change is right.
- Gap, deferred deliberately: no tier runs tests, and adding a
  test-running tier is the safety-line decision, not an agent-surface
  change. It is not smuggled in here.

## 6. Long-context degradation

Function-calling accuracy drops steeply as conversations grow toward
80K tokens, and tool definitions themselves (100–300 tokens each) eat
the budget before the task starts.

- Source: https://proxy.filestage.io/_url/https://arxiv.org/pdf/2505.10570
- el's guard: outline-only discovery (`find`, `tree` without bodies),
  `show --terse`, compact JSON, and the 15-line function budget, which
  caps how much any one body can flood a context.
- Gap: none found at el's layer; the remaining lever is the harness
  keeping sessions short, which el cannot do for it.
