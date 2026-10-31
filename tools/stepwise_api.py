#!/usr/bin/env python3
"""OpenRouter client for the stepwise agent: one request, retried.

Sends a chat-completions request and survives the failure modes the
free and cheap tiers actually produce: 429/5xx with an optional
Retry-After header, and transport errors (disconnects, timeouts).
Waits grow exponentially with jitter and are capped, so a retry storm
never exceeds MAX_DELAY between attempts; after MAX_RETRIES the error
propagates and the caller decides.

Deliberately not here: the step loop, prompt building, and state
persistence — those live in stepwise-agent.py, which imports this
module. Nothing here knows about el, tasks, or steps.
"""
import http.client
import json
import random
import sys
import time
import urllib.error
import urllib.request

API_URL = "https://openrouter.ai/api/v1/chat/completions"
MAX_RETRIES = 30
BASE_DELAY = 10.0
MAX_DELAY = 300.0


def log(event):
    sys.stdout.write(json.dumps(event) + "\n")
    sys.stdout.flush()


def backoff(attempt, retry_after):
    if retry_after is not None:
        return retry_after
    delay = BASE_DELAY * (2 ** attempt)
    delay *= 0.75 + random.random() * 0.5
    return min(delay, MAX_DELAY)


def call_api(key, model, messages, tools):
    body = json.dumps({
        "model": model,
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto",
    }).encode()
    attempt = 0
    while True:
        req = urllib.request.Request(API_URL, data=body, headers={
            "Authorization": "Bearer " + key,
            "Content-Type": "application/json",
        })
        try:
            with urllib.request.urlopen(req, timeout=180) as resp:
                return json.loads(resp.read().decode())
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503) and attempt < MAX_RETRIES:
                retry_after = None
                raw = e.headers.get("Retry-After") if e.headers else None
                if raw is not None:
                    try:
                        retry_after = float(raw)
                    except ValueError:
                        retry_after = None
                wait = backoff(attempt, retry_after)
                log({"type": "auto_retry_start", "attempt": attempt + 1,
                     "status": e.code, "wait": wait})
                time.sleep(wait)
                attempt += 1
                continue
            raise
        except (urllib.error.URLError, http.client.HTTPException,
                TimeoutError, ConnectionError, OSError):
            if attempt < MAX_RETRIES:
                wait = backoff(attempt, None)
                log({"type": "auto_retry_start", "attempt": attempt + 1,
                     "status": "network", "wait": wait})
                time.sleep(wait)
                attempt += 1
                continue
            raise
