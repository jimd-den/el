// The acceptance test M3 is defined by: a two-module catalog renders to the
// checked-in golden files, byte for byte, and changing one statement changes
// exactly one line (FR-67 — diff noise is a defect).
//
// The golden files were written by hand from the specification, not recorded
// from the output. A golden recorded from a bug pins the bug.

use el::core::model::catalog::Catalog;
use el::core::model::element::{NewElement, Visibility};
use el::core::model::kind::Kind;
use el::core::render::{File, render};
use std::fs;
use std::path::Path;

const GOLDEN: &str = "tests/fixtures/golden/two_modules";

fn new_el<'a>(
    kind: Kind,
    module: &'a str,
    signature: &'a str,
    visibility: Visibility,
) -> NewElement<'a> {
    NewElement {
        kind,
        module,
        signature,
        visibility,
    }
}

/// `src/main.rs` with a `main` that calls `greet`, and `src/greet.rs` with the
/// function it calls. Two modules, one statement each, one cross-module mod
/// declaration — the smallest catalog that needs every part of the renderer.
///
/// The body calls `greet::greet()`, not `greet()`, because a generated
/// `mod greet;` is a real namespace: the renderer decides where the file goes,
/// and the compiler makes that decision visible.
fn demo() -> Catalog {
    let mut catalog = Catalog::new();
    let main = catalog
        .add_element(&new_el(
            Kind::Function,
            "src/main.rs",
            "fn main()",
            Visibility::Private,
        ))
        .expect("add main");
    catalog.add_stmt(main, "greet::greet();").expect("add stmt");
    let greet = catalog
        .add_element(&new_el(
            Kind::Function,
            "src/greet.rs",
            "fn greet()",
            Visibility::Pub,
        ))
        .expect("add greet");
    catalog
        .add_stmt(greet, "println!(\"hi\");")
        .expect("add stmt");

    catalog
}

fn golden(path: &str) -> String {
    fs::read_to_string(Path::new(GOLDEN).join(path)).expect("golden file")
}

fn text_of<'a>(files: &'a [File], path: &str) -> &'a str {
    files
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.text.as_str())
        .unwrap_or_else(|| panic!("no rendered file at {path}"))
}

#[test]
fn a_two_module_catalog_renders_to_the_golden_files() {
    let projection = render(&demo(), "demo");

    assert_eq!(text_of(&projection.files, "src/main.rs"), golden("src/main.rs"));
    assert_eq!(text_of(&projection.files, "src/greet.rs"), golden("src/greet.rs"));
    assert_eq!(text_of(&projection.files, "Cargo.toml"), golden("Cargo.toml"));
}

#[test]
fn the_projection_contains_exactly_the_expected_files() {
    let projection = render(&demo(), "demo");

    let paths: Vec<&str> = projection
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();

    assert_eq!(paths, vec!["Cargo.toml", "src/greet.rs", "src/main.rs"]);
}

#[test]
fn changing_one_statement_changes_exactly_one_line() {
    let mut catalog = demo();
    let main = catalog
        .elements
        .iter()
        .find(|element| element.name.as_deref() == Some("main"))
        .expect("main element")
        .id;
    let stmt = catalog.children_of(main)[0].id;

    let before = render(&catalog, "demo");
    catalog
        .statements
        .iter_mut()
        .find(|statement| statement.id == stmt)
        .expect("statement")
        .text = "greet_twice();".to_string();
    let after = render(&catalog, "demo");

    let changed = changed_lines(
        text_of(&before.files, "src/main.rs"),
        text_of(&after.files, "src/main.rs"),
    );

    assert_eq!(changed, 1, "one statement edit must move one line");
    assert_eq!(text_of(&before.files, "src/greet.rs"), text_of(&after.files, "src/greet.rs"));
}

/// How many lines differ, positionally. An edit that moves one line changes one
/// position; a reordering or an indentation drift changes several, which is the
/// diff noise FR-67 calls a defect.
fn changed_lines(before: &str, after: &str) -> usize {
    let before: Vec<&str> = before.lines().collect();
    let after: Vec<&str> = after.lines().collect();

    assert_eq!(before.len(), after.len(), "the line count changed");
    before
        .iter()
        .zip(after.iter())
        .filter(|(old, new)| old != new)
        .count()
}
