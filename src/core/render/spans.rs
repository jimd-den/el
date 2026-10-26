// The span map: which element owns which line of which file (FR-17). The
// diagnostic delta depends on it — a compiler error is a path and a line, and
// "does this change introduce a new error" is a question about elements.
//
// Spans are 1-based and inclusive at both ends, because that is what rustc and
// rust-analyzer emit; converting at the boundary is where off-by-one bugs live,
// so there is no conversion.
//
// Lookup answers the *innermost* element containing the line. Statements nest
// inside items and items inside modules, so "innermost" is "the one that starts
// last", which is also what a reader would call the most specific answer
// (FR-68, FR-22).

use super::super::model::id::ElId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub path: String,
    /// First line, 1-based.
    pub first: u32,
    /// Last line, 1-based, inclusive.
    pub last: u32,
    pub element: ElId,
}

/// Gathers spans while the renderer writes, then sorts them into the canonical
/// order: by path, then by start line, then by element id.
#[derive(Debug, Default)]
pub struct Collector {
    spans: Vec<Span>,
}

impl Collector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, path: &str, first: u32, last: u32, element: ElId) {
        self.spans.push(Span {
            path: path.to_string(),
            first,
            last,
            element,
        });
    }

    pub fn finish(mut self) -> Vec<Span> {
        self.spans.sort_by(|a, b| {
            (a.path.as_str(), a.first, a.element).cmp(&(b.path.as_str(), b.first, b.element))
        });
        self.spans
    }
}

/// The innermost element owning `line` in `path`.
pub fn innermost(spans: &[Span], path: &str, line: u32) -> Option<ElId> {
    spans
        .iter()
        .filter(|span| span.path == path && span.first <= line && line <= span.last)
        .max_by_key(|span| (span.first, span.last - span.first))
        .map(|span| span.element)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_innermost_span_wins() {
        let spans = vec![
            Span {
                path: "src/a.rs".to_string(),
                first: 1,
                last: 10,
                element: ElId(1),
            },
            Span {
                path: "src/a.rs".to_string(),
                first: 5,
                last: 6,
                element: ElId(2),
            },
        ];

        assert_eq!(innermost(&spans, "src/a.rs", 5), Some(ElId(2)));
        assert_eq!(innermost(&spans, "src/a.rs", 8), Some(ElId(1)));
    }

    #[test]
    fn a_line_outside_every_span_has_no_owner() {
        let spans = vec![Span {
            path: "src/a.rs".to_string(),
            first: 3,
            last: 4,
            element: ElId(1),
        }];

        assert_eq!(innermost(&spans, "src/a.rs", 2), None);
        assert_eq!(innermost(&spans, "src/b.rs", 3), None);
    }

    #[test]
    fn spans_sort_by_path_then_start() {
        let mut collector = Collector::new();
        collector.push("src/b.rs", 1, 2, ElId(9));
        collector.push("src/a.rs", 7, 8, ElId(3));
        collector.push("src/a.rs", 1, 2, ElId(2));

        let spans = collector.finish();

        let order: Vec<(String, u32)> = spans
            .into_iter()
            .map(|span| (span.path, span.first))
            .collect();
        assert_eq!(
            order,
            vec![
                ("src/a.rs".to_string(), 1),
                ("src/a.rs".to_string(), 7),
                ("src/b.rs".to_string(), 1)
            ]
        );
    }
}
