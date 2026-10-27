// Tests for `add` and `edit`; sibling of the module they exercise (§4.4).
//
// `edit` is here because `--body-mode` is a command-line override of the project
// policy: the parser must preserve which one the user asked for, and refuse a
// word that is neither.

use super::*;
use crate::cli::request::Command;
use crate::cli::parse::parse;
use crate::cli::request::Parsed;

fn parsed(line: &str) -> Parsed {
    let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    parse(&argv).unwrap_or_else(|error| panic!("{line:?}: {error}"))
}

fn refused(line: &str) -> Usage {
    let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    parse(&argv).expect_err(&format!("{line:?} was accepted"))
}

#[test]
fn edit_reads_a_rename_as_one_value() {
    let Command::Edit(edit) = parsed("edit el-0007 --rename apply_change").command else {
        panic!("not an edit");
    };

    assert_eq!(edit.id, "el-0007");
    assert_eq!(edit.rename.as_deref(), Some("apply_change"));
    assert_eq!(edit.mode, None);
}

#[test]
fn body_mode_is_parsed_to_a_split() {
    let Command::Edit(edit) = parsed("edit el-0007 --body - --body-mode opaque").command else {
        panic!("not an edit");
    };

    assert_eq!(edit.mode, Some(Split::Never));
}

#[test]
fn body_mode_refuses_an_unknown_value() {
    let refusal = refused("edit el-0007 --body - --body-mode sometimes");

    assert_eq!(refusal.code, "usage.bad_value");
    assert!(refusal.message.contains("split"), "{refusal}");
}
