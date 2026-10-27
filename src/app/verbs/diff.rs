// `el diff`: the line view of two trees, and — when both sequences have a catalog
// snapshot — the element/statement view that says which id changed. The id is the
// point: a line range would move when a statement above it is inserted.

use super::super::{error, load, Env, Report};
use crate::core::history::entry::Entry;
use crate::core::model::catalog::Catalog;
use crate::core::text::diff::{self, Line};
use crate::core::text::json::Json;

mod elements;
use elements::{change_json, element_changes, element_lines, line_text};

pub fn diff(env: &Env<'_>, a: Option<u64>, b: Option<u64>) -> Report {
    if let Err(error) = load::ensure_crate(env) {
        return Report::fail("diff", error);
    }
    let head = match env.journal.head().map_err(error::store) {
        Ok(Some(head)) => head,
        Ok(None) => return Report::fail("diff", missing("the crate has no history yet")),
        Err(error) => return Report::fail("diff", error),
    };
    let (from, to) = match (a, b) {
        (None, None) => (head.saturating_sub(1), head),
        (Some(a), None) => (a, head),
        (Some(a), Some(b)) => (a, b),
        (None, Some(b)) => (head, b),
    };
    let left = match files_at(env, from) {
        Ok(files) => files,
        Err(error) => return Report::fail("diff", error),
    };
    let right = match files_at(env, to) {
        Ok(files) => files,
        Err(error) => return Report::fail("diff", error),
    };
    let left_catalog = catalog_at(env, from);
    let right_catalog = catalog_at(env, to);
    let mut report = Report::new("diff");
    let mut payload = vec![
        ("from".to_string(), Json::Number(from.to_string())),
        ("to".to_string(), Json::Number(to.to_string())),
    ];

    if let (Some(left), Some(right)) = (&left_catalog, &right_catalog) {
        let changes = element_changes(left, right);
        report.lines = changes.iter().flat_map(element_lines).collect();
        payload.push(("elements".to_string(), Json::Array(changes.iter().map(change_json).collect())));
    }
    let files = file_changes(&left, &right);
    report.lines.extend(files.iter().flat_map(file_lines));
    payload.push(("files".to_string(), Json::Array(files.iter().map(file_json).collect())));
    report.payload = Some(Json::Object(payload));
    report
}

fn files_at(env: &Env<'_>, seq: u64) -> Result<Vec<(String, String)>, error::Error> {
    if seq == 0 {
        return Ok(Vec::new());
    }
    let entry = entry_at(env, seq)?
        .ok_or_else(|| missing(&format!("sequence {seq} does not exist")))?;
    let tree = env.journal.get_tree(&entry.to).map_err(error::store)?;
    let mut files = Vec::new();
    for (path, digest) in tree.entries() {
        let bytes = env.journal.get_blob(digest).map_err(error::store)?;
        let text = String::from_utf8(bytes).map_err(|_| error::corrupt(format!("{path} is not UTF-8")))?;
        files.push((path.to_string(), text));
    }
    Ok(files)
}

fn catalog_at(env: &Env<'_>, seq: u64) -> Option<Catalog> {
    if seq == 0 {
        return Some(Catalog::new());
    }
    env.journal
        .catalog_at(seq)
        .ok()
        .flatten()
        .and_then(|text| Catalog::decode(&text))
}

fn entry_at(env: &Env<'_>, seq: u64) -> Result<Option<Entry>, error::Error> {
    if let Some(entry) = env.journal.entry(seq).map_err(error::store)? {
        return Ok(Some(entry));
    }
    Ok(env
        .journal
        .attic_entries()
        .map_err(error::store)?
        .into_iter()
        .find(|entry| entry.seq == seq))
}

pub(super) fn lines_json(lines: &[Line]) -> Json {
    Json::Array(lines.iter().map(|line| {
        let (op, text) = match line {
            Line::Same(text) => (" ", text),
            Line::Remove(text) => ("-", text),
            Line::Add(text) => ("+", text),
        };
        Json::Object(vec![
            ("op".to_string(), Json::Str(op.to_string())),
            ("text".to_string(), Json::Str(text.clone())),
        ])
    }).collect())
}

fn file_changes(left: &[(String, String)], right: &[(String, String)]) -> Vec<(String, Vec<Line>)> {
    let mut out = Vec::new();
    for (path, new) in right {
        match left.iter().find(|(candidate, _)| candidate == path) {
            Some((_, old)) if old == new => {}
            Some((_, old)) => out.push((path.clone(), diff::lines(old, new))),
            None => out.push((path.clone(), diff::lines("", new))),
        }
    }
    for (path, old) in left {
        if !right.iter().any(|(candidate, _)| candidate == path) {
            out.push((path.clone(), diff::lines(old, "")));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn file_lines(change: &(String, Vec<Line>)) -> Vec<String> {
    let mut out = vec![change.0.clone()];
    out.extend(change.1.iter().map(line_text));
    out
}

fn file_json(change: &(String, Vec<Line>)) -> Json {
    Json::Object(vec![
        ("path".to_string(), Json::Str(change.0.clone())),
        ("lines".to_string(), lines_json(&change.1)),
    ])
}

fn missing(message: &str) -> error::Error {
    error::Error {
        code: "notfound.sequence",
        exit: super::super::Exit::NotFound,
        message: message.to_string(),
    }
}
