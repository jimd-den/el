// The whole surface on one page: what an agent reads instead of the source, and
// what `--help` prints. The list is written here once and `skill_is_the_surface`
// fails when a verb or a flag the parser dispatches is missing from it.
//
// It deliberately does not describe behaviour in depth: the contract is
// `docs/COMMANDS.md`, and a second copy of it here would drift. One line per verb,
// one line per flag, the exit codes, and the envelope.

use std::fmt::Write;

/// The ceiling `COMMANDS.md` §4.7 publishes for `el skill`.
pub const MAX_LINES: usize = 150;

/// One page, no crate needed.
pub fn guide() -> String {
    let mut out = String::new();
    out.push_str("el — build Rust by element\n\n");
    out.push_str("usage: el <verb> [subject] [flags]\n\n");
    out.push_str("verbs\n");
    for (group, names) in VERB_GROUPS {
        let _ = writeln!(out, "{group}");
        for (name, usage, what) in VERB_LINES {
            if names.contains(name) {
                let _ = writeln!(out, "  {name:<8} {usage:<42} {what}");
            }
        }
    }
    out.push_str("\nkinds\n  mod fn struct enum trait impl const static type test stmt\n");
    out.push_str("\ntest loop\n");
    out.push_str("  add test <name> --in <module> --sig 'fn <name>()' --body -   a #[test] fn\n");
    out.push_str("  cargo test fails before the implementation, passes after. el check does not run tests.\n");
    out.push_str("  echo 'assert_eq!(answer(), 42);' | el add test answer_returns_42 --in util --body -\n");
    out.push_str("  the body is assertions only: no #[test] line, no mod tests wrapper, no --sig needed\n");
    out.push_str("\nplain words\n");
    out.push_str("  add fn greet --takes 'name: &str' --returns String --body -   no signature syntax\n");
    out.push_str("  add const LIMIT --type usize --value 10   the tool cannot invent a type, so say it\n");
    out.push_str("  either --sig or the plain words, not both\n");
    out.push_str("\nstatements\n");
    out.push_str("  add stmt <fn-id> --body - --at 0   the subject is the owning function; a statement has no name\n");
    out.push_str("\napply example\n");
    out.push_str("  echo '1' > /tmp/one.txt\n");
    out.push_str("  printf 'add mod extra --in src\\nadd fn one --in src/extra --vis pub --returns u32 --body /tmp/one.txt\\n' > /tmp/changes.el\n");
    out.push_str("  el apply /tmp/changes.el\n");
    out.push_str("  in a script, --body takes a file, never a literal and never -; scripts cannot read stdin\n");
    out.push_str("\nuniversal flags\n");
    for (flag, what) in FLAG_LINES {
        let _ = writeln!(out, "  {flag:<14} {what}");
    }
    out.push_str("\nexit codes\n");
    out.push_str("  0 ok  1 usage  2 violation  3 gate  4 environment  5 conflict\n");
    out.push_str("  6 not found  7 internal\n");
    out.push_str("\njson\n  one object on stdout; --json puts prose on stderr.\n");
    out.push_str("  keys: el cmd ok error{code,exit,message,hint} seq changed[] gate{} data?\n");
    out.push_str("  gate diagnostics carry first_actionable: exactly one is true.\n");
    out.push_str("\nnot built: refs catalog build fix search doc\n");
    out
}

/// Verbs by the task an agent is doing, in the order the page prints them.
/// A flat catalog past ~15 tools is where selection accuracy falls, so the
/// page groups by intent — the static form of scoping the toolset per turn.
/// The test beside this table refuses a verb with no group or two groups.
const VERB_GROUPS: &[(&str, &[&str])] = &[
    ("write", &["new", "init", "add", "batch", "apply", "edit", "rm", "mv"]),
    ("read", &["ls", "tree", "show", "find", "explain", "diff", "log"]),
    ("history", &["undo", "redo", "revert"]),
    ("verify", &["run", "check", "test", "doctor"]),
    ("meta", &["version", "skill", "help"]),
];

