// Turning a file path into a module name, and back. Cargo's rule, and the only
// place that knows it: `src/main.rs` and `src/lib.rs` are the root, `src/a.rs` is
// `a`, `src/a/b.rs` is `a::b`.
//
// Kept apart from the tree itself because it is the part that is pure string
// arithmetic with no catalog in sight, and because getting it wrong is a
// specific, testable kind of wrong.

use crate::core::model::catalog::Catalog;
use crate::core::model::kind::Kind;
use std::collections::BTreeSet;

/// `src/core/plan.rs` → `core::plan`; `src/main.rs` → ``; `src/a/mod.rs` → `a`.
pub fn module_of(path: &str) -> String {
    let trimmed = path.strip_prefix("src/").unwrap_or(path);
    let trimmed = trimmed.strip_suffix(".rs").unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix("/mod").unwrap_or(trimmed);
    match trimmed {
        "" | "main" | "lib" => String::new(),
        other => other.replace('/', "::"),
    }
}

/// Every module that must exist for `module` to exist, itself included.
pub fn ancestors(module: &str) -> Vec<String> {
    let mut chain = vec![module.to_string()];
    let mut current = module.to_string();
    while !current.is_empty() {
        current = parent_of(&current);
        if current.is_empty() {
            break;
        }
        chain.push(current.clone());
    }
    chain
}

/// The file a child module lives in: the root's children sit in `src/`, and
/// everything else nests under its parent's name (`src/a.rs` → `src/a/b.rs`).
pub fn child_file(parent: &str, child: &str) -> String {
    match module_of(parent).is_empty() {
        true => format!("src/{child}.rs"),
        false => format!("{}/{child}.rs", parent.strip_suffix(".rs").unwrap_or(parent)),
    }
}

/// The file a module lives in when it is created: `a::b` → `src/a/b.rs`.
pub fn file_for(module: &str) -> String {
    match module {
        "" => "src/main.rs".to_string(),
        other => format!("src/{}.rs", other.replace("::", "/")),
    }
}

pub fn parent_of(module: &str) -> String {
    match module.rsplit_once("::") {
        Some((parent, _)) => parent.to_string(),
        None => String::new(),
    }
}

pub fn leaf(module: &str) -> &str {
    module.rsplit("::").next().unwrap_or(module)
}

/// Every file path the catalog mentions, for a non-statement element.
pub fn mentioned(catalog: &Catalog) -> BTreeSet<String> {
    catalog
        .elements
        .iter()
        .filter(|element| element.kind != Kind::Stmt)
        .map(|element| element.module.clone())
        .collect()
}
