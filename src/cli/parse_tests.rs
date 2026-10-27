// Tests for `parse`; sibling of the module they exercise (§4.4).
//
// Each test states a command line as a user would type it, because the parser's
// only contract is "what a person types means what they meant". The argv is built
// by a helper so a test never has to write `Vec<String>` by hand.

use super::*;

fn parsed(line: &str) -> Parsed {
    let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    parse(&argv).unwrap_or_else(|error| panic!("{line:?}: {error}"))
}

fn refused(line: &str) -> Usage {
    let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    parse(&argv).expect_err(&format!("{line:?} was accepted"))
}

#[test]
fn universal_flags_are_read_wherever_they_appear() {
    let before = parsed("--json --fast add fn f");
    let after = parsed("add fn f --json --fast");

    assert_eq!(before.globals, after.globals);
    assert!(before.globals.json);
    assert!(before.globals.fast);
}

#[test]
fn everything_after_two_dashes_belongs_to_the_program() {
    let Command::Run { args } = parsed("run -- --nocapture --release").command else {
        panic!("not a run");
    };

    assert_eq!(args, ["--nocapture", "--release"]);
}

#[test]
fn rm_takes_many_ids_and_keeps_them_in_order() {
    let Command::Rm { ids, cascade } = parsed("rm el-0007 el-0002 --cascade").command else {
        panic!("not an rm");
    };

    assert_eq!(ids, ["el-0007", "el-0002"]);
    assert!(cascade);
}

#[test]
fn mv_reads_the_subject_then_the_module() {
    let Command::Mv(move_) = parsed("mv el-0004 src/core/gate.rs").command else {
        panic!("not an mv");
    };

    assert_eq!(move_.subject, "el-0004");
    assert_eq!(move_.module.as_deref(), Some("src/core/gate.rs"));
}

#[test]
fn log_reads_a_count() {
    let Command::Log { count, .. } = parsed("log -n 5").command else {
        panic!("not a log");
    };

    assert_eq!(count, Some(5));
}

#[test]
fn a_new_crate_can_be_a_library_with_a_policy_file() {
    let Command::New { path, lib, policy } = parsed("new hello --lib --policy").command else {
        panic!("not a new");
    };

    assert_eq!(path, "hello");
    assert!(lib && policy);
}

#[test]
fn an_unknown_verb_is_a_usage_error_that_names_itself() {
    let refusal = refused("frobnicate el-0001");

    assert_eq!(refusal.code, "usage.unknown_verb");
    assert!(refusal.message.contains("frobnicate"), "{refusal}");
}

#[test]
fn an_unknown_verb_names_only_verbs_that_exist() {
    let refusal = refused("frobnicate");

    assert!(!refusal.message.contains("skill"), "{refusal}");
    assert!(refusal.message.contains("show"), "{refusal}");
}

#[test]
fn every_listed_verb_is_dispatched() {
    let undispatched: Vec<&&str> = VERBS
        .iter()
        .filter(|verb| {
            parse(&[verb.to_string()]).is_err_and(|error| error.code == "usage.unknown_verb")
        })
        .collect();

    assert_eq!(undispatched, Vec::<&&str>::new());
}

#[test]
fn an_unknown_kind_lists_the_ones_that_exist() {
    let refusal = refused("add widget thing");

    assert_eq!(refusal.code, "usage.unknown_kind");
    assert!(refusal.message.contains("fn"), "{refusal}");
}

#[test]
fn a_flag_at_the_end_without_its_value_is_refused() {
    let refusal = refused("add fn f --sig");

    assert_eq!(refusal.code, "usage.missing_value");
    assert!(refusal.message.contains("--sig"), "{refusal}");
}

#[test]
fn the_same_flag_twice_is_refused_rather_than_guessed() {
    let refusal = refused("add fn f --sig fn f --sig fn g");

    assert_eq!(refusal.code, "usage.repeated_flag");
}

#[test]
fn a_stray_word_is_refused_instead_of_ignored() {
    let refusal = refused("add fn f extra");

    assert_eq!(refusal.code, "usage.unexpected_argument");
}

#[test]
fn a_bad_visibility_says_what_it_accepts() {
    let refusal = refused("add fn f --vis private");

    assert_eq!(refusal.code, "usage.bad_visibility");
    assert!(refusal.message.contains("crate"), "{refusal}");
}

#[test]
fn no_verb_at_all_is_a_usage_error() {
    assert_eq!(refused("").code, "usage.missing_verb");
}

#[test]
fn a_negative_number_is_a_value_not_a_flag() {
    let args = Args::split(&["log".to_string(), "-n".to_string(), "-1".to_string()]).expect("split");

    assert_eq!(args.value("-n").expect("value"), Some("-1".to_string()));
}
