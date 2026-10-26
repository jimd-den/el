// FR-72: the split mode is policy, not a guess. `never` keeps a body opaque;
// `always` refuses a body the grammar cannot prove; `auto` falls back to opaque.
// The reader sees the mode in `el show`, so the choice is never silent.

use crate::program::{self, el, el_stdin, ok};

fn policy(hello: &std::path::Path, split: &str) {
    let path = hello.join("el.policy");
    let text = std::fs::read_to_string(&path).expect("policy");
    std::fs::write(&path, format!("{text}split = {split}\n")).expect("write policy");
}

fn add(hello: &std::path::Path, body: &str) -> std::process::Output {
    el_stdin(hello, &["add", "fn", "opaque", "--in", "src/greet.rs", "--sig", "fn opaque()", "--body", "-"], body)
}

/// The auto-fallback case measures the split policy, not compilation: its
/// body is an unterminated string, which the compiler gate would refuse
/// before the policy question is ever reached.
fn add_fast(hello: &std::path::Path, body: &str) -> std::process::Output {
    el_stdin(hello, &["add", "fn", "opaque", "--in", "src/greet.rs", "--sig", "fn opaque()", "--body", "-", "--fast"], body)
}

#[test]
fn never_stores_the_body_opaque_and_shows_the_mode() {
    let built = program::build("split-never");
    policy(&built.hello, "never");

    let output = add(&built.hello, "let a = 1;\nlet b = 2;\n");
    ok(&output, "add never");
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let show = el(&built.hello, &["show", &id, "--json"]);
    ok(&show, "show");
    let line = String::from_utf8_lossy(&show.stdout);
    assert!(line.contains(r#""mode":"opaque""#), "{line}");
}

#[test]
fn always_refuses_what_auto_keeps_opaque() {
    let built = program::build("split-modes");
    policy(&built.hello, "always");
    let refused = add(&built.hello, "log(\"open");
    assert_eq!(refused.status.code(), Some(2), "{}", String::from_utf8_lossy(&refused.stdout));

    policy(&built.hello, "auto");
    let accepted = add_fast(&built.hello, "log(\"open");
    ok(&accepted, "auto fallback");
    let id = String::from_utf8_lossy(&accepted.stdout).trim().to_string();
    let show = el(&built.hello, &["show", &id, "--json"]);
    assert!(String::from_utf8_lossy(&show.stdout).contains(r#""mode":"opaque""#));
}
