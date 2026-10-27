// The loading half of the read verbs: load the catalog and hand the pure
// `inspect` functions a projection. They never stage a transaction, never write,
// and never repair — recovery needs the writer lock, and a read holding it would
// refuse to run during the transaction it is reading about. `doctor --repair` and
// the next mutating command own that; see ADR-0020.
//
// They follow `log`, not `run`, on drift: these verbs answer from the catalog, so
// a hand edit on disk does not change the answer. `show --source` renders the
// committed model, not the drifted file; that is the read-only contract ADR-0020
// records.

use super::super::{change, error, inspect, load, Env, Exit, Report};
use crate::core::model::catalog::Catalog;
use crate::core::model::kind::Kind;
use crate::core::render::{render, Projection};

pub fn ls(env: &Env<'_>, module: Option<&str>, kind: Option<Kind>, opaque: bool) -> Report {
    let (catalog, projection) = match prepare(env) {
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
    report("ls", view)
}

pub fn tree(env: &Env<'_>, statements: bool) -> Report {
    let (catalog, _) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tree", error),
    };
    report("tree", inspect::tree(&catalog, &env.crate_name, statements))
}

pub fn find(env: &Env<'_>, query: &str) -> Report {
    let (catalog, _) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("find", error),
    };
    let view = inspect::find(&catalog, query);
    if view.lines.is_empty() {
        return Report::fail(
            "find",
            error::Error {
                code: "notfound.element",
                exit: Exit::NotFound,
                message: format!("{query} does not match any element"),
            },
        );
    }
    report("find", view)
}

pub fn explain(env: &Env<'_>, id: &str) -> Report {
    let id = match change::parse_id(id) {
        Ok(id) => id,
        Err(error) => return Report::fail("explain", error),
    };
    let (catalog, _) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("explain", error),
    };
    let entries = match env.journal.entries() {
        Ok(entries) => entries,
        Err(error) => return Report::fail("explain", error::store(error)),
    };
    let element = match catalog.find(id) {
        Some(element) => element,
        None => match catalog.find_stmt(id) {
            Some(stmt) => match catalog.find(stmt.owner) {
                Some(owner) => owner,
                None => return Report::fail("explain", missing(id)),
            },
            None => return Report::fail("explain", missing(id)),
        },
    };
    report("explain", crate::app::inspect::explain(&catalog, &entries, element))
}

pub fn show(
    env: &Env<'_>,
    id: &str,
    statements: bool,
    source: bool,
    history: bool,
    terse: bool,
) -> Report {
    let id = match change::parse_id(id) {
        Ok(id) => id,
        Err(error) => return Report::fail("show", error),
    };
    let (catalog, projection) = match prepare(env) {
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
        terse,
    };
    let view = match catalog.find(id) {
        Some(element) => inspect::show(&catalog, &projection, &entries, element, &options),
        None => match catalog.find_stmt(id) {
            Some(stmt) => inspect::statement(&catalog, &projection, &entries, stmt, &options),
            None => return Report::fail("show", missing(id)),
        },
    };
    report("show", view)
}

fn report(cmd: &'static str, view: inspect::View) -> Report {
    let mut report = Report::new(cmd);
    report.lines = view.lines;
    report.payload = Some(view.payload);
    report
}

fn prepare(env: &Env<'_>) -> Result<(Catalog, Projection), error::Error> {
    load::ensure_crate(env)?;
    let catalog = load::catalog(env)?;
    let projection = render(&catalog, &env.crate_name);
    Ok((catalog, projection))
}

fn missing(id: crate::core::model::id::ElId) -> error::Error {
    error::Error {
        code: "notfound.element",
        exit: Exit::NotFound,
        message: format!("{id} does not exist"),
    }
}
