// The compiler gate reports only what is new: a change that adds one error
// to a tree that already carries one is refused with "1 new error" and
// "1 pre-existing", the bytes on disk are the bytes before the attempt, and
// the journal records the attempt as `rejected:1`. A timeout is an
// environment error, never a verdict.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/gate-reports";
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
    el_stdin_env(cwd, args, body, &[])
}

fn el_stdin_env(cwd: &Path, args: &[&str], body: &str, envs: &[(&str, &str)]) -> Output {
    let mut child = Command::new(BIN)
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(envs.iter().copied())
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
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn add_fn(cwd: &Path, name: &str, body: &str, fast: bool) -> Output {
    let sig = format!("fn {name}()");
    let mut owned: Vec<String> = vec![
        "add".to_string(),
        "fn".to_string(),
        name.to_string(),
        "--sig".to_string(),
        sig,
        "--body".to_string(),
        "-".to_string(),
    ];
    if fast {
        owned.push("--fast".to_string());
    }
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    el_stdin(cwd, &refs, body)
}

#[test]
fn a_rejection_reports_only_the_new_error_and_restores_the_bytes() {
    let root = fresh_root("reject");
    let shop = root.join("shop");
    ok(&el(&root, &["new", "shop"]), "el new");
    ok(&add_fn(&shop, "main", "", false), "add main");
    ok(&add_fn(&shop, "broken_one", "undefined_one();", true), "add broken_one --fast");
    let before = fs::read(shop.join("src/main.rs")).expect("read main.rs");

    let rejected = add_fn(&shop, "broken_two", "undefined_two();", false);

    assert_eq!(rejected.status.code(), Some(3), "exit\n{}", all_text(&rejected));
    let text = all_text(&rejected);
    assert!(text.contains("1 new error"), "counts:\n{text}");
    assert!(text.contains("1 pre-existing"), "counts:\n{text}");
    assert_eq!(fs::read(shop.join("src/main.rs")).expect("read main.rs"), before);

    let log = el(&shop, &["log"]);
    let log_text = all_text(&log);
    assert!(log_text.contains("rejected:1"), "log:\n{log_text}");

    let json = el_stdin(
        &shop,
        &["--json", "add", "fn", "broken_two", "--sig", "fn broken_two()", "--body", "-"],
        "undefined_two();",
    );
    let json_text = String::from_utf8_lossy(&json.stdout).into_owned();
    assert!(json_text.contains("gate.rejected"), "json:\n{json_text}");
    assert!(json_text.contains("\"new\":1"), "json:\n{json_text}");
    assert!(json_text.contains("\"existing\":1"), "json:\n{json_text}");
}

#[test]
fn a_timed_out_gate_is_an_environment_error_not_a_verdict() {
    let root = fresh_root("timeout");
    let shop = root.join("shop-timeout");
    ok(&el(&root, &["new", "shop-timeout"]), "el new");
    let before = fs::read(shop.join("src/main.rs")).expect("read main.rs");

    let timed_out = el_stdin_env(
        &shop,
        &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
        "",
        &[("EL_CARGO_TIMEOUT", "0")],
    );

    assert_eq!(timed_out.status.code(), Some(4), "exit\n{}", all_text(&timed_out));
    assert!(
        all_text(&timed_out).contains("timed out after 0s"),
        "output\n{}",
        all_text(&timed_out)
    );
    assert_eq!(fs::read(shop.join("src/main.rs")).expect("read main.rs"), before);

    let json = el_stdin_env(
        &shop,
        &["--json", "add", "fn", "main", "--sig", "fn main()", "--body", "-"],
        "",
        &[("EL_CARGO_TIMEOUT", "0")],
    );
    let json_text = String::from_utf8_lossy(&json.stdout).into_owned();
    assert!(json_text.contains("env.timeout"), "json:\n{json_text}");
}
