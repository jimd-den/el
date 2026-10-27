#!/usr/bin/env bash
set -eu
el() { "$EL_BIN" "$@"; }
printf '42\n' | "$EL_BIN" add fn answer --in src/util.rs --vis pub --sig 'fn answer() -> u32' --body - >/dev/null
