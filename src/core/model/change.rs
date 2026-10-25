use super::id::ElId;
use super::kind::Kind;

#[derive(Debug, Clone)]
pub enum Change {
    Add { kind: Kind, name: Option<String>, parent: Option<ElId> },
    Remove { id: ElId },
    Rename { id: ElId, name: String },
}
