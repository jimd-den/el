// `el check` is the reporting verb: it runs the compiler through the same parser
// the gate uses, attributes what a catalog can, and does not refuse anything. It
// works on a plain Cargo crate too, which is what lets `make check` dogfood it on
// this repository.

use std::fs;
use std::path::Path;

use crate::program::{el, fresh, ok};

fn text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn plain_crate(name: &str) -> std::path::PathBuf {
    let root = fresh(name);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"plain\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("main");
    root
}

#[test]
fn check_works_on_a_plain_cargo_crate() {
    let root = plain_crate("check-plain");
    assert!(!root.join(".el").exists());

    let output = el(&root, &["check"]);

    ok(&output, "check");
    assert!(text(&output).contains("check ok"), "{}", text(&output));
}

#[test]
fn check_refuses_a_directory_with_no_manifest() {
    let root = fresh("check-empty");

    let output = el(&root, &["check"]);

    assert_eq!(output.status.code(), Some(4), "{}", text(&output));
    let shown = text(&output);
    assert!(shown.contains("Cargo.toml"), "{shown}");
    assert!(shown.contains("next: el new hello"), "{shown}");
}

#[test]
fn check_reports_errors_and_exits_three() {
    let root = plain_crate("check-errors");
    fs::write(root.join("src/main.rs"), "fn main() { let x: u32 = \"no\"; }\n")
        .expect("broken main");

    let output = el(&root, &["check", "--json"]);

    assert_eq!(output.status.code(), Some(3), "{}", text(&output));
    let text = String::from_utf8_lossy(&output.stdout);
    let json = el::core::text::json::parse(text.trim()).expect("one object");
    let gate = json.get("gate").expect("gate");
    assert!(gate.get("diagnostics").and_then(|v| v.as_array()).is_some(), "{text}");
}

#[test]
fn the_policy_can_skip_the_compiler() {
    let root = plain_crate("check-policy");
    fs::write(root.join("el.policy"), "gate = model\n").expect("policy");
    fs::write(root.join("src/main.rs"), "fn main() { let x: u32 = \"no\"; }\n")
        .expect("broken main");

    let output = el(&root, &["check"]);

    ok(&output, "check under gate=model");
    assert!(text(&output).contains("skipped"), "{}", text(&output));
}

#[test]
fn check_never_writes() {
    let root = plain_crate("check-read-only");

    let before = fs::read_to_string(root.join("src/main.rs")).expect("source");
    ok(&el(&root, &["check"]), "check");

    assert_eq!(fs::read_to_string(root.join("src/main.rs")).expect("source"), before);
    assert!(!Path::new(&root.join(".el")).exists());
}
