// Tests for `refs`; sibling of the module they exercise (§4.4).

use super::*;
use crate::core::model::element::{NewElement, Visibility};
use crate::core::model::kind::Kind;

fn element(catalog: &mut Catalog, signature: &str) -> ElId {
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/a.rs",
            signature,
            visibility: Visibility::Private,
        })
        .expect("add")
}

/// A catalog with one function `apply` and one caller whose body is `body`.
fn with_caller(body: &str) -> (Catalog, ElId, ElId) {
    let mut catalog = Catalog::new();
    let apply = element(&mut catalog, "fn apply(p: &Plan)");
    let caller = element(&mut catalog, "fn run(p: &Plan)");
    catalog.add_stmt(caller, body).expect("add stmt");
    (catalog, apply, caller)
}

#[test]
fn a_call_in_another_body_is_a_reference() {
    let (catalog, _, caller) = with_caller("apply(p);");

    let hits = references(&catalog, "apply").expect("scannable");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].site, Site::Stmt(catalog.children_of(caller)[0].id));
}

#[test]
fn a_name_in_a_string_is_not_a_reference() {
    let (catalog, _, _) = with_caller(r#"log("apply failed");"#);

    assert_eq!(references(&catalog, "apply").expect("scannable"), Vec::new());
}

#[test]
fn a_longer_identifier_is_not_a_reference() {
    let (catalog, _, _) = with_caller("applier(p);");

    assert_eq!(references(&catalog, "apply").expect("scannable"), Vec::new());
}

#[test]
fn a_head_is_a_site_too() {
    let mut catalog = Catalog::new();
    let point = catalog
        .add_element(&NewElement {
            kind: Kind::Struct,
            module: "src/a.rs",
            signature: "struct point",
            visibility: Visibility::Private,
        })
        .expect("add");
    let user = element(&mut catalog, "fn draw(p: point)");

    let hits = references(&catalog, "point").expect("scannable");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].site, Site::Head(user));
    assert_ne!(hits[0].site.element(), point);
}

#[test]
fn an_offset_points_at_the_identifier() {
    let (catalog, _, _) = with_caller("if ready { apply(p); }");

    let hits = references(&catalog, "apply").expect("scannable");
    let stmt = catalog.find_stmt(hits[0].site.element()).expect("stmt");

    assert_eq!(&stmt.text[hits[0].offset..], "apply(p); }");
}

#[test]
fn rewrite_keeps_strings_comments_and_similar_names() {
    let text = "apply(a); // apply\nlog(\"apply\"); applier();";

    assert_eq!(
        rewrite(text, "apply", "dispatch").expect("scannable"),
        "dispatch(a); // apply\nlog(\"apply\"); applier();"
    );
}

#[test]
fn rewrite_refuses_a_body_it_cannot_scan() {
    assert_eq!(rewrite("apply(\"open);", "apply", "dispatch"), None);
}

#[test]
fn an_unscannable_body_makes_the_whole_index_unknowable() {
    let (catalog, _, _) = with_caller("apply(\"open);");

    assert_eq!(references(&catalog, "apply"), None);
}

#[test]
fn names_in_use_names_only_what_is_referred_to() {
    let (catalog, _, _) = with_caller("apply(p);");
    let names = vec!["apply".to_string(), "nowhere".to_string()];

    assert_eq!(names_in_use(&catalog, &names), Some(vec!["apply".to_string()]));
}

/// A catalog with a module `greet` holding `fn greet`, plus `main` calling
/// `greet::greet()`: the shape where a name and its module are spelled alike.
fn module_and_function_named_alike() -> (Catalog, ElId, ElId, ElId) {
    let mut catalog = Catalog::new();
    let module = catalog
        .add_element(&NewElement {
            kind: Kind::Module,
            module: "src/greet.rs",
            signature: "mod greet",
            visibility: Visibility::Pub,
        })
        .expect("add module");
    let greet = element(&mut catalog, "fn greet()");
    let main = element(&mut catalog, "fn main()");
    catalog.add_stmt(main, "greet::greet();").expect("add stmt");
    (catalog, module, greet, main)
}

fn only_statement(catalog: &Catalog, owner: ElId) -> String {
    catalog.children_of(owner)[0].text.clone()
}

#[test]
fn renaming_a_function_leaves_the_module_qualifier_alone() {
    let (mut catalog, _, greet, main) = module_and_function_named_alike();
    catalog.rename(greet, "salute").expect("rename");

    rename_references(&mut catalog, greet, "greet");

    assert_eq!(only_statement(&catalog, main), "greet::salute();");
}

#[test]
fn renaming_a_module_rewrites_its_qualifier() {
    let (mut catalog, module, _, main) = module_and_function_named_alike();
    catalog.rename(module, "salute").expect("rename");

    rename_references(&mut catalog, module, "greet");

    assert_eq!(only_statement(&catalog, main), "salute::greet();");
}

#[test]
fn renaming_a_function_leaves_a_method_of_the_same_name_alone() {
    let (mut catalog, apply, caller) = with_caller("plan.apply();");
    catalog.rename(apply, "dispatch").expect("rename");

    rename_references(&mut catalog, apply, "apply");

    assert_eq!(only_statement(&catalog, caller), "plan.apply();");
}
