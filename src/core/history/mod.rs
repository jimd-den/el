// The store: content-addressed snapshots of the projection, and the journal that
// says how it got here. Everything reversible in `el` rests on this being right
// after a crash, so the pieces are small and the formats are written down
// (`docs/HISTORY.md` §4).
//
// This package is pure. It knows how to frame a blob, encode a tree, encode a
// journal entry, and verify a digest; it does not know what a file is. Reading and
// writing belong to `adapters::fs_local` behind `ports::fs`, which is also what
// makes the store testable without a disk.
//
// Invariants it exists to keep: a blob's path is its digest (`FR-33`), a tree is
// sorted by path, and a record is always one line (`SRD.md` §10).

pub mod entry;
pub mod object;
pub mod sha256;
pub mod tree;

pub use entry::Entry;
pub use object::Digest;
pub use tree::Tree;
