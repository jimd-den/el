// The read surface: `ls`, `tree`, `show` and `find`. It is pure over a catalog, a
// projection and the journal, so the same function answers a human and `--json`;
// the only difference is which half of the returned `View` the edge prints.
//
// It deliberately does not load anything or decide when a tree is clean. Those
// are `verbs::inspect`'s questions, and keeping them there is what makes these
// functions testable without a filesystem.

mod explain;
mod find;
mod ls;
mod show;
mod tree;

pub use explain::explain;
pub use find::find;
pub use ls::{Filter, ls};
pub use show::{Options, show, statement};
pub use tree::tree;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[cfg(test)]
#[path = "find_tests.rs"]
mod find_tests;
#[cfg(test)]
#[path = "show_tests.rs"]
mod show_tests;
#[cfg(test)]
#[path = "show_terse_tests.rs"]
mod show_terse_tests;

use crate::core::history::sha256;
use crate::core::model::element::Element;
use crate::core::model::kind::Kind;
use crate::core::render::Projection;
use crate::core::text::json::Json;

/// One read verb's answer in both shapes. `lines` is the human view; `payload`
/// is the `data` field of the JSON envelope. They are built from one pass so the
/// two cannot disagree about what exists.
pub struct View {
    pub lines: Vec<String>,
    pub payload: Json,
}

/// The committed source slice for an element: a module is its whole file,
/// an item is its span's lines. `show --source` and the revision digest
/// share this, so a digest can never drift from what `read` returns.
pub fn source_of(projection: &Projection, element: &Element) -> Option<String> {
    if element.kind == Kind::Module {
        return projection.file(&element.module).map(|file| file.text.clone());
    }
    let span = projection
        .spans
        .iter()
        .find(|span| span.element == element.id)?;
    let file = projection.file(&span.path)?;
    let lines: Vec<&str> = file.text.lines().collect();
    let start = span.first.saturating_sub(1) as usize;
    let end = span.last as usize;
    lines.get(start..end).map(|slice| slice.join("\n"))
}

/// The revision of an element: the digest of its committed source slice.
/// A body edit changes the span, so it changes the digest; an element
/// with no span (a module with no file) digests to the empty string.
pub fn rev_of(projection: &Projection, element: &Element) -> String {
    let source = source_of(projection, element).unwrap_or_default();
    format!("sha256:{}", sha256::hex(&sha256::digest(source.as_bytes())))
}
