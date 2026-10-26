// `el doctor`: the two explicit recovery operations. `--repair` performs the same
// inflight rollback every command does, but says what it found; `--rebuild`
// rewrites `.el/catalog` from the head's snapshot and refuses to pretend when
// there is no snapshot to rebuild from.

use super::super::{error, load, transaction, Env, Report};

pub fn doctor(env: &Env<'_>, repair: bool, rebuild: bool) -> Report {
    if repair {
        return run_repair(env);
    }
    if rebuild {
        return run_rebuild(env);
    }
    health(env)
}

fn run_repair(env: &Env<'_>) -> Report {
    if let Err(error) = load::ensure_crate(env) {
        return Report::fail("doctor", error);
    }
    let mut report = Report::new("doctor");
    match transaction::recover(env) {
        Ok(notes) if notes.is_empty() => report.lines.push("no interrupted transaction".to_string()),
        Ok(notes) => report.lines.extend(notes),
        Err(error) => return Report::fail("doctor", error),
    }
    report
}

fn run_rebuild(env: &Env<'_>) -> Report {
    if let Err(error) = load::ensure_crate(env) {
        return Report::fail("doctor", error);
    }
    let head = match env.journal.head().map_err(error::store) {
        Ok(Some(head)) => head,
        Ok(None) => return Report::fail("doctor", missing("the crate has no history yet")),
        Err(error) => return Report::fail("doctor", error),
    };
    let snapshot = match env.journal.catalog_at(head).map_err(error::store) {
        Ok(Some(text)) => text,
        Ok(None) => {
            return Report::fail(
                "doctor",
                error::corrupt(format!("catalog snapshot for seq {head} is missing")),
            );
        }
        Err(error) => return Report::fail("doctor", error),
    };
    let current = env.journal.catalog().map_err(error::store);
    match current {
        Ok(Some(text)) if text == snapshot => {
            let mut report = Report::new("doctor");
            report.lines.push(format!("catalog for seq {head} is already identical"));
            report
        }
        _ => match env.journal.set_catalog(&snapshot).map_err(error::store) {
            Ok(()) => {
                let mut report = Report::new("doctor");
                report.lines.push(format!("rebuilt catalog from seq {head}"));
                report
            }
            Err(error) => Report::fail("doctor", error),
        },
    }
}

fn health(env: &Env<'_>) -> Report {
    let mut report = Report::new("doctor");
    report.lines.push(format!("root        {}", env.root));
    match env.journal.version() {
        Ok(version) => report.lines.push(format!("version     {}", version.unwrap_or_else(|| "-".to_string()))),
        Err(error) => report.lines.push(format!("version     error: {error}")),
    }
    match env.journal.head() {
        Ok(head) => report.lines.push(format!("head        {}", head.map_or("-".to_string(), |h| h.to_string()))),
        Err(error) => report.lines.push(format!("head        error: {error}")),
    }
    match env.journal.entries() {
        Ok(entries) => report.lines.push(format!("journal     {} entries", entries.len())),
        Err(error) => report.lines.push(format!("journal     error: {error}")),
    }
    match env.journal.attic_entries() {
        Ok(entries) => report.lines.push(format!("attic       {} entries", entries.len())),
        Err(error) => report.lines.push(format!("attic       error: {error}")),
    }
    report
}

fn missing(message: &str) -> error::Error {
    error::Error {
        code: "notfound.sequence",
        exit: crate::app::Exit::NotFound,
        message: message.to_string(),
    }
}
