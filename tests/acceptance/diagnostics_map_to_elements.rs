// FR-17: the tool emits a span map — path → element id per line range — with
// every projection. The diagnostic delta reads it, so the question this test asks
// is the one a compiler error asks: given a file and a line, whose line is it?
//
// Innermost wins. A statement sits inside a function, so an error on a statement
// line must name the statement and not its owner (FR-68); the owner is reachable
// through the catalog, and losing that distinction is what makes an agent edit
// the wrong thing.

use el::core::model::catalog::Catalog;
use el::core::model::element::{NewElement, Visibility};
use el::core::model::kind::Kind;
use el::core::render::{Projection, render};

fn demo() -> (Catalog, Projection) {
    let mut catalog = Catalog::new();
    let main = catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/main.rs",
            signature: "fn main()",
            visibility: Visibility::Private,
        })
        .expect("add");
    catalog.add_stmt(main, "go();\n").expect("stmt");
    let greet = catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/greet.rs",
            signature: "fn greet()",
            visibility: Visibility::Pub,
        })
        .expect("add");
    catalog.add_stmt(greet, "println!(\"hi\");\n").expect("stmt");

    let projection = render(&catalog, "demo");

    (catalog, projection)
}

/// The 1-based line of the first line containing `needle`.
fn line_of(text: &str, needle: &str) -> u32 {
    text.lines()
        .position(|line| line.contains(needle))
        .map(|index| index as u32 + 1)
        .unwrap_or_else(|| panic!("no line contains {needle}"))
}

#[test]
fn a_statement_line_names_the_statement() {
    let (catalog, projection) = demo();
    let text = projection.file("src/main.rs").expect("file").text.as_str();
    let statement = catalog.elements[0].id;

    let owner = projection.element_at("src/main.rs", line_of(text, "go();"));

    assert_eq!(owner, Some(catalog.children_of(statement)[0].id));
}

#[test]
fn a_declaration_line_names_the_item() {
    let (catalog, projection) = demo();
    let text = projection.file("src/greet.rs").expect("file").text.as_str();
    let greet = catalog
        .elements
        .iter()
        .find(|element| element.name.as_deref() == Some("greet"))
        .expect("greet")
        .id;

    assert_eq!(
        projection.element_at("src/greet.rs", line_of(text, "fn greet()")),
        Some(greet)
    );
}

#[test]
fn a_line_outside_every_element_belongs_to_nobody() {
    let (_, projection) = demo();

    assert_eq!(projection.element_at("src/main.rs", 1), None, "the marker line");
    assert_eq!(projection.element_at("src/main.rs", 99), None, "past the end");
    assert_eq!(projection.element_at("src/absent.rs", 5), None, "no such file");
}

#[test]
fn a_span_points_at_a_line_that_exists() {
    let (_, projection) = demo();

    for span in &projection.spans {
        let file = projection.file(&span.path).expect("span names a rendered file");
        let lines = file.text.lines().count() as u32;

        assert!(span.first >= 1 && span.last <= lines, "{span:?} out of range");
        assert!(span.first <= span.last, "{span:?} is inverted");
    }
}
