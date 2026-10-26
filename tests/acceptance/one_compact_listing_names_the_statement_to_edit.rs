// An agent edits a statement it found from one compact listing, without
// reading a source file and without the whole-program dump. The use case is
// the edit loop: `tree` finds the function, one `show --statements --terse`
// names the statement and shows its body, `edit` changes only that statement.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/compact-show";
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

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn one_compact_listing_names_the_statement_to_edit() {
    let root = fresh_root();
    let shop = root.join("shop");

    ok(&el(&root, &["new", "shop"]), "el new");
    ok(
        &el_with_stdin(
            &shop,
            &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
            "let items = vec![\"a\"];\nfor item in &items {\n    println!(\"{item}\");\n}\n",
        ),
        "el add fn main",
    );

    let tree = el(&shop, &["tree"]);
    ok(&tree, "el tree");
    let main_id = text(&tree)
        .lines()
        .find(|line| line.contains("fn  main::fn/main"))
        .expect("main in tree")
        .split_whitespace()
        .next()
        .expect("main id")
        .to_string();

    let listing = el(&shop, &["show", &main_id, "--statements", "--terse"]);
    ok(&listing, "compact listing");
    let listing_text = text(&listing);
    let listing_lines: Vec<&str> = listing_text.lines().collect();
    assert_eq!(listing_lines.len(), 4, "header plus three statements:\n{listing_text}");
    assert!(listing_lines[0].starts_with(&format!("{main_id} fn ")), "{listing_text}");
    let println_line = listing_lines
        .iter()
        .find(|line| line.contains("println!"))
        .expect("println in listing");
    let stmt_id = println_line.split_whitespace().next().expect("stmt id").to_string();

    let dump = el(&shop, &["tree", "--statements"]);
    ok(&dump, "whole-program dump");
    assert!(
        listing_text.len() < text(&dump).len(),
        "compact listing is cheaper than the dump:\n{listing_text}"
    );

    ok(
        &el_with_stdin(&shop, &["edit", &stmt_id, "--body", "-"], "println!(\"got {item}\");"),
        "el edit",
    );
    let run = el(&shop, &["run"]);
    ok(&run, "el run");
    assert!(text(&run).contains("got a"), "run output:\n{}", text(&run));
}
