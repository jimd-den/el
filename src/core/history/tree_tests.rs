// Tests for `tree`; sibling of the module they exercise (§4.4).
//
// Every assertion here is about *canonical bytes*, because a tree's only job is to be
// one name for one projection. A test that compared structures would let two
// encodings of the same tree pass, which is exactly what content addressing forbids.

use super::*;
use crate::core::history::object::of;

fn tree(pairs: &[(&str, &[u8])]) -> Tree {
    Tree::new(
        pairs
            .iter()
            .map(|(path, bytes)| (path.to_string(), of(bytes)))
            .collect(),
    )
    .expect("distinct paths")
}

#[test]
fn a_tree_encodes_one_path_tab_digest_line_per_file() {
    let one = tree(&[("src/lib.rs", b"pub mod core;\n")]);

    assert_eq!(
        one.encode(),
        format!("src/lib.rs\t{}\n", object::show(&of(b"pub mod core;\n")))
    );
}

#[test]
fn entries_are_sorted_by_path_whatever_order_they_arrived_in() {
    let unsorted = tree(&[
        ("src/main.rs", b"fn main() {}\n"),
        ("Cargo.toml", b"[package]\n"),
        ("src/app.rs", b"pub fn run() {}\n"),
    ]);

    let paths: Vec<&str> = unsorted.paths().collect();

    assert_eq!(paths, ["Cargo.toml", "src/app.rs", "src/main.rs"]);
}

#[test]
fn two_trees_with_the_same_files_encode_to_the_same_bytes() {
    let first = tree(&[("a.rs", b"a"), ("b.rs", b"b")]);
    let second = tree(&[("b.rs", b"b"), ("a.rs", b"a")]);

    assert_eq!(first.encode(), second.encode());
}

#[test]
fn encode_and_decode_round_trip_byte_identically() {
    let original = tree(&[("el.policy", b"ceiling = 200\n"), ("src/lib.rs", b"")]);

    let decoded = Tree::decode(&original.encode()).expect("our own encoding");

    assert_eq!(decoded, original);
    assert_eq!(decoded.encode(), original.encode());
}

#[test]
fn two_entries_for_one_path_are_refused_not_resolved() {
    let error = Tree::new(vec![
        ("src/lib.rs".to_string(), of(b"first")),
        ("src/lib.rs".to_string(), of(b"second")),
    ])
    .expect_err("a repeated path has no single meaning");

    assert_eq!(error.path, "src/lib.rs");
    assert!(
        error
            .to_string()
            .starts_with("env.corrupt: tree has two entries for ")
    );
}

#[test]
fn decoding_a_repeated_path_is_refused_as_well() {
    let line = format!("src/lib.rs\t{}\n", object::show(&of(b"a")));

    assert_eq!(Tree::decode(&format!("{line}{line}")), None);
}

#[test]
fn the_diff_of_a_tree_against_itself_is_empty() {
    let one = tree(&[("a.rs", b"a")]);

    assert!(one.diff(&one).is_empty());
}

#[test]
fn the_diff_names_what_was_added_removed_and_changed() {
    let before = tree(&[
        ("kept.rs", b"same"),
        ("gone.rs", b"x"),
        ("edited.rs", b"v1"),
    ]);
    let after = tree(&[("kept.rs", b"same"), ("new.rs", b"y"), ("edited.rs", b"v2")]);

    let diff = before.diff(&after);

    assert_eq!(diff.added, ["new.rs"]);
    assert_eq!(diff.removed, ["gone.rs"]);
    assert_eq!(diff.changed, ["edited.rs"]);
    assert!(!diff.is_empty());
}

#[test]
fn the_diff_is_symmetric_when_the_arguments_swap_sides() {
    let before = tree(&[("gone.rs", b"x"), ("kept.rs", b"same")]);
    let after = tree(&[("new.rs", b"y"), ("kept.rs", b"same")]);

    let forward = before.diff(&after);
    let backward = after.diff(&before);

    assert_eq!(forward.added, backward.removed);
    assert_eq!(forward.removed, backward.added);
}

#[test]
fn a_digest_is_found_by_its_path_and_nothing_else_is() {
    let one = tree(&[("a.rs", b"a")]);

    assert_eq!(one.digest_of("a.rs"), Some(&of(b"a")));
    assert_eq!(one.digest_of("b.rs"), None);
}

#[test]
fn decoding_refuses_a_line_that_is_not_path_tab_digest() {
    assert_eq!(Tree::decode("no tab here\n"), None);
    assert_eq!(Tree::decode(&format!("a.rs\t{}\n", "z".repeat(64))), None);
    assert_eq!(
        Tree::decode(&format!("\t{}\n", object::show(&of(b"a")))),
        None
    );
}

#[test]
fn an_empty_tree_encodes_to_nothing_and_decodes_back_to_empty() {
    let empty = Tree::default();

    assert_eq!(empty.encode(), "");
    assert_eq!(Tree::decode(""), Some(empty));
    assert!(Tree::decode("").expect("empty").is_empty());
}
