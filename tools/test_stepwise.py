#!/usr/bin/env python3
"""Tests for the stepwise handoff contract (see stepwise-handoff.md).

No network, no API key: every test exercises pure functions of
stepwise-agent.py (prompt building, state IO, backoff, command
selection). Run: python3 test_stepwise.py
"""
import importlib.util
import os
import sys

spec = importlib.util.spec_from_file_location(
    "sw", os.path.join(os.path.dirname(__file__), "stepwise-agent.py"))
sw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sw)

FAILURES = []


def check(name, cond):
    print(("PASS " if cond else "FAIL ") + name)
    if not cond:
        FAILURES.append(name)


def steps(n):
    return [{"command": "cmd%d" % i, "result": "out%d" % i}
            for i in range(n)]


# CTQ 1: per-step token bound — prompt size stays flat as steps grow.
p10 = sw.step_prompt("task", steps(10), 5)
p100 = sw.step_prompt("task", steps(100), 5)
check("ctq1 prompt flat: 100 steps barely larger than 10",
      0 < len(p100) - len(p10) < 50)
check("ctq1 window keeps only last 5",
      "cmd99" in p100 and "cmd95" in p100 and "cmd94" not in p100)
check("ctq1 omitted count stated",
      "95 earlier" in p100)

# CTQ 2: no lost steps — save/load round-trips exactly.
sw.save_state("/tmp/ctq-state.json", {"steps": steps(7)})
check("ctq2 save/load round-trips",
      sw.load_state("/tmp/ctq-state.json")["steps"] == steps(7))
check("ctq2 atomic write leaves no temp file",
      not os.path.exists("/tmp/ctq-state.json.tmp"))

# CTQ 3: resume fidelity — reloaded state builds the identical prompt.
state = {"steps": steps(12)}
sw.save_state("/tmp/ctq-state.json", state)
check("ctq3 resumed prompt equals in-memory prompt",
      sw.step_prompt("task", sw.load_state("/tmp/ctq-state.json")["steps"], 5)
      == sw.step_prompt("task", state["steps"], 5))

# CTQ 4: one action per step — only the first tool call executes.
calls = [
    {"function": {"arguments": '{"command": "first"}'}},
    {"function": {"arguments": '{"command": "second"}'}},
]
check("ctq4 first call only", sw.first_command(calls) == "first")
check("ctq4 no calls -> None", sw.first_command([]) is None)
check("ctq4 malformed args -> empty",
      sw.first_command([{"function": {"arguments": "not json"}}]) == "")

# CTQ 5: retry survival — waits grow; Retry-After honoured.
waits = [sw.backoff(a, None) for a in range(5)]
check("ctq5 backoff grows", all(b > a for a, b in zip(waits, waits[1:])))
check("ctq5 backoff capped at MAX_DELAY",
      sw.backoff(20, None) <= sw.MAX_DELAY)
check("ctq5 Retry-After honoured", sw.backoff(0, 42.0) == 42.0)

# CTQ 6: no-action discipline — a text-only reply is representable as
# a step, so the loop records it and continues instead of dying.
state = {"steps": []}
state["steps"].append({"command": "(no action)", "result": "chatter"})
sw.save_state("/tmp/ctq-state.json", state)
check("ctq6 no-action step persists",
      sw.load_state("/tmp/ctq-state.json")["steps"][0]["command"]
      == "(no action)")

# Structure: the agent entry point fits el's 200-line file ceiling, with
# the API client split out into stepwise_api.py.
agent_path = os.path.join(os.path.dirname(__file__), "stepwise-agent.py")
check("agent entry fits 200-line ceiling",
      len(open(agent_path).read().splitlines()) <= 200)
try:
    api_spec = importlib.util.spec_from_file_location(
        "sw_api", os.path.join(os.path.dirname(__file__), "stepwise_api.py"))
    sw_api = importlib.util.module_from_spec(api_spec)
    api_spec.loader.exec_module(sw_api)
    check("api client split out", hasattr(sw_api, "call_api")
          and hasattr(sw_api, "backoff"))
except Exception:
    check("api client split out", False)

print()
if FAILURES:
    print("FAILURES:", FAILURES)
    sys.exit(1)
print("all stepwise handoff tests pass")
