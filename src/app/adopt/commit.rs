// The one write `init` makes: the adopted tree, its catalog, and the two journal
// entries that say what happened. The tree is built from the files that were just
// verified byte for byte, so the store names the bytes that are already there.

use super::super::create::POLICY;
use super::super::error::{self, Error};
use super::super::Env;
use crate::core::history::entry::{Entry, Op};
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;

pub(super) fn commit(
    env: &Env<'_>,
    catalog: &Catalog,
    texts: &[(String, String)],
) -> Result<usize, Error> {
    write_if_absent(env, "el.policy", POLICY.as_bytes())?;
    ensure_gitignore(env)?;

    let empty = Tree::new(Vec::new()).expect("an empty tree");
    let empty_digest = env.journal.put_tree(&empty).map_err(error::store)?;
    let init = Entry {
        seq: 1,
        ts: env.clock.now_unix(),
        actor: env.actor.clone(),
        op: Op::Init,
        target: "-".to_string(),
        from: empty_digest,
        to: empty_digest,
        gate: "model".to_string(),
        msg: format!("created {}", env.crate_name),
    };
    env.journal.put_entry(&init).map_err(error::store)?;
    env.journal.set_head(1).map_err(error::store)?;
    let empty_catalog = Catalog::new().encode();
    env.journal.set_catalog(&empty_catalog).map_err(error::store)?;
    env.journal.put_catalog(1, &empty_catalog).map_err(error::store)?;

    let mut entries = Vec::with_capacity(texts.len());
    for (path, text) in texts {
        let digest = env.journal.put_blob(text.as_bytes()).map_err(error::store)?;
        entries.push((path.clone(), digest));
    }
    let tree = Tree::new(entries)
        .map_err(|duplicate| error::corrupt(duplicate.to_string()))?;
    let digest = env.journal.put_tree(&tree).map_err(error::store)?;
    let entry = Entry {
        seq: 2,
        ts: env.clock.now_unix(),
        actor: env.actor.clone(),
        op: Op::Import,
        target: "src".to_string(),
        from: empty_digest,
        to: digest,
        gate: "skip".to_string(),
        msg: format!("adopted {} files as opaque modules", texts.len()),
    };
    let text = catalog.encode();
    env.journal.set_catalog(&text).map_err(error::store)?;
    env.journal.put_catalog(2, &text).map_err(error::store)?;
    env.journal.put_entry(&entry).map_err(error::store)?;
    env.journal.set_head(2).map_err(error::store)?;
    // Last: a failure above leaves no version stamp, so `init` can be retried.
    env.journal.stamp_version().map_err(error::store)?;
    Ok(texts.len())
}

fn ensure_gitignore(env: &Env<'_>) -> Result<(), Error> {
    let existing = env.fs.read(".gitignore").map_err(error::filesystem)?;
    let mut text = match existing {
        None => ".el/\ntarget/\ntmp/\n".to_string(),
        Some(bytes) => String::from_utf8(bytes)
            .map_err(|_| error::corrupt(".gitignore is not UTF-8"))?,
    };
    if !text.lines().any(|line| line.trim() == ".el/" || line.trim() == ".el") {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(".el/\n");
    }
    env.fs.write(".gitignore", text.as_bytes()).map_err(error::filesystem)
}

fn write_if_absent(env: &Env<'_>, path: &str, bytes: &[u8]) -> Result<(), Error> {
    if env.fs.exists(path).unwrap_or(false) {
        return Ok(());
    }
    env.fs.write(path, bytes).map_err(error::filesystem)
}
