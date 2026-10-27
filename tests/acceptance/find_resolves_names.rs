// `el find` resolves a name to an id without the whole-crate dump: a bare
// name, a module path, or a full alias all name the element, the output
// carries outline lines only (no bodies), and an unknown name is a named
// refusal, exit 6 — not an empty success.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/find-resolves";
const BIN: &str = env!("CARGO_BIN_EXE_el");

fn fresh_root(test: &str) -> PathBuf {
    let root = Path::new(ROOT).join(test);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("create root");
    root
}

fn el(cwd: &Path, args: &[&str]) -> Output {
    Command::new(BIN).current_dir(cwd).args(args).output().expect("run el")
}

fn el_stdin(cwd: &Path, args: &[&str], body: &str) -> Output {
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

fn build_shop(root: &Path) -> PathBuf {
    let out = el(root, &["new", "shop"]);
    ok(&out, "new");
    let shop = root.join("shop");
    let out = el(&shop, &["add", "mod", "catalog"]);
    ok(&out, "add mod");
    let out = el_stdin(
        &shop,
        &["add", "fn", "list_items", "--in", "catalog", "--vis", "pub",
          "--sig", "fn list_items() -> Vec<&'static str>", "--body", "-"],
        "vec![\"a\", \"b\"]",
    );
    ok(&out, "add list_items");
    let out = el_stdin(
        &shop,
        &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
        "let items = catalog::list_items();\nfor item in &items {\n    println!(\"{item}\");\n}\n",
    );
    ok(&out, "add main");
    shop
}

#[test]
fn find_names_an_element_three_ways() {
    let root = fresh_root("three-ways");
    let shop = build_shop(&root);

    for query in ["list_items", "catalog::list_items", "catalog::fn/list_items"] {
        let out = el(&shop, &["find", query]);
        ok(&out, &format!("find {query}"));
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(text.contains("catalog::fn/list_items"), "find {query}:\n{text}");
        assert!(!text.contains("vec!"), "find must not print bodies:\n{text}");
    }
}

#[test]
fn find_refuses_an_unknown_name() {
    let root = fresh_root("unknown");
    let shop = build_shop(&root);

    let out = el(&shop, &["find", "nope"]);

    assert_eq!(out.status.code(), Some(6), "exit must be 6 (not found)");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("nope"), "refusal names the query:\n{text}");
}
