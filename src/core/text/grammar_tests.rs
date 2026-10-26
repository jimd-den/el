// Tests for `grammar`; sibling of the module they exercise (§4.4).
//
// Every assertion is against literal body text, so a change in trimming, brace
// ownership or semicolon policy fails here rather than three layers away in the
// renderer.

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
fn an_empty_or_blank_body_has_no_statements() {
    assert_eq!(parse_ok(""), Body::default());
    assert_eq!(parse_ok("\n  \n\t\n"), Body::default());
}

#[test]
fn whitespace_is_trimmed_from_each_line() {
    assert_eq!(texts("  a()  \n\n\t b()\n"), ["a();", "b()"]);
}

#[test]
fn a_single_statement_is_a_tail_expression() {
    assert_eq!(texts("a()"), ["a()"]);
}

#[test]
fn a_non_tail_statement_gains_a_semicolon() {
    assert_eq!(texts("a()\nb()"), ["a();", "b()"]);
}

#[test]
fn a_statement_that_cannot_be_a_tail_expression_gains_a_semicolon() {
    assert_eq!(texts("let x = 1"), ["let x = 1;"]);
    assert_eq!(texts("use a::b"), ["use a::b;"]);
    assert_eq!(texts("const N: u8 = 1"), ["const N: u8 = 1;"]);
    assert_eq!(texts("static N: u8 = 1"), ["static N: u8 = 1;"]);
    assert_eq!(texts("type Item = u8"), ["type Item = u8;"]);
    assert_eq!(texts("return x"), ["return x;"]);
    assert_eq!(texts("break"), ["break;"]);
    assert_eq!(texts("continue"), ["continue;"]);
}

#[test]
fn a_comment_or_attribute_statement_gains_no_semicolon() {
    assert_eq!(texts("///doc\nlet x = 1"), ["///doc", "let x = 1;"]);
    assert_eq!(texts("// note\na()"), ["// note", "a()"]);
    assert_eq!(texts("#[cfg(test)]\ntest()"), ["#[cfg(test)]", "test()"]);
}

#[test]
fn a_statement_ending_in_a_brace_gains_no_semicolon() {
    assert_eq!(texts("if x { a(); }"), ["if x { a(); }"]);
}

#[test]
fn a_block_owns_the_lines_until_its_closing_brace() {
    let expected = body(vec![with_children("if x {", vec![leaf("a()")])]);

    assert_eq!(parse_ok("if x {\n    a()\n}"), expected);
}

#[test]
fn a_block_without_its_final_brace_is_still_a_body() {
    let expected = body(vec![with_children("if x {", vec![leaf("a()")])]);

    assert_eq!(parse_ok("if x {\n    a()"), expected);
}

#[test]
fn blocks_nest_to_any_depth() {
    let inner = with_children("if y {", vec![leaf("a()")]);
    let expected = body(vec![with_children("while x {", vec![inner])]);

    assert_eq!(parse_ok("while x {\n if y {\n a()\n }\n}"), expected);
}

#[test]
fn more_closing_braces_than_blocks_is_an_error() {
    assert_eq!(parse("a()\n}"), Err(Error::UnexpectedClose { line: 2 }));
}

#[test]
fn an_unterminated_literal_or_comment_is_refused() {
    assert_eq!(parse("log(\"open"), Err(Error::Unterminated { line: 1 }));
    assert_eq!(parse("a()\n/* open"), Err(Error::Unterminated { line: 2 }));
}

#[test]
fn a_wrong_closer_is_refused() {
    assert_eq!(parse("let v = (1];"), Err(Error::Unbalanced { line: 1 }));
}

#[test]
fn an_unclosed_brace_that_is_not_a_block_opener_is_refused() {
    assert_eq!(parse("if x { a();"), Err(Error::Unbalanced { line: 1 }));
}

#[test]
fn a_parenthesis_closes_at_the_end_of_its_logical_statement() {
    assert_eq!(texts("call(a"), ["call(a)"]);
    assert_eq!(texts("call(a\nb)"), ["call(a\nb)"]);
}

#[test]
fn an_unclosed_bracket_is_closed_before_the_semicolon() {
    assert_eq!(texts("let v = [1, 2"), ["let v = [1, 2];"]);
}

#[test]
fn nested_unclosed_brackets_are_closed_in_reverse_order() {
    assert_eq!(texts("f([1, 2"), ["f([1, 2])"]);
}

#[test]
fn a_balanced_prefix_before_a_block_brace_opens_a_block() {
    let expected = body(vec![with_children("if foo(a) {", vec![leaf("b()")])]);

    assert_eq!(parse_ok("if foo(a) {\n b()\n}"), expected);
}

#[test]
fn a_trailing_brace_is_a_block_opener_not_a_missing_closer() {
    let expected = body(vec![with_children("if x {", Vec::new())]);

    assert_eq!(parse_ok("if x {"), expected);
}

#[test]
fn an_else_after_a_closed_block_is_refused() {
    let text = "if x {\n a();\n}\nelse {\n b();\n}";

    assert_eq!(parse(text), Err(Error::ElseAfterBlock { line: 4 }));
}

#[test]
fn an_else_branch_on_the_closing_line_is_refused() {
    let text = "if x {\n a();\n} else {\n b();\n}";

    assert_eq!(parse(text), Err(Error::ElseAfterBlock { line: 3 }));
}

#[test]
fn a_trailing_comma_gains_no_semicolon() {
    let expected = body(vec![with_children(
        "Point {",
        vec![leaf("x: 1,"), leaf("y: 2,")],
    )]);

    assert_eq!(parse_ok("Point {\n x: 1,\n y: 2,\n}"), expected);
}

#[test]
fn crlf_line_endings_are_trimmed() {
    assert_eq!(texts("a()\r\nb()\r\n"), ["a();", "b()"]);
}

#[test]
fn display_names_the_line_and_the_failure() {
    let unterminated = Error::Unterminated { line: 2 }.to_string();
    let unbalanced = Error::Unbalanced { line: 3 }.to_string();
    let unexpected = Error::UnexpectedClose { line: 4 }.to_string();
    let else_branch = Error::ElseAfterBlock { line: 5 }.to_string();

    assert_eq!(unexpected, "line 4: unexpected `}` with no open block");
    assert!(unterminated.contains("line 2") && unterminated.contains("unterminated"));
    assert!(unbalanced.contains("line 3") && unbalanced.contains("unbalanced"));
    assert!(else_branch.contains("line 5") && else_branch.contains("else"));
}
