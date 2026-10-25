// Splitting a function body into statements is the operation every later
// element-level edit rests on: to change one line, `el` must know where that
// line begins and ends. This module turns the scanner's terminator indices into
// contiguous, losslessly re-joinable pieces.
//
// Shape of the solution: pieces are slices of the original body, never
// reconstructed strings. Concatenating them reproduces the input byte for byte,
// so formatting survives an edit to some other statement. Whitespace is carried
// by the statement on its left, and a trailing run is folded left, so a piece is
// never whitespace-only unless the whole body was.
//
// It deliberately does not decide whether a piece is a *valid* statement — the
// compiler does, behind the gate. What it does not do either: split at a depth
// it cannot prove, or guess past an unterminated literal. Both are `None`.

use super::terminator;

/// Split `body` into statements, each a contiguous slice of the input.
///
/// Contract:
///   - `Some(pieces)` implies `join(&pieces) == body`; nothing is trimmed.
///   - Pieces are non-overlapping and in source order; every piece contains a
///     non-whitespace character, except that a whitespace-only body yields one
///     piece.
///   - `None` when a boundary cannot be proved (see `terminator`).
///   - Pure and idempotent: splitting any piece yields that same piece.
pub fn split(body: &str) -> Option<Vec<&str>> {
    let ends = terminator::statement_ends(body)?;
    let merged = merge_whitespace(body, spans(body, &ends));

    Some(merged.into_iter().map(|(s, e)| &body[s..e]).collect())
}

/// The inverse of `split`: the original body, exactly.
pub fn join(pieces: &[&str]) -> String {
    pieces.concat()
}

/// Contiguous spans ending just after each terminator, plus any tail.
fn spans(body: &str, ends: &[usize]) -> Vec<(usize, usize)> {
    let mut spans = Vec::with_capacity(ends.len() + 1);
    let mut start = 0;

    for &end in ends {
        // Terminators are ASCII, so one byte after the index is a safe edge.
        spans.push((start, end + 1));
        start = end + 1;
    }
    if start < body.len() {
        spans.push((start, body.len()));
    }
    spans
}

/// Attach whitespace-only spans to the statement beside them, preferring the
/// one on the left; a leading run attaches right, and a lone run stands alone.
fn merge_whitespace(body: &str, spans: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::new();
    let mut pending: Option<(usize, usize)> = None;

    for (start, end) in spans {
        if body[start..end].trim().is_empty() {
            pending = Some(match pending {
                Some((first, _)) => (first, end),
                None => (start, end),
            });
            continue;
        }
        merged.push((pending.take().map(|(start, _)| start).unwrap_or(start), end));
    }

    if let Some((start, end)) = pending {
        match merged.last_mut() {
            Some(last) => last.1 = end.max(last.1),
            None => merged.push((start, end)),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_body_has_no_statements() {
        assert_eq!(split(""), Some(vec![]));
    }

    #[test]
    fn a_blank_body_is_one_piece() {
        assert_eq!(split("  \n "), Some(vec!["  \n "]));
    }

    #[test]
    fn trailing_whitespace_belongs_to_the_last_statement() {
        assert_eq!(split("a();\n"), Some(vec!["a();\n"]));
    }

    #[test]
    fn two_statements_keep_their_separator() {
        assert_eq!(split("a(); b();"), Some(vec!["a();", " b();"]));
    }

    #[test]
    fn joining_restores_the_body() {
        let body = "let a = 1;\n  let b = \"x;y\";\n";

        assert_eq!(join(&split(body).expect("splittable")), body);
    }
}
