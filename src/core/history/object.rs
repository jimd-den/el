// A blob is one projection file's bytes, addressed by their digest
// (`HISTORY.md` §2). There is no header and no wrapper: the path *is* the digest, so
// dedupe is free and a file on disk is exactly the file the compiler will see.
//
// That makes verification the whole of this module's job. Every read re-hashes and
// compares (`FR-33`), because a store that trusts its own bytes turns a bad disk
// into a wrong restore — the one failure mode `el` cannot recover from, since the
// restore is what the user asked for.
//
// It deliberately does not authenticate or encrypt: the threat is corruption, not an
// adversary (`HISTORY.md` §3), and a MAC would need a secret, which is a different
// product.

use super::sha256;
use std::fmt;

/// A SHA-256 digest of a blob's bytes.
pub type Digest = [u8; 32];

/// The blob's digest, given the bytes to store.
pub fn of(bytes: &[u8]) -> Digest {
    sha256::digest(bytes)
}

/// The store-relative path a blob's digest implies: `objects/<ab>/<cdef…>`.
///
/// Two hex characters of prefix, like git, so a store with ten thousand objects has
/// hundreds of files per directory rather than ten thousand in one.
pub fn path(digest: &Digest) -> String {
    let hex = sha256::hex(digest);
    format!("objects/{}/{}", &hex[..2], &hex[2..])
}

/// The digest as it is written in a record or a path.
pub fn show(digest: &Digest) -> String {
    sha256::hex(digest)
}

/// The inverse of `show`; `None` when the text is not a digest.
pub fn parse(text: &str) -> Option<Digest> {
    sha256::unhex(text)
}

/// A blob whose bytes are not the bytes its digest promised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corrupt {
    pub path: String,
    pub expected: Digest,
    pub actual: Digest,
}

impl fmt::Display for Corrupt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "env.corrupt: {} holds {}, expected {}",
            self.path,
            show(&self.actual),
            show(&self.expected)
        )
    }
}

/// Whether `bytes` are the blob `expected` names.
///
/// Contract: `Ok(())` when the digests match, `Corrupt` naming both digests when they
/// do not. Callers must not use `bytes` on `Err` — the point of naming both is that
/// the caller has no basis left for a decision.
pub fn verify(path: &str, bytes: &[u8], expected: &Digest) -> Result<(), Corrupt> {
    let actual = of(bytes);
    if actual == *expected {
        Ok(())
    } else {
        Err(Corrupt {
            path: path.to_string(),
            expected: *expected,
            actual,
        })
    }
}

#[cfg(test)]
#[path = "object_tests.rs"]
mod object_tests;
