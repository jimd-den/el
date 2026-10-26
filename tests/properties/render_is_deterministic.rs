use el::core::model::catalog::Catalog;
use el::core::model::element::{NewElement, Visibility};
use el::core::model::kind::Kind;
use el::core::render::render;

// FR-13: `render` is deterministic — same catalog, same bytes, whatever order
// the catalog's vectors happen to be in.
//
// This is the property the history store rests on. If it fails, two snapshots of
// an unchanged project differ, every diff is noise, and undo has nothing exact
// to restore.
//
// Honest note: the property was green on first run, because the renderer sorts
// by content from the start. It is a pin, and the mutation that breaks it is
// removing the sort — recorded in the commit body rather than claimed as a test
// that drove the code.

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

/// Two modules, nested statements, an unnamed `impl`, an opaque body and a
/// preamble: enough that every ordering decision in the renderer is exercised.
fn sample() -> Catalog {
    let mut catalog = Catalog::new();
    catalog
        .add_element(&new_el(
            Kind::Module,
            "src/main.rs",
            "mod main",
            Visibility::Private,
        ))
        .expect("module");
    let run = catalog
        .add_element(&new_el(Kind::Function, "src/core.rs", "fn run()", Visibility::Pub))
        .expect("run");
    let helper = catalog
        .add_element(&new_el(
            Kind::Function,
            "src/core.rs",
            "fn helper(x: u8) -> u8",
            Visibility::Private,
        ))
        .expect("helper");
    catalog
        .add_element(&new_el(
            Kind::Impl,
            "src/core.rs",
            "impl Iterator for Wrap",
            Visibility::Private,
        ))
        .expect("impl");

    catalog.add_stmt(run, "let n = helper(1);").expect("stmt");
    let block = catalog.add_stmt(run, "if n > 0 {").expect("stmt");
    catalog.add_child_stmt(block, "go();").expect("stmt");

    catalog.add_stmt(helper, "todo!()").expect("stmt");
    catalog
}

#[test]
fn the_same_catalog_renders_the_same_bytes() {
    assert_eq!(render(&sample(), "demo"), render(&sample(), "demo"));
}

#[test]
fn the_order_of_the_catalogs_vectors_does_not_matter() {
    let canonical = render(&sample(), "demo");

    let mut shuffled = sample();
    shuffled.elements.reverse();
    shuffled.statements.reverse();

    assert_eq!(render(&shuffled, "demo"), canonical);
}

#[test]
fn the_span_map_is_in_the_same_order_however_the_catalog_is() {
    let canonical = render(&sample(), "demo");

    let mut shuffled = sample();
    shuffled.elements.reverse();
    shuffled.statements.reverse();
    let shuffled = render(&shuffled, "demo");

    assert_eq!(shuffled.spans, canonical.spans);
}

#[test]
fn every_file_ends_with_one_newline() {
    for file in render(&sample(), "demo").files {
        assert!(file.text.ends_with('\n'), "{} does not end in a newline", file.path);
        assert!(!file.text.ends_with("\n\n"), "{} ends in a blank line", file.path);
    }
}
