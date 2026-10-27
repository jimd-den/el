#!/usr/bin/env bash
set -eu
"$EL_BIN" find LIMIT | grep -q 'util::const/LIMIT'
cargo check -q
