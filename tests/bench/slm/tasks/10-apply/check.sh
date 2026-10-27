#!/usr/bin/env bash
set -eu
"$EL_BIN" tree | grep -q 'extra::fn/one'
"$EL_BIN" tree | grep -q 'extra::fn/two'
"$EL_BIN" tree | grep -q 'extra::fn/three'
"$EL_BIN" tree | grep -q 'extra::const/LIMIT'
"$EL_BIN" log | grep -q 'apply'
cargo check -q
