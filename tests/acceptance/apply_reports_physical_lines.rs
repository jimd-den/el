// A script failure after blank and comment lines must name the line the
// human sees in the file, not the line's position among the changes.
// Parse errors already carry the physical line; build-stage failures did
// not, because the builder re-enumerated the stripped lines.

use crate::program::{el, el_stdin, fresh, ok};

fn text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn a_build_failure_after_comment_names_the_physical_line() {
    let root = fresh("apply-physical-line");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let script = "# a note\n\nrm el-9999\n";

    let output = el_stdin(&hello, &["apply"], script);

    assert_eq!(output.status.code(), Some(6), "{}", text(&output));
    assert!(
        text(&output).contains("apply line 3"),
        "must name physical line 3, not stripped line 1:\n{}",
        text(&output)
    );
}
