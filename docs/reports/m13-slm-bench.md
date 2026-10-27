# M13 — the footprint and the measured loop

Date: 2026-10-27 · Branch: `feat/m12-m13-agent-surface`

## Footprint

```console
$ tools/footprint.sh
footprint: 1184664 bytes (limit 1500000)
```

A stripped release binary, 1.13 MB against the plan’s 1.5 MB. The script builds
`--release`, copies to a temporary path, strips there, and refuses over the limit;
it never strips the built binary in place.

## The harness

`tests/bench/slm/` holds ten tasks over one fixture crate. Each task has a prompt,
a check and a solution; the runner is `tools/slm-bench.sh`. `--solution` is the
self-test: it runs every checked-in solution and then every check, so the harness
is known to be solvable and the checks are not vacuous.

```console
$ tools/slm-bench.sh --solution
... 10 tasks ...
task	model	result	seconds	prompt_bytes
01-add-answer	not-named	pass	4	...
...
10-apply	not-named	pass	4	...
```

All ten pass. The model named for a real run is `qwen2.5-coder:7b-instruct`; the
runner requires `EL_AGENT` and `--model`, records both, and refuses to call a run
with no model named.

## What is deliberately not claimed

There is no benchmark result here. The harness exists, the tasks are checked in,
and the solutions prove the tasks are solvable through `el`; a model has not been
run against them in this session. The design note’s claim about small models stays
a hypothesis until `target/slm/report.tsv` comes from a real `EL_AGENT` run.

The ten tasks are narrow on purpose: each one exercises one verb or one failure
path. They are not a measure of general coding ability, and ten tasks cannot rank
two models that are close.
