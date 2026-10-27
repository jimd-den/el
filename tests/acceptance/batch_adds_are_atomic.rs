// `el batch` is one transaction for many adds: every item is planned
// against the working catalog, the gate runs once on the final tree, and
// one sequence commits — or nothing is applied at all. A gate rejection
// restores the old tree and journals one rejected attempt; a plan failure
// names its item; statements are refused because they need ids that do
// not exist until planning.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/batch-atomic";
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

fn text(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn esc(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn item(kind: &str, name: &str, module: &str, sig: &str, body: &str) -> String {
    format!(
        "{{\"kind\":\"{kind}\",\"name\":\"{name}\",\"in\":\"{module}\",\"vis\":\"pub\",\"sig\":\"{}\",\"body\":\"{}\"}}",
        esc(sig),
        esc(body)
    )
}

#[test]
fn batch_adds_a_whole_module_in_one_transaction() {
    let root = fresh_root("whole-module");
    let out = el(&root, &["new", "shop"]);
    ok(&out, "new");
    let shop = root.join("shop");
    let batch = format!(
        "[{{\"kind\":\"mod\",\"name\":\"catalog\"}}, {}, {}]",
        item(
            "fn",
            "list_items",
            "catalog",
            "fn list_items() -> Vec<&'static str>",
            "vec![\"a\", \"b\"]"
        ),
        item(
            "fn",
            "main",
            "main",
            "fn main()",
            "let items = catalog::list_items();\nfor item in &items {\n    println!(\"{item}\");\n}\n"
        )
    );
    let out = el_stdin(&shop, &["batch"], &batch);
    ok(&out, "batch");
    let out = el(&shop, &["tree"]);
    ok(&out, "tree");
    let tree = text(&out);
    assert!(tree.contains("catalog::fn/list_items"), "tree:\n{tree}");
    assert!(tree.contains("main::fn/main"), "tree:\n{tree}");
    let out = el(&shop, &["run"]);
    ok(&out, "run");
    assert!(text(&out).contains('a'), "run output:\n{}", text(&out));
}

#[test]
fn batch_gate_rejection_applies_nothing() {
    let root = fresh_root("gate-rejection");
    let out = el(&root, &["new", "shop"]);
    ok(&out, "new");
    let shop = root.join("shop");
    let batch = format!(
        "[{{\"kind\":\"mod\",\"name\":\"catalog\"}}, {}, {}]",
        item("fn", "good", "catalog", "fn good() -> u32", "7"),
        item("fn", "broken", "catalog", "fn broken()", "undefined_fn();")
    );
    let out = el_stdin(&shop, &["batch"], &batch);
    assert_eq!(out.status.code(), Some(3), "exit must be 3 (gate):\n{}", text(&out));
    let out = el(&shop, &["tree"]);
    ok(&out, "tree");
    let tree = text(&out);
    assert!(!tree.contains("catalog"), "nothing from the batch applied:\n{tree}");
    let out = el(&shop, &["log"]);
    ok(&out, "log");
    assert!(text(&out).contains("rejected:1"), "log:\n{}", text(&out));
}

#[test]
fn batch_plan_failure_names_its_item_and_applies_nothing() {
    let root = fresh_root("plan-failure");
    let out = el(&root, &["new", "shop"]);
    ok(&out, "new");
    let shop = root.join("shop");
    let batch = "[{\"kind\":\"mod\",\"name\":\"catalog\"},{\"kind\":\"mod\",\"name\":\"catalog\"}]";
    let out = el_stdin(&shop, &["batch"], batch);
    assert_eq!(out.status.code(), Some(2), "exit must be 2 (violation):\n{}", text(&out));
    assert!(text(&out).contains("catalog"), "failure names the item:\n{}", text(&out));
    let out = el(&shop, &["tree"]);
    ok(&out, "tree");
    assert!(!text(&out).contains("catalog"), "nothing applied:\n{}", text(&out));
}

#[test]
fn batch_refuses_statements() {
    let root = fresh_root("stmt-refused");
    let out = el(&root, &["new", "shop"]);
    ok(&out, "new");
    let shop = root.join("shop");
    let batch = "[{\"kind\":\"stmt\",\"name\":\"el-0001\",\"body\":\"go();\"}]";
    let out = el_stdin(&shop, &["batch"], batch);
    assert_eq!(out.status.code(), Some(1), "exit must be 1 (usage):\n{}", text(&out));
    assert!(text(&out).contains("stmt"), "refusal names stmt:\n{}", text(&out));
}
