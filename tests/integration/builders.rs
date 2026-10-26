// The catalogs FR-14 compiles: five shapes, chosen so that between them they
// exercise every layout decision the renderer makes — a brace or a semicolon, a
// `mod` line, a module that exists only because its child does, a nested block,
// and a body kept opaque.
//
// They are builder functions rather than fixtures because the model is a
// catalog, not a file: there is nothing on disk to read them from.

use crate::helpers::*;
use el::core::model::catalog::Catalog;
use el::core::model::element::{BodyMode, Visibility};
use el::core::model::kind::Kind;

pub fn two_modules() -> Catalog {
    let mut catalog = Catalog::new();
    let main = add(&mut catalog, Kind::Function, "src/main.rs", "fn main()");
    stmts(&mut catalog, main, &["greet::greet();\n"]);
    let greet = add_vis(&mut catalog, Kind::Function, "src/greet.rs", "fn greet()", Visibility::Pub);
    stmts(&mut catalog, greet, &["println!(\"hi\");\n"]);

    catalog
}

pub fn a_struct_and_its_impl() -> Catalog {
    let mut catalog = Catalog::new();
    let main = add(&mut catalog, Kind::Function, "src/main.rs", "fn main()");
    stmts(
        &mut catalog,
        main,
        &[
            "let p = geom::Point::point(1, 2);\n",
            "println!(\"{}\", p.sum());\n",
        ],
    );
    // A field is `x: i32` or `pub x: i32` as its statement text says: `Visibility`
    // belongs to the item, and `el` does not model field visibility. These are
    // private, so the constructor below is the only door another module has.
    let point = add_vis(&mut catalog, Kind::Struct, "src/geom.rs", "struct Point", Visibility::Pub);
    stmts(&mut catalog, point, &["x: i32,\n", "y: i32,\n"]);
    let implementation = add(&mut catalog, Kind::Impl, "src/geom.rs", "impl Point");
    block(
        &mut catalog,
        implementation,
        "pub fn point(x: i32, y: i32) -> Point {\n",
        &["Point { x, y }\n"],
    );
    block(
        &mut catalog,
        implementation,
        "pub fn sum(&self) -> i32 {\n",
        &["if self.x > 0 {\n", "return self.x;\n"],
    );

    catalog
}

pub fn every_leaf_kind() -> Catalog {
    let mut catalog = Catalog::new();
    add(&mut catalog, Kind::Const, "src/main.rs", "const LIMIT: usize = 10");
    add(&mut catalog, Kind::Static, "src/main.rs", "static NAME: &str = \"el\"");
    add(&mut catalog, Kind::TypeAlias, "src/main.rs", "type Id = u32");
    let color = add(&mut catalog, Kind::Enum, "src/main.rs", "enum Color");
    stmts(&mut catalog, color, &["Red,\n", "Blue,\n"]);
    let main = add(&mut catalog, Kind::Function, "src/main.rs", "fn main()");
    stmts(
        &mut catalog,
        main,
        &[
            "let _id: Id = LIMIT as Id;\n",
            "println!(\"{} {}\", NAME, _id);\n",
            "let _ = Color::Red;\n",
        ],
    );

    catalog
}

pub fn a_nested_module() -> Catalog {
    let mut catalog = Catalog::new();
    let main = add(&mut catalog, Kind::Function, "src/main.rs", "fn main()");
    stmts(
        &mut catalog,
        main,
        &["println!(\"{}\", crate::core::plan::apply(1));\n"],
    );
    let apply = add_vis(
        &mut catalog,
        Kind::Function,
        "src/core/plan.rs",
        "fn apply(n: u8) -> u8",
        Visibility::Pub,
    );
    stmts(&mut catalog, apply, &["n + 1\n"]);

    catalog
}

pub fn an_opaque_body() -> Catalog {
    let mut catalog = Catalog::new();
    let main = add(&mut catalog, Kind::Function, "src/main.rs", "fn main()");
    stmts(
        &mut catalog,
        main,
        &["let s = r#\"a\nb\"#;\nprintln!(\"{}\", s);\n"],
    );
    if let Some(element) = catalog.find_mut(main) {
        element.mode = BodyMode::Opaque;
    }

    catalog
}
