#!/usr/bin/env bash
set -eu
id=$("$EL_BIN" find main | awk '/fn\/main/ {print $1; exit}')
"$EL_BIN" show "$id" --statements | grep -c 'println!' | grep -q '^2$'
out=$(cargo run -q)
[ "$(printf '%s\n' "$out" | grep -c '^42$')" -eq 2 ]
