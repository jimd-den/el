#!/usr/bin/env bash
set -eu
snippets=$(mktemp -d)
printf '1\n' > "$snippets/one"
printf '2\n' > "$snippets/two"
printf '3\n' > "$snippets/three"
"$EL_BIN" apply - <<EOF >/dev/null
add mod src/extra.rs
add fn one --in src/extra.rs --vis pub --sig 'fn one() -> u32' --body $snippets/one
add fn two --in src/extra.rs --vis pub --sig 'fn two() -> u32' --body $snippets/two
add fn three --in src/extra.rs --vis pub --sig 'fn three() -> u32' --body $snippets/three
add const LIMIT --in src/extra.rs --vis pub --sig 'const LIMIT: u32 = 5'
EOF
rm -rf "$snippets"
