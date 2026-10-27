// The read verbs: `run`, `log`, `version`. They share one property — they never
// stage a transaction. `run` holds the writer lock (it executes the projection),
// so it may roll back an interrupted one and refuses a drifted tree; `log` holds
// neither and repairs nothing.

use super::super::{error, load, projection, Env, Exit, Report};
use crate::core::history::entry::Entry;
use crate::core::history::object;
use crate::core::text::json::Json;

pub fn run(env: &Env<'_>, args: &[String]) -> Report {
    if let Err(error) = load::ensure_crate(env) {
        return Report::fail("run", error);
    }
    if let Err(error) = super::super::transaction::recover(env) {
        return Report::fail("run", error);
    }
    let catalog = match load::catalog(env) {
        Ok(catalog) => catalog,
        Err(error) => return Report::fail("run", error),
    };
    if let Err(error) = projection::check_drift(env, &catalog, &env.crate_name) {
        return Report::fail("run", error);
    }

    let mut argv = vec!["cargo".to_string(), "run".to_string()];
    if !args.is_empty() {
        argv.push("--".to_string());
        argv.extend(args.iter().cloned());
    }
    let finished = match env.tools.run(&argv, None) {
        Ok(finished) => finished,
        Err(failure) => {
            return Report::fail("run", error::Error {
                code: "env.missing_tool",
                exit: Exit::Env,
                message: failure.to_string(),
            });
        }
    };
    let mut report = Report::new("run");
    report.lines = finished.stdout.lines().map(str::to_string).collect();
    report.notes = finished.stderr.lines().map(str::to_string).collect();
    report.payload = Some(Json::Object(vec![
        ("code".to_string(), Json::Number(finished.code.to_string())),
        ("stdout".to_string(), Json::Str(finished.stdout.clone())),
        ("stderr".to_string(), Json::Str(finished.stderr.clone())),
    ]));
    if finished.code != 0 {
        report = Report::fail("run", error::Error {
            code: "env.run_failed",
            exit: Exit::Env,
            message: format!("the program exited {}", finished.code),
        });
        report.notes = finished.stderr.lines().map(str::to_string).collect();
    }
    report
}

pub fn log(env: &Env<'_>, count: Option<usize>, attic: bool) -> Report {
    if let Err(error) = load::ensure_crate(env) {
        return Report::fail("log", error);
    }
    let mut entries = match env.journal.entries() {
        Ok(entries) => entries,
        Err(error) => return Report::fail("log", error::store(error)),
    };
    if attic {
        match env.journal.attic_entries() {
            Ok(entries) => entries,
            Err(error) => return Report::fail("log", error::store(error)),
        }
        .into_iter()
        .for_each(|entry| entries.push(entry));
    }
    entries.sort_by_key(|entry| entry.seq);
    let mut report = Report::new("log");
    let visible: Vec<&Entry> = entries.iter().rev().take(count.unwrap_or(usize::MAX)).collect();
    report.lines = visible.iter().map(|entry| line(entry)).collect();
    report.payload = Some(Json::Array(visible.iter().map(|entry| entry_payload(entry)).collect()));
    report
}

fn line(entry: &Entry) -> String {
    format!(
        "{:06}  {}  {}  {}  {}",
        entry.seq,
        entry.actor,
        entry.op.as_str(),
        entry.gate,
        entry.msg
    )
}

pub(super) fn entry_payload(entry: &Entry) -> Json {
    Json::Object(vec![
        ("seq".to_string(), Json::Number(entry.seq.to_string())),
        ("ts".to_string(), Json::Number(entry.ts.to_string())),
        ("actor".to_string(), Json::Str(entry.actor.clone())),
        ("op".to_string(), Json::Str(entry.op.as_str().to_string())),
        ("target".to_string(), Json::Str(entry.target.clone())),
        ("from".to_string(), Json::Str(object::show(&entry.from))),
        ("to".to_string(), Json::Str(object::show(&entry.to))),
        ("gate".to_string(), Json::Str(entry.gate.clone())),
        ("msg".to_string(), Json::Str(entry.msg.clone())),
    ])
}

pub fn version() -> Report {
    let mut report = Report::new("version");
    let text = "el 0.1.0 (M6)".to_string();
    report.lines.push(text.clone());
    report.payload = Some(Json::Str(text));
    report
}
