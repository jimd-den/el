// A small crate built through the real binary, shared by the read-verb tests.
// It is deliberately a builder, not a fixture: each test names the program it
// needs, and the ids are the parser's output, not a number guessed by the test.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

pub const BIN: &str = env!("CARGO_BIN_EXE_el");
pub const ROOT: &str = "target/acceptance/read-verbs";

pub struct Built {
    pub hello: PathBuf,
    pub greet: String,
    pub main: String,
}

pub fn fresh(name: &str) -> PathBuf {
    let root = Path::new(ROOT).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("create");
    root
}

pub fn el(cwd: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run el {args:?}: {error}"))
}

pub fn el_stdin(cwd: &Path, args: &[&str], body: &str) -> Output {
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
            .expect("write");
    }
    child.wait_with_output().expect("wait")
}

pub fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn build(name: &str) -> Built {
    let root = fresh(name);
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/greet.rs"]), "add mod");
    let greet = el_stdin(
        &hello,
        &[
            "add", "fn", "greet", "--in", "src/greet.rs", "--vis", "pub",
            "--sig", "fn greet(name: &str) -> String", "--body", "-",
        ],
        "/// Greets by name.\nformat!(\"hello, {name}\")",
    );
    ok(&greet, "add greet");
    let main = el_stdin(
        &hello,
        &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
        "greet::greet(\"world\");",
    );
    ok(&main, "add main");
    Built {
        hello,
        greet: String::from_utf8_lossy(&greet.stdout).trim().to_string(),
        main: String::from_utf8_lossy(&main.stdout).trim().to_string(),
    }
}

/// Every element id in the catalog. The catalog is the source of truth and is not
/// a `.rs` file, so a test may read it to know what the tree must contain.
pub fn element_ids(hello: &Path) -> Vec<String> {
    let text = fs::read_to_string(hello.join(".el/catalog")).expect("catalog");
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(tag, _)| *tag == "E")
        .filter_map(|(_, rest)| rest.split_once('\t').map(|(id, _)| id.to_string()))
        .collect()
}
