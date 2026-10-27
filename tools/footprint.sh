#!/usr/bin/env bash
# The one number the headline promise costs: a stripped release binary. The limit
# is the plan's 1.5 MB; override it only to measure a change, not to pass.
set -eu
limit=${FOOTPRINT_LIMIT:-1500000}
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
cargo build --release --quiet
staged=$(mktemp)
cp target/release/el "$staged"
strip "$staged"
bytes=$(stat -c %s "$staged")
rm -f "$staged"
printf 'footprint: %s bytes (limit %s)\n' "$bytes" "$limit"
if [ "$bytes" -gt "$limit" ]; then
    printf 'footprint: over the limit by %s bytes\n' "$((bytes - limit))" >&2
    exit 1
fi
