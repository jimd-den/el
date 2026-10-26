// Golden tests for the read surface. The catalogs are built by hand, not recorded
// from the renderer, so a change in ordering or field spelling fails here rather
// than in a user's script.

use super::*;
use crate::core::history::entry::Entry;
use crate::core::history::object;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::{NewElement, Visibility};
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;
use crate::core::render::render;
use crate::core::text::json::{self, Json};

pub(super) fn fixture() -> Catalog {
    let mut catalog = Catalog::new();
    let main_module = add(&mut catalog, Kind::Module, "src/main.rs", "mod main");
    catalog.add_stmt(main_module, "// main module").expect("preamble");
    let main = add(&mut catalog, Kind::Function, "src/main.rs", "fn main()");
    catalog.add_stmt(main, "greet::greet();").expect("body");
    let point = add(&mut catalog, Kind::Struct, "src/main.rs", "struct Point");
    catalog.add_stmt(point, "x: u8,").expect("field");
    let implementation = add(&mut catalog, Kind::Impl, "src/main.rs", "impl Point");
    let block = catalog.add_stmt(implementation, "if ready {").expect("block");
    catalog.add_child_stmt(block, "go();").expect("child");

    let greet_module = add(&mut catalog, Kind::Module, "src/greet.rs", "mod greet");
    catalog.add_stmt(greet_module, "// greet module").expect("preamble");
    let greet = add(&mut catalog, Kind::Function, "src/greet.rs", "fn greet(name: &str) -> String");
    catalog.find_mut(greet).expect("greet").visibility = Visibility::Pub;
    catalog.add_stmt(greet, "/// Greets by name.").expect("doc");
    catalog.add_stmt(greet, "format!(\"hello, {name}\")").expect("body");
    catalog
}

fn add(catalog: &mut Catalog, kind: Kind, module: &str, signature: &str) -> ElId {
    catalog
        .add_element(&NewElement {
            kind,
            module,
            signature,
            visibility: Visibility::Private,
        })
        .expect("element")
}

pub(super) fn entries() -> Vec<Entry> {
    [
        (1, "-", "created demo"),
        (2, "mod src/greet.rs", "add mod src/greet.rs"),
        (3, "fn main", "add fn main"),
        (12, "fn greet", "add fn greet"),
    ]
    .into_iter()
    .map(|(seq, target, msg)| Entry {
        seq,
        ts: seq as i64,
        actor: "human".to_string(),
        op: crate::core::history::entry::Op::Add,
        target: target.to_string(),
        from: object::of(b"from"),
        to: object::of(b"to"),
        gate: "model".to_string(),
        msg: msg.to_string(),
    })
    .collect()
}

fn ids(payload: &Json) -> Vec<String> {
    payload
        .as_array()
        .expect("array")
        .iter()
        .map(|row| row.get("id").and_then(|v| v.as_str()).expect("id").to_string())
        .collect()
}

#[test]
fn ls_lists_every_element_in_module_then_id_order() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");

    let view = ls(&catalog, &projection, &Filter { module: None, kind: None, opaque: false });

    assert_eq!(view.lines.len(), 7, "header plus six elements");
    assert_eq!(
        ids(&view.payload),
        ["el-0010", "el-0012", "el-0001", "el-0003", "el-0005", "el-0007"]
    );
    assert!(view.lines[1].contains("mod"));
    assert!(view.lines[1].contains("split"));
}

#[test]
fn ls_filters_by_module_kind_and_opaque() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");

    let greet = ls(
        &catalog,
        &projection,
        &Filter { module: Some("greet".to_string()), kind: None, opaque: false },
    );
    assert_eq!(ids(&greet.payload), ["el-0010", "el-0012"]);

    let functions = ls(
        &catalog,
        &projection,
        &Filter { module: None, kind: Some(Kind::Function), opaque: false },
    );
    assert_eq!(ids(&functions.payload), ["el-0012", "el-0003"]);

    let opaque = ls(
        &catalog,
        &projection,
        &Filter { module: None, kind: None, opaque: true },
    );
    assert!(ids(&opaque.payload).is_empty());
}

#[test]
fn tree_without_statements_is_the_three_level_outline() {
    let catalog = fixture();

    let view = tree(&catalog, "demo", false);

    assert_eq!(
        view.lines.join("\n"),
        "crate demo\n  src/greet.rs  mod el-0010\n    el-0012  fn  greet::fn/greet\n  src/main.rs  mod el-0001\n    el-0003  fn  main::fn/main\n    el-0005  struct  main::struct/Point\n    el-0007  impl  -"
    );
}

#[test]
fn tree_with_statements_shows_preamble_bodies_and_nesting() {
    let catalog = fixture();

    let view = tree(&catalog, "demo", true);

    let text = view.lines.join("\n");
    assert!(text.contains("el-0011  // greet module"), "{text}");
    assert!(text.contains("el-0013  /// Greets by name."), "{text}");
    assert!(text.contains("el-0014  format!(\"hello, {name}\")"), "{text}");
    assert!(text.contains("el-0009  go();"), "{text}");
    assert!(text.contains("        el-0009"), "{text}");
}

#[test]
fn tree_payload_round_trips_through_the_project_writer_and_reader() {
    let catalog = fixture();

    let view = tree(&catalog, "demo", true);
    let text = json::write(&view.payload);

    assert_eq!(json::parse(&text).expect("parses"), view.payload);
}
