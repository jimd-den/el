#!/usr/bin/env bash
set -eu
el() { "$EL_BIN" "$@"; }
"$EL_BIN" find answer | grep -q 'util::fn/answer'
cargo check -q
