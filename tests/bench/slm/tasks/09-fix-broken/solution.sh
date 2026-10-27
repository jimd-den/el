#!/usr/bin/env bash
set -eu
"$EL_BIN" check >/dev/null 2>&1 || true
id=$("$EL_BIN" find broken | awk 'NR==1 {print $1}')
printf '42\n' | "$EL_BIN" edit "$id" --body - >/dev/null
