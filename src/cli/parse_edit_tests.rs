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

fn parsed_argv(argv: &[&str]) -> Parsed {
    let argv: Vec<String> = argv.iter().map(|word| word.to_string()).collect();
    parse(&argv).unwrap_or_else(|error| panic!("{argv:?}: {error}"))
}

fn refused(line: &str) -> Usage {
    let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    parse(&argv).expect_err(&format!("{line:?} was accepted"))
}

#[test]
fn an_add_carries_every_flag_it_was_given() {
    // Spelled as argv, not split on whitespace: `--sig` takes one shell-quoted
    // word, and `fn_greet(` must not swallow the next flag. The old one-line
    // spelling did exactly that and tested the typo instead of the parser.
    let command = parsed_argv(&[
        "add",
        "fn",
        "greet",
        "--in",
        "src/greet.rs",
        "--vis",
        "pub",
        "--sig",
        "fn_greet(",
        "--body",
        "-",
    ])
    .command;

    let Command::Add(add) = command else {
        panic!("not an add");
    };
    assert_eq!(add.kind, Kind::Function);
    assert_eq!(add.name, "greet");
    assert_eq!(add.module.as_deref(), Some("src/greet.rs"));
    assert_eq!(add.visibility, Some(Visibility::Pub));
    assert_eq!(add.signature.as_deref(), Some("fn_greet("));
    assert_eq!(add.body, Some(Source::Stdin));
}

#[test]
fn add_mod_takes_a_path_as_its_subject() {
    let Command::Add(add) = parsed("add mod src/greet.rs").command else {
        panic!("not an add");
    };

    assert_eq!(add.kind, Kind::Module);
    assert_eq!(add.name, "src/greet.rs");
    assert_eq!(add.module, None);
}

#[test]
fn a_body_can_come_from_a_file_instead_of_stdin() {
    let Command::Add(add) = parsed("add fn f --body body.rs").command else {
        panic!("not an add");
    };

    assert_eq!(add.body, Some(Source::File("body.rs".to_string())));
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
