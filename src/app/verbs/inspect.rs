// The loading half of the read verbs: recover an interrupted transaction, load
// the catalog, and hand the pure `inspect` functions a projection. They never
// stage a transaction and never write.
//
// They follow `log`, not `run`, on drift: these verbs answer from the catalog, so
// a hand edit on disk does not change the answer. `show --source` renders the
// committed model, not the drifted file; that is the read-only contract ADR-0020
// records.

use super::super::{change, error, inspect, load, transaction, Env, Exit, Report};
use crate::core::model::catalog::Catalog;
use crate::core::model::kind::Kind;
use crate::core::render::{render, Projection};

pub fn ls(env: &Env<'_>, module: Option<&str>, kind: Option<Kind>, opaque: bool) -> Report {
    let (catalog, projection, notes) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("ls", error),
    };
    let view = inspect::ls(
        &catalog,
        &projection,
        &inspect::Filter {
            module: module.map(str::to_string),
            kind,
            opaque,
        },
    );
    report("ls", view, notes)
}

pub fn tree(env: &Env<'_>, statements: bool) -> Report {
    let (catalog, _, notes) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tree", error),
    };
    report("tree", inspect::tree(&catalog, &env.crate_name, statements), notes)
}

pub fn show(
    env: &Env<'_>,
    id: &str,
    statements: bool,
    source: bool,
    history: bool,
) -> Report {
    let id = match change::parse_id(id) {
        Ok(id) => id,
        Err(error) => return Report::fail("show", error),
    };
    let (catalog, projection, notes) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("show", error),
    };
    let entries = match env.journal.entries() {
        Ok(entries) => entries,
        Err(error) => return Report::fail("show", error::store(error)),
    };
    let options = inspect::Options {
        statements,
        source,
        history,
    };
    let view = match catalog.find(id) {
        Some(element) => inspect::show(&catalog, &projection, &entries, element, &options),
        None => match catalog.find_stmt(id) {
            Some(stmt) => inspect::statement(&catalog, &projection, &entries, stmt, &options),
            None => return Report::fail("show", missing(id)),
        },
    };
    report("show", view, notes)
}

fn report(cmd: &'static str, view: inspect::View, notes: Vec<String>) -> Report {
    let mut report = Report::new(cmd);
    report.lines = view.lines;
    report.payload = Some(view.payload);
    report.notes = notes;
    report
}

fn prepare(env: &Env<'_>) -> Result<(Catalog, Projection, Vec<String>), error::Error> {
    load::ensure_crate(env)?;
    let notes = transaction::recover(env)?;
    let catalog = load::catalog(env)?;
    let projection = render(&catalog, &env.crate_name);
    Ok((catalog, projection, notes))
}

fn missing(id: crate::core::model::id::ElId) -> error::Error {
    error::Error {
        code: "notfound.element",
        exit: Exit::NotFound,
        message: format!("{id} does not exist"),
    }
}
