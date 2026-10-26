// Tests for `entry`; sibling of the module they exercise (§4.4).
//
// The two properties worth pinning are the boring ones: a record is exactly ten
// fields so the next field added is a version bump rather than a silent shift, and
// free text cannot forge a line. Both are asserted against literal text, not against
// the encoder's own output, so a changed separator fails here.

use super::*;
use crate::core::history::object::of;

fn entry() -> Entry {
    Entry {
        seq: 42,
        ts: 1_767_225_600,
        actor: "human".to_string(),
        op: Op::Add,
        target: "el-0007".to_string(),
        from: of(b"before"),
        to: of(b"after"),
        gate: "ok:new".to_string(),
        msg: "added Plan::clamp".to_string(),
    }
}

#[test]
fn an_entry_is_exactly_ten_tab_separated_fields() {
    let line = entry().encode();

    assert_eq!(line.split('\t').count(), 10);
    assert!(line.ends_with('\n'));
    assert_eq!(line.lines().count(), 1, "a record is one line");
}

#[test]
fn the_first_field_is_the_format_tag() {
    assert!(
        entry()
            .encode()
            .starts_with("el1\t42\t1767225600\thuman\tadd\t")
    );
}

#[test]
fn encode_and_decode_round_trip() {
    let original = entry();

    assert_eq!(Entry::decode(&original.encode()), Ok(original));
}

#[test]
fn decoding_refuses_a_field_count_that_is_not_ten() {
    let error = Entry::decode("el1\t1\t2\thuman\tadd").expect_err("five fields");

    assert_eq!(error.field, "field count");
    assert!(
        error
            .to_string()
            .starts_with("env.corrupt: entry field count is "),
        "{error}"
    );
}

#[test]
fn decoding_refuses_a_tag_it_was_never_taught() {
    let line = entry().encode().replacen("el1", "el2", 1);
    let error = Entry::decode(&line).expect_err("future format");

    assert_eq!(error.field, "tag");
    assert_eq!(error.text, "el2");
}

#[test]
fn decoding_names_the_field_that_failed() {
    let line = entry().encode();

    let bad_seq = line.replacen("\t42\t", "\tforty-two\t", 1);
    let bad_op = line.replacen("\tadd\t", "\tappend\t", 1);
    let bad_digest = line.replacen(&object::show(&of(b"after")), "nothex", 1);

    assert_eq!(Entry::decode(&bad_seq).expect_err("seq").field, "seq");
    assert_eq!(Entry::decode(&bad_op).expect_err("op").field, "op");
    assert_eq!(Entry::decode(&bad_digest).expect_err("to").field, "to");
}

#[test]
fn free_text_is_sanitized_so_a_record_cannot_forge_the_next_one() {
    let mut dirty = entry();
    dirty.msg = "line one\nline two\ttabbed\rreturned".to_string();
    dirty.actor = "hu\nman".to_string();

    let encoded = dirty.encode();

    assert_eq!(
        encoded.lines().count(),
        1,
        "a newline in free text would forge a line"
    );
    assert_eq!(
        encoded.split('\t').count(),
        10,
        "a tab in free text would forge a field"
    );
    assert!(encoded.contains("line one line two tabbed returned"));
    assert_eq!(Entry::decode(&encoded).expect("decodes").actor, "hu man");
}

#[test]
fn a_long_message_is_truncated_to_two_hundred_characters() {
    let mut long = entry();
    long.msg = "é".repeat(250);

    let decoded = Entry::decode(&long.encode()).expect("decodes");

    assert_eq!(decoded.msg.chars().count(), 200);
    assert!(
        decoded.msg.chars().all(|c| c == 'é'),
        "truncation is by character, not byte"
    );
}

#[test]
fn every_operation_has_a_token_and_the_token_parses_back() {
    for op in Op::ALL {
        assert_eq!(Op::parse(op.as_str()), Some(op), "{}", op.as_str());
        assert!(!op.as_str().contains('\t'));
    }
    assert_eq!(Op::ALL.len(), 11);
    assert_eq!(Op::parse("ADD"), None, "a token is exact");
    assert_eq!(Op::parse(""), None);
}

#[test]
fn a_lost_trailing_newline_still_decodes() {
    let line = entry().encode();

    assert_eq!(Entry::decode(line.trim_end_matches('\n')), Ok(entry()));
}
