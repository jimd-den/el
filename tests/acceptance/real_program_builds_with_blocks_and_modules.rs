// A real program, built only with el verbs: two modules, multi-line blocks,
// a cross-module call. The three breaks this names: a block with children was
// refused as unbalanced, a bare module name became a phantom tree node no
// file renders, and `add mod` with a bare name collided with itself.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/real-program";
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
    Command::new(BIN).current_dir(cwd).args(args).output().unwrap_or_else(|error| {
        panic!("run el {args:?}: {error}")
    })
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

#[test]
fn a_real_program_builds_with_blocks_and_modules() {
    let root = fresh_root();
    let shop = root.join("shop");

    ok(&el(&root, &["new", "shop"]), "el new");
    ok(&el(&shop, &["add", "mod", "catalog"]), "el add mod catalog");
    ok(
        &el_with_stdin(
            &shop,
            &[
                "add", "fn", "list_items", "--in", "catalog", "--vis", "pub", "--sig",
                "fn list_items() -> Vec<&'static str>", "--body", "-",
            ],
            "vec![\"a\", \"b\"]",
        ),
        "el add fn list_items",
    );
    ok(
        &el_with_stdin(
            &shop,
            &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
            "let items = catalog::list_items();\nfor item in &items {\n    println!(\"{item}\");\n}\nif !items.is_empty() {\n    println!(\"have items\");\n}\n",
        ),
        "el add fn main",
    );

    let tree = el(&shop, &["tree"]);
    ok(&tree, "el tree");
    let tree_text = String::from_utf8_lossy(&tree.stdout).to_string();
    assert!(!tree_text.contains("mod -"), "no phantom module node:\n{tree_text}");
    assert!(tree_text.contains("src/catalog.rs"), "tree:\n{tree_text}");

    let catalog = fs::read_to_string(shop.join("src/catalog.rs")).expect("catalog file");
    assert!(catalog.contains("fn list_items"), "catalog:\n{catalog}");

    let run = el(&shop, &["run"]);
    ok(&run, "el run");
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(stdout.contains('a'), "stdout: {stdout}");
    assert!(stdout.contains("have items"), "stdout: {stdout}");
}
