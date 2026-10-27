// FR-74: `--body-mode` is the per-command override of the project's split
// policy. Before this existed the flag was parsed into nothing, so an author who
// asked for `opaque` got the policy's answer and no way to tell. The reader sees
// the stored mode in `el show`, and a mode without a body is refused before the
// crate is touched.

use crate::program::{build, el, el_stdin, ok};

fn mode(built: &crate::program::Built, id: &str) -> String {
    let show = el(&built.hello, &["show", id, "--json"]);
    ok(&show, "show");
    let text = String::from_utf8_lossy(&show.stdout).into_owned();
    let key = "\"mode\":\"";
    let start = text.find(key).expect("mode in show payload") + key.len();
    text[start..start + text[start..].find('"').expect("closing quote")].to_string()
}

#[test]
fn body_mode_overrides_the_policy_and_is_visible() {
    let built = build("body-mode");

    let opaque = el_stdin(
        &built.hello,
        &["edit", &built.main, "--body", "-", "--body-mode", "opaque"],
        "let a = 1;\nlet b = 2;",
    );
    ok(&opaque, "edit --body-mode opaque");
    assert_eq!(mode(&built, &built.main), "opaque");

    let split = el_stdin(
        &built.hello,
        &["edit", &built.main, "--body", "-", "--body-mode", "split"],
        "let a = 1;\nlet b = 2;",
    );
    ok(&split, "edit --body-mode split");
    assert_eq!(mode(&built, &built.main), "split");
}

#[test]
fn body_mode_without_a_body_is_refused() {
    let built = build("body-mode-refused");
    let before = std::fs::read_to_string(built.hello.join("src/main.rs")).expect("main");

    let output = el(&built.hello, &["edit", &built.main, "--body-mode", "opaque"]);

    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        std::fs::read_to_string(built.hello.join("src/main.rs")).expect("main"),
        before
    );
}
