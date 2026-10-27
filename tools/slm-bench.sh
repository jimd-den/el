#!/usr/bin/env bash
# The SLM harness: one fixture crate, ten tasks, one command an agent must drive.
# It does not bundle a model. `--solution` runs each task's checked-in solution,
# which validates the harness and the checks without a model; a real run names the
# model and feeds each prompt to $EL_AGENT on stdin.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
bench="$root/tests/bench/slm"
el_bin=${EL_BIN:-$root/target/debug/el}
model=not-named
mode=run
tasks=()

while [ $# -gt 0 ]; do
    case "$1" in
        --list) mode=list ;;
        --solution) mode=solution ;;
        --model) shift; model=${1:?--model needs a name} ;;
        --task) shift; tasks+=("${1:?--task needs an id}") ;;
        -h|--help)
            printf 'usage: %s [--list] [--solution] [--model NAME] [--task ID]...\n' "$0"
            exit 0
            ;;
        *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
    esac
    shift
done

if [ "$mode" = list ]; then
    cut -f1,2 "$bench/tasks.tsv"
    exit 0
fi

if [ "$mode" = run ] && { [ -z "${EL_AGENT:-}" ] || [ "$model" = not-named ]; }; then
    printf 'a real run needs EL_AGENT and --model; use --solution to validate the harness\n' >&2
    exit 2
fi

if [ -z "${EL_BIN:-}" ]; then
    (cd "$root" && cargo build --quiet)
    target_dir=$(cd "$root" && cargo metadata --format-version 1 --no-deps \
        | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
    el_bin=${target_dir:-$root/target}/debug/el
fi
if [ ! -x "$el_bin" ]; then
    printf 'el binary is not executable: %s\n' "$el_bin" >&2
    exit 2
fi
el_bin=$(cd "$(dirname "$el_bin")" && pwd)/$(basename "$el_bin")
for want in "${tasks[@]}"; do
    if ! awk -F'\t' -v id="$want" '$1 == id { found = 1 } END { exit !found }' "$bench/tasks.tsv"; then
        printf 'unknown task: %s\n' "$want" >&2
        exit 2
    fi
done

work="$root/target/slm"
report="$work/report.tsv"
rm -rf "$work"
mkdir -p "$work"
printf 'task\tmodel\tresult\tseconds\tprompt_bytes\n' > "$report"

selected() {
    [ ${#tasks[@]} -eq 0 ] && return 0
    local want
    for want in "${tasks[@]}"; do [ "$want" = "$1" ] && return 0; done
    return 1
}

failed=0
ran=0
while IFS=$'\t' read -r id description; do
    [ -n "$id" ] || continue
    selected "$id" || continue
    case "$id" in \#*) continue ;; esac
    ran=$((ran + 1))
    task_dir="$bench/tasks/$id"
    dir="$work/$id"
    mkdir -p "$dir"
    started=$(date +%s)
    prompt_bytes=$(wc -c < "$task_dir/prompt.md" | tr -d ' ')
    printf '== %s — %s\n' "$id" "$description"
    if ! (cd "$dir" && EL_BIN="$el_bin" SLM_MODEL="$model" bash "$bench/base.sh" >/dev/null 2>&1); then
        printf '  base setup failed\n' >&2
        printf '%s\t%s\tfail-base\t%s\t%s\n' "$id" "$model" "$(( $(date +%s) - started ))" "$prompt_bytes" >> "$report"
        failed=$((failed + 1))
        continue
    fi
    if [ -f "$task_dir/setup.sh" ]; then
        if ! (cd "$dir" && EL_BIN="$el_bin" bash "$task_dir/setup.sh" >/dev/null 2>&1); then
            printf '  task setup failed\n' >&2
            printf '%s\t%s\tfail-setup\t%s\t%s\n' "$id" "$model" "$(( $(date +%s) - started ))" "$prompt_bytes" >> "$report"
            failed=$((failed + 1))
            continue
        fi
    fi
    if [ "$mode" = solution ]; then
        runner="$task_dir/solution.sh"
    else
        runner=""
    fi
    ok=1
    if [ -n "$runner" ]; then
        (cd "$dir/hello" && EL_BIN="$el_bin" SLM_MODEL="$model" bash "$runner" >/dev/null 2>&1) || ok=0
    else
        (cd "$dir/hello" && EL_BIN="$el_bin" SLM_MODEL="$model" bash -c "$EL_AGENT" < "$task_dir/prompt.md" >/dev/null 2>&1) || ok=0
    fi
    if [ "$ok" -eq 1 ] && (cd "$dir/hello" && EL_BIN="$el_bin" SLM_MODEL="$model" bash "$task_dir/check.sh" >/dev/null 2>&1); then
        result=pass
        printf '  pass\n'
    else
        result=fail
        printf '  FAIL\n' >&2
        failed=$((failed + 1))
    fi
    printf '%s\t%s\t%s\t%s\t%s\n' "$id" "$model" "$result" "$(( $(date +%s) - started ))" "$prompt_bytes" >> "$report"
done < "$bench/tasks.tsv"

printf '\nreport: %s\n' "$report"
cat "$report"
if [ "$ran" -eq 0 ]; then
    printf 'no task ran\n' >&2
    exit 2
fi
[ "$failed" -eq 0 ]
