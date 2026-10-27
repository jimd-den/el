// A script line is argv, one line per change. The tokenizer is the only place
// quote rules live, so an `--sig` carrying spaces, a `#` inside a string, and a
// comment after a word all mean what they mean here and nowhere else.

use super::*;

fn words(line: &str) -> Vec<String> {
    parse(line).expect("parse").remove(0)
}

#[test]
fn spaces_inside_single_quotes_stay_in_one_word() {
    let got = words("add fn run --sig 'fn run(p: &Plan) -> Verdict' --in core/gate.rs");

    assert_eq!(got[4], "fn run(p: &Plan) -> Verdict");
    assert_eq!(got.len(), 7);
}

#[test]
fn double_quotes_take_escapes_and_keep_spaces() {
    let got = words(r#"edit el-0007 --body "a \"quoted\" word""#);

    assert_eq!(got[3], "a \"quoted\" word");
}

#[test]
fn a_comment_ends_the_line_but_not_a_quoted_hash() {
    let got = words("rm el-0001  # the old one, gone");
    assert_eq!(got, ["rm", "el-0001"]);

    let got = words("add const SIG --sig 'fn f() -> &str # real' # comment");
    assert_eq!(got[4], "fn f() -> &str # real");
}

#[test]
fn blank_and_comment_lines_are_skipped() {
    let lines = parse("\n# a note\n\nrm el-0001\n").expect("parse");

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], ["rm", "el-0001"]);
}

#[test]
fn an_unterminated_quote_names_its_line() {
    let error = parse("# note\nadd fn f --sig 'fn f(\n").expect_err("unterminated");

    assert_eq!(error.line, 2);
    assert!(error.message.contains("quote"), "{error}");
}

#[test]
fn a_backslash_escapes_the_next_character() {
    let got = words(r"add fn f --sig fn f\(x\)");

    assert_eq!(got[5], "f(x)");
}
