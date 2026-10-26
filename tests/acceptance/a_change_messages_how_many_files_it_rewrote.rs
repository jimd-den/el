// A change says how many files it wrote, so an agent can see the footprint
// of an edit without reading anything back: one statement edited in one
// function rewrites one file, not the program. A quiet run stays quiet:
// program output only, no cargo build noise.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/rewrite-message";
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
    Command::new(BIN).current_dir(cwd).args(args).output().expect("run el")
}

fn el_with_stdin(cwd: &Path, args: &[&str], body: &str) -> Output {
    let mut child = Command::new(BIN)
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn el");
    {
        use std::io::Write;
        child.stdin.as_mut().expect("stdin").write_all(body.as_bytes()).expect("write body");
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

fn all_text(output: &Output) -> String {
    format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
}

fn shop() -> PathBuf {
    let root = fresh_root();
    let shop = root.join("shop");
    ok(&el(&root, &["new", "shop"]), "el new");
    ok(
        &el_with_stdin(
            &shop,
            &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
            "println!(\"hello\");\n",
        ),
        "el add fn main",
    );
    shop
}

#[test]
fn an_edit_messages_one_file_rewritten() {
    let shop = shop();
    let tree = el(&shop, &["tree", "--statements"]);
    ok(&tree, "el tree");
    let stmt_id = String::from_utf8_lossy(&tree.stdout)
        .lines()
        .find(|line| line.contains("println!"))
        .expect("println in tree")
        .split_whitespace()
        .next()
        .expect("stmt id")
        .to_string();

    let edit = el_with_stdin(&shop, &["edit", &stmt_id, "--body", "-"], "println!(\"bye\");");
    ok(&edit, "el edit");
    assert!(
        all_text(&edit).contains("el: rewrote 1 file"),
        "edit output:\n{}",
        all_text(&edit)
    );
}

#[test]
fn a_quiet_run_prints_program_output_only() {
    let shop = shop();
    let run = el(&shop, &["run", "--quiet"]);
    ok(&run, "el run --quiet");
    let text = all_text(&run);
    assert!(text.contains("hello"), "run output:\n{text}");
    assert!(!text.contains("Compiling"), "run output:\n{text}");
    assert!(!text.contains("Finished"), "run output:\n{text}");
    assert!(!text.contains("Running"), "run output:\n{text}");
}
