// Tests for the module layout: one declaration per item, one `mod` line per
// child, and the ordering that makes FR-13 hold. Separate from `rust.rs` because
// the layout code crossed the warning band, and the sibling-file convention is
// already how this package keeps its tests.

use super::*;
use crate::core::model::element::{NewElement, Visibility};
use crate::core::model::id::ElId;

fn element(signature: &str, visibility: Visibility) -> Element {
    Element::create(
        ElId(1),
        &NewElement {
            kind: Kind::Function,
            module: "src/a.rs",
            signature,
            visibility,
        },
    )
    .expect("valid")
}

#[test]
fn a_declaration_is_visibility_keyword_name_and_head() {
    let element = element("fn first<T>(xs: &[T]) -> Option<&T>", Visibility::Pub);

    assert_eq!(
        declaration(&element),
        "pub fn first<T>(xs: &[T]) -> Option<&T>"
    );
}

#[test]
fn a_private_declaration_has_no_prefix() {
    assert_eq!(declaration(&element("fn main", Visibility::Private)), "fn main");
}

#[test]
fn a_crate_visible_declaration_is_pub_crate() {
    assert_eq!(
        declaration(&element("fn helper", Visibility::Crate)),
        "pub(crate) fn helper"
    );
}

#[test]
fn an_impl_declares_without_a_name() {
    let element = Element::create(
        ElId(2),
        &NewElement {
            kind: Kind::Impl,
            module: "src/a.rs",
            signature: "impl Display for Point",
            visibility: Visibility::Private,
        },
    )
    .expect("valid");

    assert_eq!(declaration(&element), "impl Display for Point");
}

#[test]
fn an_opaque_module_renders_its_file_byte_for_byte() {
    use crate::core::model::element::{BodyMode, NewElement};
    use crate::core::model::kind::Kind;

    let mut catalog = crate::core::model::catalog::Catalog::new();
    let id = catalog
        .add_element(&NewElement {
            kind: Kind::Module,
            module: "src/main.rs",
            signature: "mod main",
            visibility: Visibility::Private,
        })
        .expect("module");
    catalog.add_stmt(id, "fn main() {\n    println!(\"hi\");\n}\n").expect("body");
    catalog.find_mut(id).expect("module").mode = BodyMode::Opaque;

    let projection = crate::core::render::render(&catalog, "opaque");

    assert_eq!(
        projection.file("src/main.rs").expect("file").text,
        "fn main() {\n    println!(\"hi\");\n}\n"
    );
}

#[test]
fn a_test_renders_with_its_attribute_line() {
    use crate::core::model::element::NewElement;
    use crate::core::model::kind::Kind;

    let mut catalog = crate::core::model::catalog::Catalog::new();
    catalog
        .add_element(&NewElement {
            kind: Kind::Module,
            module: "src/util.rs",
            signature: "mod util",
            visibility: Visibility::Private,
        })
        .expect("module");
    let test = catalog.add_element(&NewElement {
        kind: Kind::Test,
        module: "src/util.rs",
        signature: "fn checks_answer()",
        visibility: Visibility::Private,
    });
    assert!(test.is_ok(), "a test element must be creatable");
    let test = test.expect("test");
    catalog.add_stmt(test, "assert_eq!(answer(), 42);").expect("body");

    let projection = crate::core::render::render(&catalog, "demo");
    let text = &projection.file("src/util.rs").expect("file").text;

    assert!(text.contains("#[test]\nfn checks_answer() {"), "rendered:\n{text}");
    assert!(text.contains("assert_eq!(answer(), 42);"), "rendered:\n{text}");
}
