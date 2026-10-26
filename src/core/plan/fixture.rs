// A factory, not a fixture: every test states the overrides it needs and nothing
// is shared between them. The one default worth naming is `require_preamble =
// false`, because most rules are easier to test in isolation when the preamble
// rule is not also shouting; the test that exercises the preamble rule turns it
// back on.

use crate::core::model::catalog::Catalog;
use crate::core::model::element::{NewElement, Visibility};
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;
use crate::core::plan::policy::Policy;
use crate::core::render::Projection;

pub fn policy() -> Policy {
    Policy {
        require_preamble: false,
        ..Policy::default()
    }
}

pub fn catalog() -> Catalog {
    Catalog::new()
}

/// A module element at `path`, carrying a one-line preamble.
pub fn module(catalog: &mut Catalog, path: &str) -> ElId {
    let name = path.trim_start_matches("src/").trim_end_matches(".rs");
    let id = catalog
        .add_element(&NewElement {
            kind: Kind::Module,
            module: path,
            signature: &format!("mod {name}"),
            visibility: Visibility::Private,
        })
        .expect("module");
    catalog
        .add_stmt(id, "// this module exists because the test says so\n")
        .expect("preamble");
    id
}

pub fn function(catalog: &mut Catalog, path: &str, signature: &str) -> ElId {
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: path,
            signature,
            visibility: Visibility::Private,
        })
        .expect("function")
}

pub fn item(catalog: &mut Catalog, kind: Kind, path: &str, signature: &str) -> ElId {
    catalog
        .add_element(&NewElement {
            kind,
            module: path,
            signature,
            visibility: Visibility::Private,
        })
        .expect("item")
}

pub fn projection(catalog: &Catalog) -> Projection {
    crate::core::render::render(catalog, "demo")
}
