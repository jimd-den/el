// The parser tests for the read verbs, kept beside `parse_tests` so neither file
// crosses the line ceiling. They repeat the two-line argv helpers rather than
// making test helpers public across modules: the duplication is smaller than the
// coupling it would buy.

use super::*;
use crate::core::model::kind::Kind;

fn parsed(line: &str) -> Parsed {
    let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    parse(&argv).unwrap_or_else(|error| panic!("{line:?}: {error}"))
}

fn refused(line: &str) -> Usage {
    let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    parse(&argv).expect_err(&format!("{line:?} was accepted"))
}

#[test]
fn ls_reads_its_filters() {
    let Command::Ls(ls) = parsed("ls --module src/greet.rs --kind fn --opaque").command else {
        panic!("not an ls");
    };

    assert_eq!(ls.module.as_deref(), Some("src/greet.rs"));
    assert_eq!(ls.kind, Some(Kind::Function));
    assert!(ls.opaque);
}

#[test]
fn tree_reads_statements() {
    let Command::Tree(tree) = parsed("tree --statements").command else {
        panic!("not a tree");
    };

    assert!(tree.statements);
}

#[test]
fn show_reads_its_id_and_optional_views() {
    let Command::Show(show) = parsed("show el-0005 --statements --source --history").command else {
        panic!("not a show");
    };

    assert_eq!(show.id, "el-0005");
    assert!(show.statements && show.source && show.history);
}

#[test]
fn show_reads_terse() {
    let Command::Show(show) = parsed("show el-0005 --terse").command else {
        panic!("not a show");
    };

    assert!(show.terse);
    assert!(!show.statements && !show.source && !show.history);
}

#[test]
fn ls_refuses_an_unknown_kind_and_lists_the_ones_that_exist() {
    let refusal = refused("ls --kind widget");

    assert_eq!(refusal.code, "usage.unknown_kind");
    assert!(refusal.message.contains("fn"), "{refusal}");
}


#[test]
fn an_unknown_flag_is_a_usage_error() {
    assert_eq!(refused("ls --bogus").code, "usage.unknown_flag");
}
