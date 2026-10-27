#!/usr/bin/env bash
set -eu
id=$("$EL_BIN" find main | awk '/fn\/main/ {print $1; exit}')
printf 'println!("{n}");\n' | "$EL_BIN" add stmt "$id" --at 1 --body - >/dev/null
