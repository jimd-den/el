// The read surface: `ls`, `tree` and `show`. It is pure over a catalog, a
// projection and the journal, so the same function answers a human and `--json`;
// the only difference is which half of the returned `View` the edge prints.
//
// It deliberately does not load anything or decide when a tree is clean. Those
// are `verbs::inspect`'s questions, and keeping them there is what makes these
// functions testable without a filesystem.

mod ls;
mod show;
mod tree;

pub use ls::{Filter, ls};
pub use show::{Options, show, statement};
pub use tree::tree;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[cfg(test)]
#[path = "show_tests.rs"]
mod show_tests;

use crate::core::text::json::Json;

/// One read verb's answer in both shapes. `lines` is the human view; `payload`
/// is the `data` field of the JSON envelope. They are built from one pass so the
/// two cannot disagree about what exists.
pub struct View {
    pub lines: Vec<String>,
    pub payload: Json,
}
