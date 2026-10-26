// Rendering: the catalog becomes the files the compiler sees, plus a span map
// that says which element owns each line (FR-13, FR-17). This is the only place
// where a model becomes Rust text.
//
// Shape of the solution: one pure function `render(catalog, crate_name)`, whose
// output is a `Projection` — a sorted list of files and a sorted list of spans.
// Ordering is part of the contract, not an accident of iteration: two catalogs
// with the same content must produce the same bytes (FR-13), so every list is
// sorted by a key the content determines (path, then id).
//
// It deliberately does not write anything: the caller decides what to do with a
// projection, which is what makes `--dry-run` cheap and rollback possible. It
// also does not parse Rust — heads are text the model already holds.

pub mod manifest;
pub mod rust;
pub mod spans;
pub mod tree;

mod body;
mod style;

use crate::core::model::catalog::Catalog;

pub use spans::Span;

/// One file as it should appear on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub path: String,
    pub text: String,
}

/// The whole projection: every file, sorted by path, with spans sorted by
/// (path, first line, element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub files: Vec<File>,
    pub spans: Vec<Span>,
}

impl Projection {
    /// The id of the innermost element owning `line` in `path`.
    ///
    /// Innermost wins because statements nest inside items and items inside
    /// modules: a diagnostic on a statement line must name the statement, not
    /// its function (FR-22, FR-68).
    pub fn element_at(&self, path: &str, line: u32) -> Option<crate::core::model::id::ElId> {
        spans::innermost(&self.spans, path, line)
    }

    pub fn file(&self, path: &str) -> Option<&File> {
        self.files.iter().find(|file| file.path == path)
    }
}

/// Render `catalog` as the Rust crate named `crate_name`.
///
/// Contract:
///   - Pure: no filesystem, no clock, no environment.
///   - Deterministic: equal content renders to equal bytes, whatever order the
///     catalog's vectors happen to be in (FR-13).
///   - The package name is an argument because it belongs to the project, not to
///     the model; `el.policy` and the project directory are both outside `core`.
pub fn render(catalog: &Catalog, crate_name: &str) -> Projection {
    let tree = tree::Tree::of(catalog);
    let mut spans = spans::Collector::new();
    let mut files = Vec::new();

    for module in tree.modules() {
        files.push(File {
            path: module.path.clone(),
            text: rust::render_module(catalog, &tree, module, &mut spans),
        });
    }
    files.push(File {
        path: "Cargo.toml".to_string(),
        text: manifest::render(crate_name),
    });
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Projection {
        files,
        spans: spans.finish(),
    }
}
