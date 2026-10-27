#!/usr/bin/env bash
set -eu
id=$("$EL_BIN" find double | awk 'NR==1 {print $1}')
printf 'let _ = x;\n' | "$EL_BIN" add stmt "$id" --at 0 --body - >/dev/null
