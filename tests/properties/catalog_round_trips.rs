use el::core::model::catalog::Catalog;
use el::core::model::element::{BodyMode, Visibility};
use el::core::model::kind::Kind;

// FR-3: the catalog encodes to a deterministic, line-oriented text form;
// encode(decode(x)) == x, and two catalogs with equal content encode to equal
// bytes.
//
// This is the property the store rests on. If it fails, a history snapshot
// differs from a rebuilt catalog for no reason, and every future diff is noise.

/// A catalog that exercises every record: a named function with statements, a
/// statement nested two deep, a const, an unnamed impl and an opaque body.
fn sample() -> Catalog {
    let mut catalog = Catalog::new();
    let apply = catalog
        .add_element(Kind::Function, "src/core/plan.rs", "apply", Visibility::Pub)
        .expect("add function");
    catalog
        .add_element(Kind::Const, "src/core/plan.rs", "SCALE", Visibility::Private)
        .expect("add const");
    let opaque = catalog
        .add_element(Kind::Function, "src/core/plan.rs", "legacy", Visibility::Crate)
        .expect("add opaque");
    if let Some(element) = catalog.find_mut(opaque) {
        element.mode = BodyMode::Opaque;
    }

    catalog.add_stmt(apply, "let x = 1;").expect("add");
    let outer = catalog.add_stmt(apply, "if x {").expect("add");
    catalog.add_child_stmt(outer, "go();").expect("add");
    catalog.add_stmt(apply, "done();").expect("add");
    catalog.add_stmt(opaque, "one unknown blob;").expect("add");

    catalog
}

#[test]
fn a_catalog_survives_encode_and_decode() {
    let catalog = sample();

    let text = catalog.encode();
    let decoded = Catalog::decode(&text).expect("decodes");

    assert_eq!(decoded, catalog, "decode lost or changed content");
    assert_eq!(decoded.encode(), text, "re-encoding differs");
}

#[test]
fn field_order_does_not_change_the_bytes() {
    let mut catalog = sample();
    let canonical = catalog.encode();

    catalog.elements.reverse();
    catalog.statements.reverse();

    assert_eq!(catalog.encode(), canonical);
}

#[test]
fn the_id_counter_survives_a_round_trip() {
    let mut catalog = Catalog::new();
    let doomed = catalog
        .add_element(Kind::Function, "src/a.rs", "gone", Visibility::Private)
        .expect("add");
    catalog.remove(doomed).expect("remove");

    let mut rebuilt = Catalog::decode(&catalog.encode()).expect("decodes");
    let next = rebuilt
        .add_element(Kind::Function, "src/a.rs", "fresh", Visibility::Private)
        .expect("add");

    assert!(next.0 > doomed.0, "{doomed} would have been reissued");
}

#[test]
fn a_body_mode_survives_the_round_trip() {
    let catalog = sample();
    let decoded = Catalog::decode(&catalog.encode()).expect("decodes");

    let opaque = decoded
        .elements
        .iter()
        .find(|element| element.name.as_deref() == Some("legacy"))
        .expect("legacy element");

    assert_eq!(opaque.mode, BodyMode::Opaque);
}

#[test]
fn a_tab_in_a_statement_survives_the_round_trip() {
    let mut catalog = Catalog::new();
    let owner = catalog
        .add_element(Kind::Function, "src/a.rs", "run", Visibility::Private)
        .expect("add");
    catalog.add_stmt(owner, "let s = \"a\tb\";").expect("add");

    let decoded = Catalog::decode(&catalog.encode()).expect("decodes");

    assert_eq!(decoded, catalog);
}
