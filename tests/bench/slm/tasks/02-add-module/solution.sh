#!/usr/bin/env bash
set -eu
el() { "$EL_BIN" "$@"; }
"$EL_BIN" add mod src/text.rs >/dev/null
printf 'format!("{s}!")\n' | "$EL_BIN" add fn shout --in src/text.rs --vis pub --sig 'fn shout(s: &str) -> String' --body - >/dev/null
