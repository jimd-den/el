// FR-14, one test per shape of catalog: the rendered crate is valid Rust.
//
// The shapes are chosen so that between them they exercise every layout
// decision the renderer makes — a brace or a semicolon, a `mod` line, a module
// that exists only because its child does, a nested block, a body kept opaque.
//
// The catalogs live in `builders` and the compile harness in `helpers`: this file
// is the list of claims, and those two are what makes a claim checkable.

use crate::builders::{
    a_nested_module, a_struct_and_its_impl, an_opaque_body, every_leaf_kind, two_modules,
};
use crate::helpers::assert_compiles;

#[test]
fn a_two_module_crate_compiles() {
    assert_compiles("two_modules", &two_modules());
}

#[test]
fn a_crate_with_a_struct_and_an_impl_compiles() {
    assert_compiles("struct_and_impl", &a_struct_and_its_impl());
}

#[test]
fn a_crate_with_every_leaf_kind_compiles() {
    assert_compiles("leaf_kinds", &every_leaf_kind());
}

#[test]
fn a_crate_with_a_nested_module_compiles() {
    assert_compiles("nested", &a_nested_module());
}

#[test]
fn a_crate_with_an_opaque_body_compiles() {
    assert_compiles("opaque", &an_opaque_body());
}
