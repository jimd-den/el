#!/usr/bin/env python3
"""Stepwise SLM agent loop: no transcript re-send, one action per step.

The next step after mini-agent.py. Each API request carries a FRESH
context: system prompt + task + a compact progress summary (the last
WINDOW steps, command -> truncated result). The full transcript is
never re-sent, so per-step tokens stay roughly flat no matter how long
the task runs, instead of growing with every turn.

Why this works for el and would not for a plain editor loop: el IS the
external memory. Every change goes through the catalog, and el tree /
find / show re-read current state cheaply, so the model can re-discover
anything older than the window instead of carrying it in context.

State is persisted to --state-file after every step, so a run is
resumable across processes and runtime restarts: relaunch with the same
state file and it continues from the last completed step.

Rate-limit behaviour is inherited from mini-agent.py: Retry-After
honoured, else exponential backoff with jitter, 30 retries per request,
transport errors retried. One tool (run_shell), one action per step:
only the first tool call in a response is executed; the rest are
ignored, which is what keeps the step discipline honest.

Usage: stepwise-agent.py --system-prompt FILE --task FILE
         --state-file FILE [--model NAME]
  Env: OPENROUTER_API_KEY (required), STEP_MAX_STEPS (default 60),
       STEP_WINDOW (default 5), STEP_CMD_TIMEOUT (default 120),
       STEP_PACING (default 2.0s).
  Exit: 0 when the model replies DONE or the step cap is hit (the bench
  check decides pass/fail); 1 on API exhaustion or fatal error.
  The OpenRouter client (call_api, backoff) lives in stepwise_api.py;
  this file is the step loop and the state handling.
"""
import json
import os
import subprocess
import sys
import time

# backoff and MAX_DELAY are re-exported for test_stepwise.py.
from stepwise_api import MAX_DELAY, backoff, call_api, log

OUT_CAP = 4000
RESULT_CAP = 1200


def run_shell(command, cwd, timeout):
    try:
        proc = subprocess.run(command, shell=True, cwd=cwd, timeout=timeout,
                              capture_output=True, text=True, executable="/bin/sh")
        out = (proc.stdout or "") + (proc.stderr or "")
    except subprocess.TimeoutExpired:
        return "TIMEOUT after %ss" % timeout
    if len(out) > OUT_CAP:
        out = out[:OUT_CAP] + "\n...[output truncated]"
    return out or "(no output)"


def first_command(calls):
    """One action per step: only the first tool call is executed."""
    if not calls:
        return None
    try:
        return json.loads(calls[0]["function"]["arguments"])["command"]
    except (KeyError, ValueError):
        return ""


def step_prompt(task, steps, window):
    """Compact progress summary: only the last `window` steps verbatim."""
    lines = ["TASK:", task, "", "PROGRESS: %d step(s) taken." % len(steps)]
    recent = steps[-window:] if window > 0 else []
    if len(steps) > len(recent):
        lines.append("(%d earlier step(s) omitted; "
                     "re-read state with el tree/find/show.)"
                     % (len(steps) - len(recent)))
    for i, s in enumerate(recent, start=len(steps) - len(recent) + 1):
        result = s["result"][:RESULT_CAP]
        lines.append("Step %d: %s -> %s" % (i, s["command"], result))
    lines.append("")
    lines.append("NEXT: reply with exactly ONE run_shell call for the next "
                 "step, or reply DONE if the task is finished.")
    return "\n".join(lines)


def load_state(path):
    if os.path.exists(path):
        with open(path) as f:
            return json.load(f)
    return {"steps": []}


def save_state(path, state):
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(state, f)
    os.replace(tmp, path)


def main():
    args = sys.argv[1:]
    opts = dict(zip(args[0::2], args[1::2]))
    system = open(opts["--system-prompt"]).read()
    task = open(opts["--task"]).read()
    state_path = opts["--state-file"]
    model = opts.get("--model") or os.environ.get(
        "SLM_MODEL", "poolside/laguna-xs-2.1:free")
    key = os.environ.get("OPENROUTER_API_KEY")
    if not key:
        print("stepwise-agent: OPENROUTER_API_KEY is not set", file=sys.stderr)
        return 1
    cwd = os.getcwd()
    max_steps = int(os.environ.get("STEP_MAX_STEPS", "60"))
    window = int(os.environ.get("STEP_WINDOW", "5"))
    cmd_timeout = int(os.environ.get("STEP_CMD_TIMEOUT", "120"))
    pacing = float(os.environ.get("STEP_PACING", "2.0"))

    tools = [{
        "type": "function",
        "function": {
            "name": "run_shell",
            "description": "Run a shell command in the project directory.",
            "parameters": {"type": "object",
                           "properties": {"command": {"type": "string"}},
                           "required": ["command"]},
        },
    }]
    system += ("\n\nYou receive only a brief progress summary each step, "
               "not the full history. Use el tree/find/show to re-read "
               "anything you need. Each reply: exactly one run_shell call, "
               "or DONE.")
    state = load_state(state_path)
    steps = state["steps"]
    for _ in range(max_steps):
        time.sleep(pacing)
        messages = [
            {"role": "system", "content": system},
            {"role": "user",
             "content": step_prompt(task, steps, window)},
        ]
        try:
            data = call_api(key, model, messages, tools)
        except Exception as e:
            print("stepwise-agent: API failed: %s" % e, file=sys.stderr)
            return 1
        msg = data["choices"][0]["message"]
        usage = data.get("usage") or {}
        log({"type": "message_end",
             "message": {"usage": {"totalTokens": usage.get("total_tokens") or 0}}})
        content = (msg.get("content") or "").strip()
        calls = msg.get("tool_calls") or []
        if content == "DONE" and not calls:
            return 0
        if not calls:
            steps.append({"command": "(no action)",
                          "result": content[:RESULT_CAP] or "(empty reply)"})
            save_state(state_path, state)
            continue
        log({"type": "tool_execution_start"})
        cmd = first_command(calls)
        result = run_shell(cmd, cwd, cmd_timeout) if cmd else "empty command"
        steps.append({"command": cmd, "result": result[:RESULT_CAP]})
        save_state(state_path, state)
    return 0  # step cap: let the bench check decide


if __name__ == "__main__":
    sys.exit(main())
