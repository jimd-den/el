#!/usr/bin/env bash
set -eu
el() { "$EL_BIN" "$@"; }
"$EL_BIN" find spare | grep -q 'util::fn/spare'
if "$EL_BIN" find unused >/dev/null 2>&1; then exit 1; fi
cargo check -q
