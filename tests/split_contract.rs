use el::core::text::split::{join, split};

// The contract for splitting a function body into statements (FR-64/FR-65):
//
//   1. Lossless. `join(split(body)?) == body`, byte for byte. A splitter that
//      trims or re-joins with its own separators loses the formatting an agent
//      must preserve, so losslessness is the property everything else rests on.
//   2. Only proved boundaries. A `;` or a closing `}` at nesting depth zero
//      ends a statement. A `;` inside a string, char, raw string, comment,
//      brace, bracket or paren does not.
//   3. Refusal over guessing. Anything whose boundary cannot be proved lexically
//      is `None`, never a plausible-looking wrong answer.
//
// A body that cannot be split is refused rather than approximated: this is the
// difference between a tool that is safe to run on real code and one that
// silently rearranges it.

/// Bodies that split cleanly, with the number of statements each must yield.
const BODIES: &[(&str, usize)] = &[
    ("", 0),
    ("   ", 1),
    ("let a = 1;", 1),
    ("let a = 1; let b = 2;", 2),
    ("let a = 1;\n    let b = 2;\n", 2),
    ("if x { a(); b(); } c();", 2),
    ("let s = \"a;b\";", 1),
    ("let s = \"a\\\";b\";", 1),
    ("let c = ';';", 1),
    ("let r = r#\"a;\"#;", 1),
    ("let r = r##\"a;\"#\"##;", 1),
    ("let b = b\";\";", 1),
    ("// a; b\nlet a = 1;", 1),
    ("/* ; */ let a = 1;", 1),
    ("/* /* ; */ ; */ let a = 1;", 1),
    ("m!{ a; b }", 1),
    ("let v = vec![1, 2];", 1),
    ("let t = (1, [2, 3]);", 1),
    ("'outer: loop { break 'outer; }", 1),
    ("if c { 1 } else { 2 }", 1),
    ("if c { 1 } else if d { 2 } else { 3 }", 1),
    ("if c { 1 } else { 2 } x();", 2),
    ("match x { _ => 1 }.to_string();", 1),
    ("a();\n// note\nb();", 2),
    ("fn f<'a>(x: &'a str) -> &'a str { x }", 1),
    ("let a = (1, 2); let b = [3];", 2),
    ("x();", 1),
];

/// Bodies whose statement boundary cannot be proved, and why.
const REFUSED: &[(&str, &str)] = &[
    ("let s = \"abc;", "unterminated string"),
    ("let r = r#\"abc;", "unterminated raw string"),
    ("/* abc;", "unterminated block comment"),
    ("if x { a();", "unclosed brace"),
    ("}", "stray closing brace"),
    ("a());", "stray closing paren"),
    ("let x = ' ;", "stray quote"),
    ("(]", "mismatched delimiter"),
];

#[test]
fn every_body_survives_the_round_trip() {
    for (body, expected_pieces) in BODIES {
        let pieces = split(body).unwrap_or_else(|| panic!("refused a splittable body: {body:?}"));

        assert_eq!(pieces.len(), *expected_pieces, "piece count for {body:?}");
        assert_eq!(join(&pieces), *body, "round trip for {body:?}");
    }
}

#[test]
fn splitting_a_split_is_the_same_split() {
    for (body, _) in BODIES {
        let pieces = split(body).expect("splittable");
        let rejoined = join(&pieces);
        let again = split(&rejoined).expect("still splittable");

        assert_eq!(pieces, again, "idempotence for {body:?}");
    }
}

#[test]
fn ambiguous_input_is_refused_with_none() {
    for (body, why) in REFUSED {
        assert_eq!(split(body), None, "should have refused: {why}: {body:?}");
    }
}

#[test]
fn a_nested_block_is_one_statement() {
    let body = "if x { a(); b(); }";

    let pieces = split(body).expect("splittable");

    assert_eq!(pieces, vec![body]);
}

#[test]
fn a_second_statement_starts_after_the_block_ends() {
    let body = "if x { a(); } b();";

    let pieces = split(body).expect("splittable");

    assert_eq!(pieces, vec!["if x { a(); }", " b();"]);
}

#[test]
fn an_else_branch_belongs_to_the_same_statement() {
    let body = "if c { a(); } else { b(); }";

    let pieces = split(body).expect("splittable");

    assert_eq!(pieces, vec![body]);
}

#[test]
fn a_comment_belongs_to_the_statement_that_follows_it() {
    let body = "a();\n// why\nb();";

    let pieces = split(body).expect("splittable");

    assert_eq!(pieces, vec!["a();", "\n// why\nb();"]);
}
