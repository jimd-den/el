// The benchmark is a deliverable, so its shape is gated: ten tasks, each with a
// prompt, a check and a solution, and a runner that requires a model name for a
// real run. A task that loses one of those is a task that cannot be measured.

use std::fs;
use std::path::Path;

fn bench() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/bench/slm")
}

#[test]
fn there_are_exactly_ten_tasks_with_unique_ids() {
    let text = fs::read_to_string(bench().join("tasks.tsv")).expect("tasks.tsv");
    let ids: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split('\t').next().expect("id"))
        .collect();

    assert_eq!(ids.len(), 10);
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), ids.len());
}

#[test]
fn every_task_has_a_prompt_a_check_and_a_solution() {
    let text = fs::read_to_string(bench().join("tasks.tsv")).expect("tasks.tsv");
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let id = line.split('\t').next().expect("id");
        let dir = bench().join("tasks").join(id);
        let prompt = fs::read_to_string(dir.join("prompt.md"))
            .unwrap_or_else(|_| panic!("{id}: prompt.md"));
        assert!(!prompt.trim().is_empty(), "{id}: empty prompt");
        for script in ["check.sh", "solution.sh"] {
            let body = fs::read_to_string(dir.join(script))
                .unwrap_or_else(|_| panic!("{id}: {script}"));
            assert!(body.contains("EL_BIN"), "{id}: {script} must use the binary under test");
        }
    }
}

#[test]
fn the_runner_and_the_base_crate_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let runner = root.join("tools/slm-bench.sh");
    assert!(runner.is_file(), "tools/slm-bench.sh");
    let mode = fs::metadata(&runner).expect("runner").permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert!(mode.mode() & 0o111 != 0, "runner is not executable");
    }
    let base = fs::read_to_string(bench().join("base.sh")).expect("base.sh");
    assert!(base.contains("el new"), "base.sh must build the fixture with el");
}
