// `el fmt --check` and `el lint` are evidence verbs: they run the
// tool, report the verdict, and never write or fix anything.

use std::fs;

use crate::program::{el, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn plain_crate(name: &str, main: &str) -> std::path::PathBuf {
    let root = fresh(name);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"plain\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/main.rs"), main).expect("main");
    root
}

#[test]
fn fmt_check_passes_a_clean_tree_and_fails_a_dirty_one() {
    let clean = plain_crate("fmt-clean", "fn main() {}\n");
    let output = el(&clean, &["fmt", "--check", "--json"]);
    ok(&output, "fmt --check clean");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("fmt").and_then(|v| v.as_str()), Some("passed"));
    assert_eq!(data.get("test").and_then(|v| v.as_str()), Some("not_run"));
    let evidence = json.get("evidence").unwrap_or_else(|| panic!("no evidence: {json:?}"));
    assert_eq!(evidence.get("kind").and_then(|v| v.as_str()), Some("fmt"));

    let dirty = plain_crate("fmt-dirty", "fn main() { let x=1; }\n");
    let output = el(&dirty, &["fmt", "--check", "--json"]);
    assert_ne!(output.status.code(), Some(0));
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("fmt").and_then(|v| v.as_str()), Some("failed"));
}

#[test]
fn lint_passes_with_a_warnings_count_and_never_fixes() {
    let root = plain_crate("lint-warnings", "fn main() {\n    let unused = 1;\n}\n");
    let output = el(&root, &["lint", "--json"]);
    ok(&output, "lint with warnings still exits 0");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("lint").and_then(|v| v.as_str()), Some("passed"));
    assert!(data.get("warnings").and_then(|v| v.as_number()).unwrap_or(0) >= 1);
    let evidence = json.get("evidence").unwrap_or_else(|| panic!("no evidence: {json:?}"));
    assert_eq!(evidence.get("kind").and_then(|v| v.as_str()), Some("lint"));
    // No fix happened: the source is byte-identical.
    assert_eq!(fs::read_to_string(root.join("src/main.rs")).unwrap(), "fn main() {\n    let unused = 1;\n}\n");
}
