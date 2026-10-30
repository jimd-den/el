// `el discover`: bounded, ranked discovery over the catalog. Results
// are ranked name-exact, then name-substring, then alias, then module,
// and the page is bounded by `--limit` with an offset cursor — so the
// output is a window the agent can re-read, never a dump. Each result
// carries its revision digest, and the envelope's `refs` stand on the
// returned page.

use super::super::{error, inspect, Env, Exit, Report};
use super::inspect::prepare;
use crate::app::report::EnvelopeRef;
use crate::core::model::element::Element;
use crate::core::text::json::Json;

pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;
pub const PREVIEW_CHARS: usize = 120;

pub fn discover(env: &Env<'_>, query: &str, limit: usize, cursor: usize) -> Report {
    if query.is_empty() {
        return Report::fail(
            "discover",
            error::Error {
                code: "usage.bad_value",
                exit: Exit::Usage,
                message: "discover needs a non-empty --query".to_string(),
            },
        );
    }
    if limit > MAX_LIMIT {
        return Report::fail(
            "discover",
            error::Error {
                code: "usage.bad_value",
                exit: Exit::Usage,
                message: format!("--limit takes at most {MAX_LIMIT}").to_string(),
            },
        );
    }
    let (catalog, projection) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("discover", error),
    };
    let needle = query.to_lowercase();
    let mut ranked: Vec<(usize, &Element)> = catalog
        .elements
        .iter()
        .filter_map(|element| rank(element, &needle).map(|rank| (rank, element)))
        .collect();
    ranked.sort_by(|a, b| {
        let key = |element: &Element| {
            element
                .alias
                .as_ref()
                .map(|alias| alias.to_string())
                .unwrap_or_else(|| element.id.to_string())
        };
        (a.0, key(a.1), a.1.id).cmp(&(b.0, key(b.1), b.1.id))
    });
    let total = ranked.len();
    let page: Vec<(usize, &Element)> = ranked.into_iter().skip(cursor).take(limit).collect();
    let returned = page.len();
    let truncated = cursor + returned < total;
    let next_cursor = if truncated { Some(cursor + returned) } else { None };

    let mut report = Report::new("discover");
    let mut rows = Vec::new();
    for (_, element) in &page {
        let reference = element
            .alias
            .as_ref()
            .map(|alias| alias.to_string())
            .unwrap_or_else(|| element.id.to_string());
        let rev = inspect::rev_of(&projection, element);
        let preview: String = element.head.lines().next().unwrap_or("").chars().take(PREVIEW_CHARS).collect();
        report.refs.push(EnvelopeRef { reference: reference.clone(), rev: rev.clone() });
        rows.push(Json::Object(vec![
            ("id".to_string(), Json::Str(element.id.to_string())),
            ("kind".to_string(), Json::Str(element.kind.as_str().to_string())),
            (
                "alias".to_string(),
                element.alias.as_ref().map(|alias| Json::Str(alias.to_string())).unwrap_or(Json::Null),
            ),
            ("module".to_string(), Json::Str(element.module.clone())),
            ("preview".to_string(), Json::Str(preview.clone())),
            ("ref".to_string(), Json::Str(reference)),
            ("rev".to_string(), Json::Str(rev)),
        ]));
        report.lines.push(format!(
            "{} {} {}",
            element.id,
            element.alias.as_ref().map(|alias| alias.to_string()).unwrap_or_else(|| "-".to_string()),
            preview
        ));
    }
    if truncated {
        report.lines.push(format!(
            "{} of {total}; next cursor {}",
            returned,
            next_cursor.unwrap()
        ));
    }
    report.payload = Some(Json::Array(rows));
    report.pagination = Some(Json::Object(vec![
        ("truncated".to_string(), Json::Bool(truncated)),
        (
            "next_cursor".to_string(),
            next_cursor.map(|cursor| Json::Number(cursor.to_string())).unwrap_or(Json::Null),
        ),
        ("returned".to_string(), Json::Number(returned.to_string())),
        ("total".to_string(), Json::Number(total.to_string())),
    ]));
    report
}

/// 0 for an exact name, 1 for a name substring, 2 for an alias
/// substring, 3 for a module substring, none when nothing matches.
/// The query is already lowercased; every field is compared the same way.
fn rank(element: &Element, needle: &str) -> Option<usize> {
    let name = element.name.as_deref().unwrap_or("").to_lowercase();
    if name == needle {
        return Some(0);
    }
    if name.contains(needle) {
        return Some(1);
    }
    if element
        .alias
        .as_ref()
        .map(|alias| alias.to_string().to_lowercase().contains(needle))
        .unwrap_or(false)
    {
        return Some(2);
    }
    if element.module.to_lowercase().contains(needle) {
        return Some(3);
    }
    None
}
