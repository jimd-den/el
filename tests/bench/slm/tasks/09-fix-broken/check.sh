#!/usr/bin/env bash
set -eu
"$EL_BIN" check >/dev/null 2>&1
cargo check -q
