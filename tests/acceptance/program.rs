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

/// `el` with an overridden `PATH`, for tests that need a tool to be absent.
pub fn el_env(cwd: &Path, args: &[&str], path: &str) -> Output {
    Command::new(BIN)
        .current_dir(cwd)
        .args(args)
        .env("PATH", path)
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

/// Every owned file's bytes, for a byte-for-byte restore comparison. `.el` is
/// deliberately excluded: it is history, not the projection.
pub fn files(hello: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    walk(&hello.join("src"), "src", &mut out);
    if let Ok(bytes) = fs::read(hello.join("Cargo.toml")) {
        out.insert("Cargo.toml".to_string(), bytes);
    }
    out
}

fn walk(base: &Path, prefix: &str, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = fs::read_dir(base) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let key = format!("{prefix}/{name}");
        if path.is_dir() {
            walk(&path, &key, out);
        } else if let Ok(bytes) = fs::read(&path) {
            out.insert(key, bytes);
        }
    }
}

/// The `data` array from `el log --json`, newest first.
pub fn log_json(hello: &Path, attic: bool) -> Vec<el::core::text::json::Json> {
    let args: &[&str] = if attic { &["log", "--attic", "--json"] } else { &["log", "--json"] };
    let output = el(hello, args);
    ok(&output, "el log --json");
    let line = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(line.trim())
        .expect("our JSON parses")
        .get("data")
        .and_then(|v| v.as_array())
        .expect("log data")
        .to_vec()
}
