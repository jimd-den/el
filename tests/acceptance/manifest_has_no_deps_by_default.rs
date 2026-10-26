// FR-16: `Cargo.toml` is generated and contains **no** dependency table unless
// the catalog declares one.
//
// Absence, not emptiness, is the requirement. An empty `[dependencies]` invites a
// human to fill it in, and the next render silently deletes the edit — the
// failure mode ADR-0009 exists to prevent, one layer down.

use el::core::model::catalog::Catalog;
use el::core::model::element::{NewElement, Visibility};
use el::core::model::kind::Kind;
use el::core::render::render;

fn catalog_with_one_function() -> Catalog {
    let mut catalog = Catalog::new();
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/main.rs",
            signature: "fn main()",
            visibility: Visibility::Private,
        })
        .expect("add");
    catalog
}

fn manifest() -> String {
    render(&catalog_with_one_function(), "demo")
        .file("Cargo.toml")
        .expect("manifest")
        .text
        .clone()
}

#[test]
fn a_manifest_has_no_dependency_table() {
    let manifest = manifest();

    assert!(!manifest.contains("[dependencies]"), "{manifest}");
    assert!(!manifest.contains("[dev-dependencies]"), "{manifest}");
    assert!(!manifest.contains("[build-dependencies]"), "{manifest}");
}

#[test]
fn a_manifest_names_the_crate_it_belongs_to() {
    assert!(manifest().contains("name = \"demo\""));
}

#[test]
fn a_manifest_is_part_of_the_projection() {
    let projection = render(&catalog_with_one_function(), "demo");

    assert!(projection.file("Cargo.toml").is_some());
}
