// The one write `init` makes: the adopted tree, its catalog, and the two journal
// entries that say what happened. The tree is built from the files that were just
// verified byte for byte, so the store names the bytes that are already there.

use super::super::create::POLICY;
use super::super::error::{self, Error};
use super::super::Env;
use crate::core::history::entry::{Entry, Op};
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;

pub(super) fn commit(env: &Env<'_>, catalog: &Catalog, files: &[String]) -> Result<usize, Error> {
    write_if_absent(env, "el.policy", POLICY.as_bytes())?;
    write_if_absent(env, ".gitignore", b".el/\ntarget/\ntmp/\n")?;
    env.journal.stamp_version().map_err(error::store)?;

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

    let mut entries = Vec::with_capacity(files.len());
    for path in files {
        let bytes = env.fs.read(path).map_err(error::filesystem)?.unwrap_or_default();
        let digest = env.journal.put_blob(&bytes).map_err(error::store)?;
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
        msg: format!("adopted {} files as opaque modules", files.len()),
    };
    let text = catalog.encode();
    env.journal.set_catalog(&text).map_err(error::store)?;
    env.journal.put_catalog(2, &text).map_err(error::store)?;
    env.journal.put_entry(&entry).map_err(error::store)?;
    env.journal.set_head(2).map_err(error::store)?;
    Ok(files.len())
}

fn write_if_absent(env: &Env<'_>, path: &str, bytes: &[u8]) -> Result<(), Error> {
    if env.fs.exists(path).unwrap_or(false) {
        return Ok(());
    }
    env.fs.write(path, bytes).map_err(error::filesystem)
}
