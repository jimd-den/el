// Plain-words adds assemble the signature, so the model never writes
// sig syntax: --takes/--returns for fn and test, --type/--value for
// const. --sig stays the escape hatch, and mixing the two is refused.

use crate::program::{el, el_stdin, fresh, ok};

fn show_source(hello: &std::path::Path, id: &str) -> String {
    let shown = el(hello, &["show", id, "--source"]);
    ok(&shown, "el show --source");
    String::from_utf8_lossy(&shown.stdout).into_owned()
}

fn id_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn find_id(hello: &std::path::Path, name: &str) -> String {
    let found = el(hello, &["find", name]);
    ok(&found, "el find");
    let text = String::from_utf8_lossy(&found.stdout);
    text.split_whitespace()
        .find(|token| token.starts_with("el-"))
        .expect("find prints an id")
        .to_string()
}

fn crate_with_mod() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = fresh("plain-words");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/util.rs"]), "add mod");
    (root, hello)
}

#[test]
fn a_function_assembles_from_takes_and_returns() {
    let (_root, hello) = crate_with_mod();
    let added = el_stdin(
        &hello,
        &[
            "add", "fn", "greet", "--in", "src/util.rs", "--takes", "name: &str", "--returns",
            "String", "--body", "-",
        ],
        "format!(\"hello, {name}\")",
    );
    ok(&added, "add fn with plain words");

    let source = show_source(&hello, &id_of(&added));
    assert!(source.contains("fn greet(name: &str) -> String"), "assembled:\n{source}");
}

#[test]
fn a_test_without_a_sig_assembles_to_a_test_function() {
    let (_root, hello) = crate_with_mod();
    let bare = el(&hello, &["add", "test", "checks_nothing", "--in", "src/util.rs"]);
    ok(&bare, "add test without a sig must assemble, not refuse");
    let source = show_source(&hello, &id_of(&bare));
    assert!(source.contains("#[test]"), "assembled:\n{source}");
    assert!(source.contains("fn checks_nothing()"), "assembled:\n{source}");
}

#[test]
fn a_const_assembles_from_type_and_value() {
    let (_root, hello) = crate_with_mod();
    let added = el(
        &hello,
        &["add", "const", "LIMIT", "--in", "src/util.rs", "--type", "usize", "--value", "10"],
    );
    ok(&added, "add const with plain words");

    let source = show_source(&hello, &id_of(&added));
    assert!(source.contains("const LIMIT: usize = 10"), "assembled:\n{source}");
}

#[test]
fn a_batch_item_assembles_from_plain_words() {
    let (_root, hello) = crate_with_mod();
    let out = el_stdin(
        &hello,
        &["batch"],
        "[{\"kind\":\"fn\",\"name\":\"double\",\"in\":\"src/util.rs\",\"takes\":\"x: u32\",\"returns\":\"u32\",\"body\":\"x * 2\"}]",
    );
    ok(&out, "batch with plain words");

    let source = show_source(&hello, &find_id(&hello, "double"));
    assert!(source.contains("fn double(x: u32) -> u32"), "assembled:\n{source}");
}

#[test]
fn a_sig_and_plain_words_together_are_refused() {
    let (_root, hello) = crate_with_mod();
    let refused = el(
        &hello,
        &[
            "add", "fn", "greet", "--in", "src/util.rs", "--sig", "fn greet()", "--takes",
            "name: &str",
        ],
    );
    assert_eq!(refused.status.code(), Some(1), "mixing must be a usage refusal");
}
