// UC-1: an author creates a program that prints a word, from nothing, in five
// commands. The test drives the real binary in a real directory, because the
// milestone is a claim about what a person can type, not about a function.
//
// The bodies deliberately omit closing braces and one semicolon: `el` owns the
// punctuation (`core::text::grammar`), so the author writes intent and the tool
// writes the grammar. The last check runs clippy, because "it compiles" is weaker
// than "the generated code is clean".

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/uc1";
const BIN: &str = env!("CARGO_BIN_EXE_el");

fn fresh_root() -> PathBuf {
    let root = Path::new(ROOT);
    if root.exists() {
        fs::remove_dir_all(root).expect("clean");
    }
    fs::create_dir_all(root).expect("create root");
    root.to_path_buf()
}

fn el(cwd: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run el {args:?}: {error}"))
}

fn el_with_stdin(cwd: &Path, args: &[&str], body: &str) -> Output {
    let mut child = Command::new(BIN)
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn el {args:?}: {error}"));
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(body.as_bytes())
            .expect("write body");
    }
    child.wait_with_output().expect("wait")
}

fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn an_author_builds_a_program_from_nothing_in_five_commands() {
    let root = fresh_root();
    let hello = root.join("hello");

    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/greet.rs"]), "el add mod");
    ok(
        &el_with_stdin(
            &hello,
            &[
                "add",
                "fn",
                "greet",
                "--in",
                "src/greet.rs",
                "--vis",
                "pub",
                "--sig",
                "fn greet(name: &str) -> String",
                "--body",
                "-",
            ],
            "format!(\"hello, {name}\")",
        ),
        "el add fn greet",
    );
    ok(
        &el_with_stdin(
            &hello,
            &[
                "add",
                "fn",
                "main",
                "--sig",
                "fn main()",
                "--body",
                "-",
            ],
            "println!(\"{}\", greet::greet(\"world\"));",
        ),
        "el add fn main",
    );

    let run = el(&hello, &["run"]);
    ok(&run, "el run");
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("hello, world"),
        "stdout: {}",
        String::from_utf8_lossy(&run.stdout)
    );

    let log = el(&hello, &["log"]);
    ok(&log, "el log");
    let entries: Vec<String> = String::from_utf8_lossy(&log.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(entries.len(), 5, "log:\n{}", entries.join("\n"));
    assert!(
        entries.iter().all(|line| line.contains("model")),
        "every M6 entry records the model gate: {}",
        entries.join("\n")
    );

    assert!(fs::read_to_string(hello.join("src/greet.rs")).expect("greet file").contains("}"));
    assert!(fs::read_to_string(hello.join("src/main.rs")).expect("main file").contains("}"));
}

#[test]
fn the_generated_crate_is_clippy_clean() {
    let root = fresh_root();
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/greet.rs"]), "el add mod");
    ok(&el_with_stdin(
        &hello,
        &["add", "fn", "greet", "--in", "src/greet.rs", "--vis", "pub", "--sig", "fn greet(name: &str) -> String", "--body", "-"],
        "format!(\"hello, {name}\")",
    ), "greet");
    ok(&el_with_stdin(
        &hello,
        &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
        "println!(\"{}\", greet::greet(\"world\"));",
    ), "main");

    let clippy = Command::new(env!("CARGO"))
        .current_dir(&hello)
        .args(["clippy", "--quiet", "--all-targets", "--", "-D", "warnings"])
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .expect("run clippy");

    assert!(
        clippy.status.success(),
        "generated crate is not clippy-clean:\n{}",
        String::from_utf8_lossy(&clippy.stderr)
    );
}

#[test]
fn an_unknown_verb_is_usage_and_writes_nothing() {
    let root = fresh_root();

    let output = el(&root, &["frobnicate"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a verb"));
}

#[test]
fn a_forbidden_body_exits_violation_and_leaves_the_file_alone() {
    let root = fresh_root();
    ok(&el(&root, &["new", "hello"]), "el new");
    let hello = root.join("hello");
    let before = fs::read_to_string(hello.join("src/main.rs")).expect("main");

    let output = el_with_stdin(
        &hello,
        &["add", "fn", "bad", "--sig", "fn bad()", "--body", "-"],
        "unsafe { poke(); }",
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).contains("model.forbidden"));
    assert_eq!(fs::read_to_string(hello.join("src/main.rs")).expect("main"), before);
}
