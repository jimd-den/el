# The SLM benchmark

Ten tasks an agent must do through `el`, one fixture crate, one check per task.
This is the harness the PR’s design note asked for; it is not a result. No model
is bundled and no score is claimed until a model is actually run.

## Run it

```console
$ tools/slm-bench.sh --list
$ tools/slm-bench.sh --solution                 # validate the harness, no model
$ EL_AGENT='your-agent-command' tools/slm-bench.sh \
      --model qwen2.5-coder:7b-instruct
```

`EL_AGENT` is a shell command run inside the crate with the task prompt on stdin.
`EL_BIN` and `SLM_MODEL` are exported, so the agent can call the binary under test.
The model named here is the default small coder the design note discusses; change
`--model` for another. The runner records `target/slm/report.tsv`: task, model,
pass/fail, seconds, prompt bytes. It does not record tokens, because the tokenizer
belongs to the model, not to `el`.

## The tasks

| Id | What it asks |
|---|---|
| 01-add-answer | add a function that returns 42 |
| 02-add-module | add a module with a function |
| 03-rename | rename an element, id preserved |
| 04-remove | remove an element |
| 05-add-statement | add a statement at the top of a body |
| 06-second-print | add a second print statement |
| 07-move | move an element to a new module |
| 08-add-const | add a const, with the type it cannot infer |
| 09-fix-broken | read `el check`, fix the first actionable error |
| 10-apply | five changes in one `el apply` transaction |

Each task starts from `base.sh`, which builds the crate with `el new` and `el add`,
so every element is modelled (not opaque). A task may add `setup.sh`; task 09 uses
it to plant a compiler error with `--fast`. `--solution` runs the checked-in
solution for each task and then the same check a model would face.

## What it measures

Pass@1 on ten narrow tasks, wall time, and prompt bytes. It does **not** measure
tokens, model quality in general, or whether the agent found the cheapest path.
A failure is useful because every task names the command it expects; a comparison
between models is only meaningful at the same `--model` and the same `EL_BIN`.
