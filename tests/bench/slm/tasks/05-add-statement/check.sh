#!/usr/bin/env bash
set -eu
"$EL_BIN" show "$("$EL_BIN" find double | awk 'NR==1 {print $1}')" --statements | grep -q 'let _ = x;'
cargo check -q
