// An author writes a snippet to `/tmp/body.rs` and names it: `--body /tmp/body.rs`.
// That read used to go through the store's path resolver, which refuses anything
// that is not crate-relative, so an absolute path failed with "not a file" — and
// was reported as `el: el: /tmp/body.rs: not a file`, because the port's message
// carried a prefix the edge adds again. Reads the user names are allowed outside
// the crate; writes stay inside it.

use crate::program::{build, el, el_stdin, fresh, ok};
use std::fs;

#[test]
fn a_body_can_come_from_an_absolute_path() {
    let built = build("source-absolute");
    let outside = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(fresh("source-absolute-body"));
    let body = outside.join("body.rs");
    fs::write(&body, "let n = 1;").expect("body");

    let output = el_stdin(
        &built.hello,
        &[
            "add",
            "fn",
            "absolute",
            "--in",
            "src/greet.rs",
            "--sig",
            "fn absolute()",
            "--body",
            body.to_str().expect("utf8"),
        ],
        "",
    );

    ok(&output, "add with absolute --body");
    assert!(
        fs::read_to_string(built.hello.join("src/greet.rs"))
            .expect("greet")
            .contains("let n = 1;")
    );
}

#[test]
fn a_preamble_can_come_from_an_absolute_path() {
    let built = build("source-absolute-preamble");
    let outside = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(fresh("source-absolute-preamble-file"));
    let text = outside.join("preamble.txt");
    fs::write(&text, "// a module read from elsewhere").expect("preamble");

    let output = el(
        &built.hello,
        &[
            "add",
            "mod",
            "src/extra.rs",
            "--preamble",
            text.to_str().expect("utf8"),
        ],
    );

    ok(&output, "add mod with absolute --preamble");
    assert!(
        fs::read_to_string(built.hello.join("src/extra.rs"))
            .expect("extra")
            .contains("a module read from elsewhere")
    );
}

#[test]
fn a_missing_source_is_reported_once_with_its_path() {
    let built = build("source-missing");

    let output = el_stdin(
        &built.hello,
        &[
            "add", "fn", "gone", "--in", "src/greet.rs", "--sig", "fn gone()", "--body",
            "/tmp/el-no-such-snippet.rs",
        ],
        "",
    );

    assert_eq!(output.status.code(), Some(4), "{}", String::from_utf8_lossy(&output.stderr));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains("/tmp/el-no-such-snippet.rs"), "{text}");
    assert!(!text.contains("el: el:"), "{text}");
}

#[test]
fn a_body_can_come_from_a_relative_path_that_leaves_the_crate() {
    let built = build("source-relative");
    let outside = built.hello.parent().expect("parent").join("snippet.rs");
    fs::write(&outside, "let n = 2;").expect("snippet");

    let output = el_stdin(
        &built.hello,
        &[
            "add", "fn", "relative", "--in", "src/greet.rs", "--sig", "fn relative()",
            "--body", "../snippet.rs",
        ],
        "",
    );

    ok(&output, "add with relative --body");
    assert!(
        fs::read_to_string(built.hello.join("src/greet.rs"))
            .expect("greet")
            .contains("let n = 2;")
    );
}
