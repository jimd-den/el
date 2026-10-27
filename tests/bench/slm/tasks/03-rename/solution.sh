#!/usr/bin/env bash
set -eu
id=$("$EL_BIN" find unused | awk 'NR==1 {print $1}')
"$EL_BIN" edit "$id" --rename spare >/dev/null
