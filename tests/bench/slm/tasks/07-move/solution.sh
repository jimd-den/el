#!/usr/bin/env bash
set -eu
"$EL_BIN" add mod src/dead.rs >/dev/null
id=$("$EL_BIN" find unused | awk 'NR==1 {print $1}')
"$EL_BIN" mv "$id" src/dead.rs >/dev/null
