// Tests for `out_json`; sibling of the module they exercise (§4.4).
//
// The envelope is a compatibility promise (`COMMANDS.md` §5), so these tests compare
// bytes, not parsed values: a key that moved is a contract change, and a test that
// re-parsed would not notice.

use super::*;
use crate::app::report::Error;
use crate::app::Exit;

fn sample() -> Report {
    Report {
        cmd: "add",
        seq: Some(42),
        changed: vec![Changed {
            op: "add",
            id: "el-0007".to_string(),
            alias: Some("core::plan::apply".to_string()),
        }],
        gate: Some(Gate {
            tier: "model".to_string(),
            new: 0,
            fixed: 1,
            existing: 5,
            verdict: "ok".to_string(),
            ms: 812,
        }),
        lines: vec!["el-0007".to_string()],
        notes: vec!["rust-analyzer 2026-10-13 used for rename".to_string()],
        error: None,
    }
}

#[test]
fn a_success_envelope_is_exactly_the_published_fields_in_order() {
    assert_eq!(
        render(&sample()),
        concat!(
            r#"{"el":"1","cmd":"add","ok":true,"seq":42,"#,
            r#""changed":[{"op":"add","id":"el-0007","alias":"core::plan::apply"}],"#,
            r#""gate":{"tier":"model","new":0,"fixed":1,"existing":5,"verdict":"ok","ms":812},"#,
            r#""notes":["rust-analyzer 2026-10-13 used for rename"]}"#,
            "\n"
        )
    );
}

#[test]
fn a_failure_envelope_carries_the_code_and_the_exit_code_and_no_seq() {
    let report = Report::fail(
        "add",
        Error {
            code: "model.ceiling",
            exit: Exit::Violation,
            message: "src/core/plan.rs would be 204 lines".to_string(),
        },
    );

    let text = render(&report);

    assert!(text.contains(r#""ok":false"#), "{text}");
    assert!(
        text.contains(r#""error":{"code":"model.ceiling","exit":2,"message":"src/core/plan.rs would be 204 lines"}"#),
        "{text}"
    );
    assert!(!text.contains(r#""seq""#), "a refusal has no sequence number: {text}");
    assert!(text.ends_with("}\n"), "the envelope is one line: {text:?}");
}

#[test]
fn an_empty_change_list_is_success_not_silence() {
    let report = Report::new("rm");

    assert_eq!(render(&report), "{\"el\":\"1\",\"cmd\":\"rm\",\"ok\":true,\"changed\":[]}\n");
}

#[test]
fn a_note_with_a_quote_or_a_newline_stays_one_json_line() {
    let mut report = Report::new("edit");
    report.notes.push("said \"no\"\nthen left".to_string());

    let text = render(&report);

    assert_eq!(text.lines().count(), 1, "{text:?}");
    assert!(text.contains(r#"said \"no\"\nthen left"#), "{text}");
}