/// Every verb the parser dispatches, with its one-line shape. Kept beside
/// `parse::VERBS`; the test below refuses drift in either direction.
const VERB_LINES: &[(&str, &str, &str)] = &[
    ("new", "new <path> [--lib] [--policy]", "write a crate"),
    ("init", "init", "adopt an existing crate as opaque modules"),
    ("add", "add <kind> <name> [--in] [--body] …", "add an element or statement"),
    ("batch", "batch", "JSON adds as one transaction"),
    ("apply", "apply [<file>|-]", "many change kinds, one transaction"),
    ("edit", "edit <id> [--rename|--sig|--body|--vis]", "change one element"),
    ("rm", "rm <id>… [--cascade]", "remove elements"),
    ("mv", "mv <id> <module> | mv <stmt> --after <stmt>", "move or reorder"),
    ("run", "run [-- args…]", "cargo run, streams output"),
    ("ls", "ls [--module] [--kind] [--opaque]", "list elements"),
    ("tree", "tree [--statements]", "outline the crate"),
    ("show", "show <id> [--statements|--source|--history|--terse]", "one element"),
    ("find", "find <query>", "ids for a name, no bodies"),
    ("explain", "explain <id>", "preamble, doc, callers, tests, history"),
    ("log", "log [-n <count>] [--attic]", "the journal"),
    ("diff", "diff [<a>] [<b>]", "elements and files between sequences"),
    ("undo", "undo [<n>]", "append an entry restoring an earlier state"),
    ("redo", "redo", "append an entry restoring an undone state"),
    ("revert", "revert <seq>", "append an inverse change"),
    ("doctor", "doctor [--repair|--rebuild]", "crate health, recovery"),
    ("check", "check [--fast|--strict]", "the compiler's report, no refusal"),
    ("test", "test [--target <id>|--all]", "execute tests, report execution"),
    ("version", "version", "the package version"),
    ("skill", "skill", "this page"),
    ("help", "help [<verb>] | --help", "this page"),
];

/// The flags, in the parser's two tables. A flag without a line is a surface an
/// agent cannot discover.
const FLAG_LINES: &[(&str, &str)] = &[
    ("--json", "one JSON object on stdout"),
    ("--quiet", "suppress human notes"),
    ("--dry-run", "run the whole transaction, restore"),
    ("--yes", "accepted everywhere; el never prompts"),
    ("--no-color", "no ANSI"),
    ("--root <path>", "operate on another crate"),
    ("--fast", "L0 only, skip the compiler"),
    ("--strict", "L2: clippy -D warnings"),
    ("--semantic", "L3 (not built)"),
    ("--verbose", "stream tool output"),
    ("--lib", "new: a library crate"),
    ("--policy", "new: write el.policy"),
    ("--force", "init: adopt anyway (not built)"),
    ("--cascade", "rm: remove dangling references too (not built)"),
    ("--attic", "log: include truncated future"),
    ("--repair", "doctor: roll back an interruption"),
    ("--rebuild", "doctor: regenerate from the journal"),
    ("--statements", "show: list the statement tree"),
    ("--source", "show: the committed source slice"),
    ("--history", "show: the last five entries"),
    ("--terse", "show: one header line"),
    ("--opaque", "ls: only opaque modules"),
    ("--in <module>", "which file the element lives in"),
    ("--takes <params>", "fn/test: parameters, as written"),
    ("--returns <type>", "fn/test: the return type"),
    ("--type <type>", "const/static/type: the type"),
    ("--value <expr>", "const/static: the value"),
    ("--after <id>", "insert or reorder after this id"),
    ("--before <id>", "insert before this id"),
    ("--at <n>", "insert at a zero-based sibling index"),
    ("--vis <v>", "pub, pub(crate), or private"),
    ("--sig <text>", "the whole declaration, keyword first"),
    ("--body <file|->", "- reads stdin to EOF"),
    ("--doc <text>", "doc text (not built)"),
    ("--attr <text>", "an outer attribute (not built)"),
    ("--preamble <file|->", "a module's prose"),
    ("--edition <year>", "new: the crate edition"),
    ("--body-mode <m>", "split or opaque"),
    ("--limit <n>", "search: result limit (not built)"),
    ("--module <m>", "ls: narrow to one module"),
    ("--kind <k>", "ls: narrow to one kind"),
    ("--rename <name>", "edit: a whole-identifier rename"),
    ("--target <id>", "test: run one test element"),
    ("--all", "test: run the whole suite"),
    ("-n <count>", "log: how many entries"),
    ("--help", "this page"),
];

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
