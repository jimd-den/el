// ADR-0033: a --body path that is not a file must teach the
// file-or-stdin contract, not merely report that the path does
// not exist — the bench model repeated `--body "1"` and
// `--body "x * 2"` because nothing told it bodies come from
// files.

use crate::program::{self, el, ok};

fn out(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn add_with_a_missing_body_file_names_the_file_rule() {
    let built = program::build("body-path");
    ok(&el(&built.hello, &["add", "mod", "src/util.rs"]), "add mod");

    let refused = el(
        &built.hello,
        &[
            "add", "fn", "one", "--in", "src/util.rs", "--vis", "pub",
            "--returns", "u32", "--body", "/nonexistent-body.txt",
        ],
    );
    assert_eq!(
        refused.status.code(),
        Some(4),
        "a missing body file is an environment failure"
    );
    let text = out(&refused);
    assert!(text.contains("no file"), "the refusal names the file: {text}");
    assert!(
        text.contains("stdin"),
        "the refusal teaches the stdin alternative: {text}"
    );
}

#[test]
fn apply_line_with_a_missing_body_file_names_the_file_rule() {
    let built = program::build("body-path-apply");
    ok(&el(&built.hello, &["add", "mod", "src/util.rs"]), "add mod");

    let refused = crate::program::el_stdin(
        &built.hello,
        &["apply", "-"],
        "add fn one --in src/util.rs --vis pub --returns u32 --body /nonexistent-body.txt\n",
    );
    assert!(!refused.status.success(), "the apply line must refuse");
    let text = out(&refused);
    assert!(
        text.contains("apply line 1:"),
        "the refusal names its line: {text}"
    );
    assert!(text.contains("no file"), "the refusal names the file: {text}");
}
