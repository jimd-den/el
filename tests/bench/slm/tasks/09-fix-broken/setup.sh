#!/usr/bin/env bash
# A broken function is added with --fast, deliberately: the setup must leave the
# tree broken so the task is to read the diagnostic and repair it.
set -eu
cd hello
printf '"no"\n' | "$EL_BIN" add fn broken --sig 'fn broken() -> u32' --body - --fast >/dev/null
