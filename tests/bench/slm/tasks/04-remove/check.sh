#!/usr/bin/env bash
set -eu
el() { "$EL_BIN" "$@"; }
if "$EL_BIN" find unused >/dev/null 2>&1; then exit 1; fi
cargo check -q
