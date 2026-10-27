#!/usr/bin/env bash
# Build the task crate with el, so every task starts from modelled elements
# rather than opaque files. The runner has already created and entered the
# work directory; EL_BIN is absolute.
set -eu
el() { "$EL_BIN" "$@"; }
snippets=$(mktemp -d)
printf 'x * 2\n' > "$snippets/double"
printf '0\n' > "$snippets/unused"
printf 'let n = util::double(21);\nprintln!("{n}");\n' > "$snippets/main"
el new hello >/dev/null
cd hello
el add mod src/util.rs >/dev/null
el add fn double --in src/util.rs --vis pub --sig 'fn double(x: u32) -> u32' --body "$snippets/double" >/dev/null
el add fn unused --in src/util.rs --vis pub --sig 'fn unused() -> u32' --body "$snippets/unused" >/dev/null
el add fn main --sig 'fn main()' --body "$snippets/main" >/dev/null
rm -rf "$snippets"
