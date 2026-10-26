// The vocabulary of mutation. A change says what the user asked for, in the
// model's words, and nothing about how it will be carried out: no offsets, no
// files, no ordering. That is what lets `plan` be a pure function — the same
// change against the same catalog always means the same thing (FR-18).
//
// Ownership is explicit: every field is owned (`String`, not `&str`), because a
// change outlives the command line that produced it and gets recorded in history
// (M8). A borrowed form would tie a record's lifetime to the argv it came from.
//
// It deliberately does not carry an id. Ids are issued by the catalog's counter,
// never chosen by a caller, or "never reused" (FR-1) would be a promise the
// caller could break.

use super::element::Visibility;
use super::id::ElId;
use super::kind::Kind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A new item in `module`, its head written keyword-first.
    AddElement {
        kind: Kind,
        module: String,
        signature: String,
        visibility: Visibility,
    },
    /// Replace an item's head; the name and the body are untouched.
    EditElement { id: ElId, head: String },
    /// Move an item to another module, keeping its id and its body.
    MoveElement { id: ElId, module: String },
    /// Change an item's name. References are rewritten by `plan::refs`.
    Rename { id: ElId, name: String },
    /// Append a statement to an element's body.
    AddStmt { owner: ElId, text: String },
    /// Replace one statement's text.
    EditStmt { id: ElId, text: String },
    /// Reorder or re-parent a statement. `to` is an element or a statement; a
    /// target in another body is refused (`model.cross_body_move`).
    MoveStmt { id: ElId, to: ElId },
    /// Remove an element (with its body) or a statement (with its children).
    Remove { id: ElId },
    /// Add a dependency, which `el.policy` may deny (`model.dep_denied`).
    AddDep { name: String },
}
