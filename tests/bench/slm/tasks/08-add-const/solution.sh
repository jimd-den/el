#!/usr/bin/env bash
set -eu
"$EL_BIN" add const LIMIT --in src/util.rs --vis pub --sig 'const LIMIT: u32 = 10' >/dev/null
