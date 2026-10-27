// `--edition` used to be accepted and ignored, so `--edition 2021` produced a
// 2024 crate without a word. Only the pinned edition is built; anything else is a
// refusal that names the flag, and 2024 is accepted because it is what is written.

use crate::program::{el, fresh, ok};

#[test]
fn the_pinned_edition_is_accepted() {
    let root = fresh("edition-2024");

    ok(&el(&root, &["new", "hello", "--edition", "2024"]), "new --edition 2024");
}

#[test]
fn another_edition_is_refused_instead_of_ignored() {
    let root = fresh("edition-2021");

    let output = el(&root, &["new", "hello", "--edition", "2021"]);

    assert_eq!(output.status.code(), Some(1));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains("2021"), "{text}");
    assert!(!root.join("hello/Cargo.toml").exists(), "nothing was created");
}
