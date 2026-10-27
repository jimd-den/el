// `el doctor`: the two explicit recovery operations. `--repair` performs the same
// inflight rollback every command does, but says what it found; `--rebuild`
// rewrites `.el/catalog` from the head's snapshot and refuses to pretend when
// there is no snapshot to rebuild from.

use super::super::{error, load, transaction, Env, Report};
use crate::core::text::json::Json;

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
    let version = match env.journal.version() {
        Ok(version) => version.unwrap_or_else(|| "-".to_string()),
        Err(error) => format!("error: {error}"),
    };
    let head = match env.journal.head() {
        Ok(head) => head.map_or("-".to_string(), |h| h.to_string()),
        Err(error) => format!("error: {error}"),
    };
    let journal = match env.journal.entries() {
        Ok(entries) => format!("{} entries", entries.len()),
        Err(error) => format!("error: {error}"),
    };
    let attic = match env.journal.attic_entries() {
        Ok(entries) => format!("{} entries", entries.len()),
        Err(error) => format!("error: {error}"),
    };
    let inflight = inflight(env);
    let drift = drift(env);
    let objects = objects(env);

    report.lines = vec![
        format!("root        {}", env.root),
        format!("version     {version}"),
        format!("head        {head}"),
        format!("journal     {journal}"),
        format!("attic       {attic}"),
        format!("inflight    {inflight}"),
        format!("drift       {drift}"),
        format!("objects     {objects}"),
    ];
    report.payload = Some(Json::Object(vec![
        ("root".to_string(), Json::Str(env.root.clone())),
        ("version".to_string(), Json::Str(version)),
        ("head".to_string(), Json::Str(head)),
        ("journal".to_string(), Json::Str(journal)),
        ("attic".to_string(), Json::Str(attic)),
        ("inflight".to_string(), Json::Str(inflight)),
        ("drift".to_string(), Json::Str(drift)),
        ("objects".to_string(), Json::Str(objects)),
    ]));
    report
}

/// Whether a transaction was interrupted. The marker's own words are a pid, a
/// digest and a timestamp, which is why the line names the command that clears it.
fn inflight(env: &Env<'_>) -> String {
    match env.journal.inflight() {
        Ok(None) => "none".to_string(),
        Ok(Some(marker)) => {
            let seq = marker.split_whitespace().next().unwrap_or("?");
            format!("seq {seq} pending; `el doctor --repair` rolls it back")
        }
        Err(error) => format!("error: {error}"),
    }
}

/// Read-only drift: unlike `projection::check_drift`, this never re-renders a
/// whitespace difference, because `doctor` without `--repair` must not write.
fn drift(env: &Env<'_>) -> String {
    let catalog = match load::catalog(env) {
        Ok(catalog) => catalog,
        Err(error) => return format!("unknown ({})", error.message),
    };
    let projection = crate::core::render::render(&catalog, &env.crate_name);
    for file in &projection.files {
        if file.path == "Cargo.toml" {
            continue;
        }
        match env.fs.read(&file.path) {
            Ok(Some(bytes)) if bytes == file.text.as_bytes() => continue,
            Ok(Some(_)) => return format!("{} differs from the catalog", file.path),
            Ok(None) => return format!("{} is missing", file.path),
            Err(error) => return format!("{} could not be read ({error})", file.path),
        }
    }
    let known: std::collections::BTreeSet<&str> =
        projection.files.iter().map(|file| file.path.as_str()).collect();
    if let Ok(paths) = env.fs.list("src") {
        for path in paths {
            if !known.contains(path.as_str()) {
                return format!("{path} is not in the projection");
            }
        }
    }
    "none".to_string()
}

fn objects(env: &Env<'_>) -> String {
    match env.fs.list(".el/objects") {
        Ok(files) => format!("{} blobs", files.len()),
        Err(error) => format!("error: {error}"),
    }
}

fn missing(message: &str) -> error::Error {
    error::Error {
        code: "notfound.sequence",
        exit: crate::app::Exit::NotFound,
        message: message.to_string(),
    }
}
