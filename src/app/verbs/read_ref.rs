// `el read`: a narrow, revision-bound source range. The reference is
// an id or an alias; `--rev` pins the digest of the content as read,
// and a pin that no longer matches is `EL_STALE_REF` with the expected
// and observed digests named. This is opt-in pinning: the drift-tolerant
// reads (`show`, `ls`, `find`) keep ADR-0020's contract, and `read` is
// the verb that refuses to answer from a stale reference.

use super::super::{error, inspect, Env, Exit, Report};
use super::inspect::prepare;
use crate::app::report::{EnvelopeDiagnostic, EnvelopeRef};
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::model::id::ElId;
use crate::core::text::json::Json;

pub fn read_ref(
    env: &Env<'_>,
    reference: &str,
    rev: Option<&str>,
    lines: Option<(usize, usize)>,
) -> Report {
    let (catalog, projection) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("read", error),
    };
    let element = match resolve(&catalog, reference) {
        Ok(element) => element,
        Err(error) => return Report::fail("read", error),
    };
    let current = inspect::rev_of(&projection, element);
    if let Some(pinned) = rev {
        let pinned = pinned.strip_prefix("sha256:").unwrap_or(pinned);
        let observed = current.strip_prefix("sha256:").unwrap_or(&current);
        if pinned != observed {
            let mut report = Report::fail(
                "read",
                error::Error {
                    code: "EL_STALE_REF",
                    exit: Exit::Conflict,
                    message: "target changed after it was read".to_string(),
                },
            );
            report.diagnostics.push(EnvelopeDiagnostic {
                code: "EL_STALE_REF",
                message: "target changed after it was read".to_string(),
                hint: "read the target again, then re-plan",
                expected: Some(format!("sha256:{pinned}")),
                observed: Some(current.clone()),
                reference: Some(reference.to_string()),
            });
            return report;
        }
    }
    let source = inspect::source_of(&projection, element).unwrap_or_default();
    let all: Vec<&str> = source.lines().collect();
    let total = all.len();
    let (start, end) = match lines {
        None => (1, total),
        Some((start, end)) => {
            if start < 1 || start > end || start > total {
                return Report::fail(
                    "read",
                    error::Error {
                        code: "usage.bad_value",
                        exit: Exit::Usage,
                        message: format!("--lines {start}:{end} is outside the source ({total} lines)"),
                    },
                );
            }
            (start, end.min(total))
        }
    };
    let truncated = lines.is_some_and(|(_, end)| end < total);
    let window: Vec<&str> = if total == 0 { vec![] } else { all[start - 1..end].to_vec() };

    let mut report = Report::new("read");
    report.refs.push(EnvelopeRef { reference: reference.to_string(), rev: current.clone() });
    report.lines.push(format!("{reference} {current}"));
    report.lines.extend(window.iter().map(|line| line.to_string()));
    report.payload = Some(Json::Object(vec![
        ("ref".to_string(), Json::Str(reference.to_string())),
        ("id".to_string(), Json::Str(element.id.to_string())),
        ("kind".to_string(), Json::Str(element.kind.as_str().to_string())),
        ("module".to_string(), Json::Str(element.module.clone())),
        ("rev".to_string(), Json::Str(current)),
        (
            "lines".to_string(),
            Json::Array(window.iter().map(|line| Json::Str(line.to_string())).collect()),
        ),
        ("start".to_string(), Json::Number(start.to_string())),
        ("end".to_string(), Json::Number(end.to_string())),
        ("total_lines".to_string(), Json::Number(total.to_string())),
        ("truncated".to_string(), Json::Bool(truncated)),
    ]));
    report
}

/// An id resolves directly; anything else must be exactly one alias.
/// Two aliases for one string cannot both be the target, so that is a
/// refusal naming the count, not a guess at the first.
fn resolve<'a>(catalog: &'a Catalog, reference: &str) -> Result<&'a Element, error::Error> {
    if let Some(id) = ElId::parse(reference) {
        return catalog.find(id).ok_or_else(|| missing(id));
    }
    let matches: Vec<&Element> = catalog
        .elements
        .iter()
        .filter(|element| element.alias.as_ref().map(|alias| alias.to_string()) == Some(reference.to_string()))
        .collect();
    match matches.len() {
        0 => Err(error::Error {
            code: "notfound.element",
            exit: Exit::NotFound,
            message: format!("{reference} does not match any element"),
        }),
        1 => Ok(matches[0]),
        count => Err(error::Error {
            code: "EL_AMBIGUOUS_TARGET",
            exit: Exit::NotFound,
            message: format!("{reference} matches {count} elements; name one id"),
        }),
    }
}

fn missing(id: ElId) -> error::Error {
    error::Error {
        code: "notfound.element",
        exit: Exit::NotFound,
        message: format!("{id} does not exist"),
    }
}
