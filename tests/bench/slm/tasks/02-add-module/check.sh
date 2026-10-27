#!/usr/bin/env bash
set -eu
el() { "$EL_BIN" "$@"; }
"$EL_BIN" find shout | grep -q 'text::fn/shout'
cargo check -q
