// The filesystem port: bytes in, bytes out, one directory that `el` owns.
//
// Paths are store-relative (`objects/ab/cdef…`), never absolute, and an
// implementation must refuse to resolve one outside the store's root. That is a
// contract rather than a convention because a journal entry, a tree and a path
// all come from disk, so a store path is untrusted input like any other.
//
// The one behaviour with a crash story is `write`. Its contract is the whole
// reason this port exists rather than a `std::fs` call at each site: an
// interrupted write leaves the previous bytes, never half of the new ones
// (`HISTORY.md` §7, invariant I3).
//
// It deliberately does not model directories, modes, symlinks or timestamps. A
// projection is generated source plus a manifest; an implementation that grew a
// mode bit here would be answering a question the catalog cannot ask.

use std::fmt;
use std::io;

/// Why a filesystem call failed, as plain data that crosses the boundary.
///
/// `std::io::ErrorKind` is deliberately narrowed to the cases a caller can act
/// on: the port does not hand out an OS error object, because that would put
/// platform vocabulary in the decision layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The path is not there. Only `read` and `list` can say this; `remove` of an
    /// absent path is success, because a path that is gone is already removed.
    Missing,
    Denied,
    /// A path component is not a directory, or the path itself is one.
    NotAFile,
    Other,
}

impl Kind {
    /// The port's word for a raw OS error kind.
    pub fn from_io(kind: io::ErrorKind) -> Self {
        match kind {
            io::ErrorKind::NotFound => Kind::Missing,
            io::ErrorKind::PermissionDenied => Kind::Denied,
            io::ErrorKind::IsADirectory | io::ErrorKind::NotADirectory => Kind::NotAFile,
            _ => Kind::Other,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Kind::Missing => "not found",
            Kind::Denied => "permission denied",
            Kind::NotAFile => "not a file",
            Kind::Other => "i/o error",
        };
        f.write_str(word)
    }
}

/// A failed call, at the path it failed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub path: String,
    pub kind: Kind,
}

impl Error {
    pub fn new(path: impl Into<String>, kind: Kind) -> Self {
        Self {
            path: path.into(),
            kind,
        }
    }
}

impl fmt::Display for Error {
    /// A message fragment, not a line: the edge adds the `el: ` prefix once.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

/// A store's worth of files, addressed by relative path.
pub trait Fs {
    /// The bytes at `path`, or `None` when there are none.
    ///
    /// Contract: `Ok(None)` is an absent file and is not an error — the store's
    /// first run has no `objects/` at all, and that is not a failure.
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Error>;

    /// Replace the bytes at `path`, creating parent directories as needed.
    ///
    /// Contract: crash-atomic. After an interruption at any point the caller
    /// reads either the previous bytes or the new ones. Partial content is not a
    /// reachable state.
    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), Error>;

    /// Remove `path`. Absent is success.
    fn remove(&self, path: &str) -> Result<(), Error>;

    /// Every file under `prefix`, store-relative and sorted.
    ///
    /// Sorted because two runs over the same store must produce the same answer:
    /// a caller that reports "these are your objects" in `readdir` order is a
    /// caller whose output depends on the filesystem.
    fn list(&self, prefix: &str) -> Result<Vec<String>, Error>;

    /// Whether `path` holds a file.
    fn exists(&self, path: &str) -> Result<bool, Error>;

    /// The bytes of a file the *user* named, e.g. `--body /tmp/snippet.rs`.
    ///
    /// Contract: like `read`, but a path outside the crate is allowed. The store
    /// addresses itself by crate-relative path and refuses to escape it; a source
    /// the user typed is a different question, and the projection is still only
    /// written inside the crate. `Ok(None)` is an absent file, as for `read`.
    fn read_source(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        self.read(path)
    }
}
