#!/usr/bin/env bash
set -eu
"$EL_BIN" tree | grep -q 'dead::fn/unused'
cargo check -q
