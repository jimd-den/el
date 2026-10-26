// Reading the three things every mutating command needs: the crate's policy, the
// catalog, and the tree `head` names. Each is a value or a named failure; none of
// them writes.
//
// The catalog is a cache of the journal (ADR-0002), so a catalog that does not
// decode is `env.corrupt`, never a silent empty model. An empty catalog is only
// legal when `head` is absent, which is a crate `el new` is still creating.

use super::error;
use super::report::Error;
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::plan::policy::Policy;

pub fn ensure_crate(env: &super::Env<'_>) -> Result<(), Error> {
    match env.journal.version().map_err(error::store)? {
        Some(version) if version == "el1" => Ok(()),
        Some(_) => Err(error::corrupt("the store's format version is not el1")),
        None => Err(error::not_a_crate(&env.root)),
    }
}

pub fn catalog(env: &super::Env<'_>) -> Result<Catalog, Error> {
    match env.journal.catalog().map_err(error::store)? {
        Some(text) => Catalog::decode(&text)
            .ok_or_else(|| error::corrupt("the catalog did not decode; run `el doctor --rebuild`")),
        None => Ok(Catalog::new()),
    }
}

pub fn policy(env: &super::Env<'_>) -> Result<Policy, Error> {
    let bytes = env.fs.read("el.policy").map_err(error::filesystem)?;
    let Some(bytes) = bytes else {
        return Ok(Policy::default());
    };
    let text = String::from_utf8(bytes)
        .map_err(|_| error::corrupt("el.policy is not UTF-8"))?;
    Policy::parse(&text).map_err(|failure| Error {
        code: "env.bad_policy",
        exit: super::Exit::Env,
        message: failure.to_string(),
    })
}

/// The sequence number `head` names, and the tree it committed. `(0, empty)`
/// when there is no head yet.
pub fn head_tree(env: &super::Env<'_>) -> Result<(u64, Tree), Error> {
    let Some(seq) = env.journal.head().map_err(error::store)? else {
        return Ok((0, Tree::new(Vec::new()).expect("an empty tree")));
    };
    let entry = env
        .journal
        .entry(seq)
        .map_err(error::store)?
        .ok_or_else(|| error::corrupt(format!("head names seq {seq}, but the entry is missing")))?;
    let tree = env.journal.get_tree(&entry.to).map_err(error::store)?;
    Ok((seq, tree))
}
