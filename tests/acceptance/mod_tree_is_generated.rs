// FR-15: `mod` declarations are generated from the module tree; users never
// write them.
//
// The case that matters is the one a hand-written crate gets wrong: an element in
// `src/core/plan.rs` needs `src/core.rs` to exist *and* to declare `mod plan;`,
// even though nobody would have created that file for a module with no items of
// its own.

use el::core::model::catalog::Catalog;
use el::core::model::element::{NewElement, Visibility};
use el::core::model::kind::Kind;
use el::core::render::render;

fn catalog_with(path: &str) -> Catalog {
    let mut catalog = Catalog::new();
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: path,
            signature: "fn f()",
            visibility: Visibility::Private,
        })
        .expect("add");
    catalog
}

#[test]
fn a_nested_module_is_declared_by_every_module_above_it() {
    let projection = render(&catalog_with("src/core/plan.rs"), "demo");

    let main = projection.file("src/main.rs").expect("root").text.as_str();
    let core = projection.file("src/core.rs").expect("intermediate").text.as_str();

    assert!(main.contains("\nmod core;\n"), "root does not declare core:\n{main}");
    assert!(core.contains("\nmod plan;\n"), "core does not declare plan:\n{core}");
}

#[test]
fn an_intermediate_module_is_created_even_with_no_items_of_its_own() {
    let projection = render(&catalog_with("src/core/plan.rs"), "demo");

    assert!(projection.file("src/core.rs").is_some());
}

#[test]
fn a_leaf_module_declares_nothing() {
    let projection = render(&catalog_with("src/core/plan.rs"), "demo");

    let plan = projection.file("src/core/plan.rs").expect("leaf").text.as_str();

    assert!(!plan.contains("mod "), "a leaf declared a child:\n{plan}");
}

#[test]
fn the_root_declares_top_level_modules_and_nothing_else() {
    let mut catalog = catalog_with("src/a.rs");
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/b.rs",
            signature: "fn g()",
            visibility: Visibility::Private,
        })
        .expect("add");

    let projection = render(&catalog, "demo");
    let main = projection.file("src/main.rs").expect("root").text.as_str();

    assert!(main.contains("\nmod a;\nmod b;\n"), "{main}");
}

/// A `mod` line never hides what the catalog marked visible. Without this, a
/// `pub fn` in `src/core/plan.rs` is unreachable from `src/main.rs` and the
/// rendered crate does not compile — the compiler said so before this rule
/// existed.
#[test]
fn a_module_holding_a_public_item_is_declared_pub() {
    let mut catalog = Catalog::new();
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/core/plan.rs",
            signature: "fn apply(n: u8) -> u8",
            visibility: Visibility::Pub,
        })
        .expect("add");
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/main.rs",
            signature: "fn main()",
            visibility: Visibility::Private,
        })
        .expect("add");

    let projection = render(&catalog, "demo");
    let main = projection.file("src/main.rs").expect("root").text.as_str();
    let core = projection.file("src/core.rs").expect("core").text.as_str();

    assert!(main.contains("\npub mod core;\n"), "root does not re-export core:\n{main}");
    assert!(core.contains("\npub mod plan;\n"), "core hides plan:\n{core}");
}

#[test]
fn a_module_holding_nothing_public_stays_private() {
    let projection = render(&catalog_with("src/core/plan.rs"), "demo");
    let core = projection.file("src/core.rs").expect("core").text.as_str();

    assert!(core.contains("\nmod plan;\n"), "core over-exposes plan:\n{core}");
}
