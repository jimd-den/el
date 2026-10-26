// Tests for multi-line logical statements; separate from `grammar_tests` so each
// file stays inside the 200-line ceiling. A logical statement continues while an
// `(` or `[` is open; these tests pin the text that reaches the tree, newlines
// included, not the renderer's indentation.

use super::*;
use crate::core::model::body::{Body, BodyStmt};

fn leaf(text: &str) -> BodyStmt {
    BodyStmt {
        text: text.to_string(),
        children: Vec::new(),
    }
}

fn with_children(text: &str, children: Vec<BodyStmt>) -> BodyStmt {
    BodyStmt {
        text: text.to_string(),
        children,
    }
}

fn body(statements: Vec<BodyStmt>) -> Body {
    Body { statements }
}

fn parse_ok(text: &str) -> Body {
    parse(text).expect("body should parse")
}

fn texts(text: &str) -> Vec<String> {
    parse_ok(text)
        .statements
        .into_iter()
        .map(|s| s.text)
        .collect()
}

#[test]
fn a_call_spans_lines_until_its_parenthesis_closes() {
    assert_eq!(
        texts("log(\n    \"a\",\n    \"b\"\n)"),
        ["log(\n\"a\",\n\"b\"\n)"]
    );
}

#[test]
fn an_array_of_structs_spans_lines_until_its_brackets_close() {
    let text = "let v = vec![\n    Point {\n        x: 1,\n    },\n];";

    assert_eq!(texts(text), ["let v = vec![\nPoint {\nx: 1,\n},\n];"]);
}

#[test]
fn a_multiline_expression_inside_a_block_keeps_its_newlines() {
    let expected = body(vec![with_children("if x {", vec![leaf("take(\n1,\n2\n)")])]);

    assert_eq!(parse_ok("if x {\n take(\n 1,\n 2\n )\n}"), expected);
}

#[test]
fn a_multiline_statement_gets_one_semicolon_after_it_completes() {
    assert_eq!(
        texts("log(\n    \"a\"\n)\nnext()"),
        ["log(\n\"a\"\n);", "next()"]
    );
}

#[test]
fn a_brace_inside_an_open_call_does_not_open_a_block() {
    assert_eq!(
        texts("take(Foo {\n    x: 1,\n})"),
        ["take(Foo {\nx: 1,\n})"]
    );
}

#[test]
fn a_closing_brace_inside_a_multiline_expression_does_not_close_a_block() {
    assert_eq!(
        texts("vec![\n    Foo {\n        x: 1,\n    }\n]"),
        ["vec![\nFoo {\nx: 1,\n}\n]"]
    );
}
