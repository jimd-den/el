// Baselines: the recorded diagnostic set of a tree, per tier, under
// `.el/diags/`. A file that exists records a set, even when the set is empty;
// a missing file means never recorded, and is computed once from the previous
// tree. A corrupt line is skipped: the cache fails soft, never hard.

use super::run::run_tier;
use super::Input;
use crate::app::error::{filesystem, Error};
use crate::app::Env;
use crate::core::gate::{decode_baseline, encode_baseline, Diagnostic, Tier};
use crate::core::history::object::{self, Digest};
use crate::core::render::render;
use super::super::projection;

fn baseline_path(digest: &Digest) -> String {
    format!(".el/diags/{}", object::show(digest))
}

/// Reads a recorded baseline, if one exists.
pub(super) fn load_baseline(
    env: &Env,
    digest: &Digest,
    tier: Tier,
) -> Result<Option<Vec<Diagnostic>>, Error> {
    let bytes = env.fs.read(&baseline_path(digest)).map_err(filesystem)?;
    Ok(bytes.map(|bytes| {
        let text = String::from_utf8_lossy(&bytes);
        decode_baseline(&text, tier.as_str())
    }))
}

/// The baseline for the previous tree: recorded, or computed once by running
/// the previous tree through the same tier. Computing restores the old tree,
/// runs, records, and re-materializes the new tree; any failure leaves the
/// caller to abort, which restores the old tree again.
pub(super) fn load_or_compute_baseline(input: &Input) -> Result<Vec<Diagnostic>, Error> {
    if let Some(baseline) = load_baseline(input.env, &input.old_digest, input.tier)? {
        return Ok(baseline);
    }
    if input.old_tree.is_empty() {
        return Ok(Vec::new());
    }
    let old_projection = render(input.old_catalog, &input.env.crate_name);
    projection::restore(input.env, input.old_tree)?;
    let baseline = run_tier(input.env, input.tier, input.old_catalog, &old_projection);
    projection::materialize(input.env, input.projection, input.old_tree, input.new_tree)?;
    let baseline = baseline?;
    write_baseline(input.env, input.old_digest, input.tier, &baseline)?;
    Ok(baseline)
}

/// Records a tree's diagnostic set for one tier, preserving the sets other
/// tiers recorded for the same tree. A write failure aborts the transaction.
pub(crate) fn write_baseline(
    env: &Env,
    digest: Digest,
    tier: Tier,
    diagnostics: &[Diagnostic],
) -> Result<(), Error> {
    let path = baseline_path(&digest);
    let mut text = String::new();
    if let Some(bytes) = env.fs.read(&path).map_err(filesystem)? {
        let old = String::from_utf8_lossy(&bytes);
        for line in old.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 11 && parts[0] == "el1" && parts[1] != tier.as_str() {
                text.push_str(line);
                text.push('\n');
            }
        }
    }
    text.push_str(&encode_baseline(tier.as_str(), diagnostics));
    env.fs.write(&path, text.as_bytes()).map_err(filesystem)?;
    Ok(())
}
