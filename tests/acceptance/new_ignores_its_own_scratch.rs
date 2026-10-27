// `el new` writes a store into `.el/`, builds in `target/`, and stages every
// projection write through `tmp/`. The generated `.gitignore` named the first two
// and not the third, so a fresh crate showed an untracked `tmp/` in `git status`
// before the author had written a line — the tool's own scratch as their noise.

use crate::program::{el, el_stdin, fresh, ok};
use std::fs;

#[test]
fn a_new_crate_ignores_every_directory_el_creates() {
    let root = fresh("new-gitignore");

    ok(&el(&root, &["new", "hello"]), "el new");

    let ignore = fs::read_to_string(root.join("hello/.gitignore")).expect(".gitignore");
    assert_eq!(ignore.lines().collect::<Vec<_>>(), vec![".el/", "target/", "tmp/"]);
}

#[test]
fn the_scratch_directory_is_the_one_el_stages_in() {
    let root = fresh("new-gitignore-scratch");
    ok(&el(&root, &["new", "hello"]), "el new");
    let hello = root.join("hello");

    ok(
        &el_stdin(&hello, &["add", "fn", "one", "--sig", "fn one()", "--body", "-"], "let a = 1;"),
        "add",
    );

    assert!(hello.join("tmp").is_dir(), "staging directory missing");
}
