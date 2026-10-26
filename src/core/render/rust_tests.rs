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
