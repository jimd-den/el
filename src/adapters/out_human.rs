// The human view. Two strings, because stdout and stderr have different jobs
// (`COMMANDS.md` §6): what a script might want on stdout — ids, tables, the one line
// that says what changed — and prose about the change on stderr, where a pipe
// cannot swallow it and a human still sees it.
//
// `--quiet` suppresses stderr and nothing else: an error is still an error, and a
// quiet failure with no message is a support ticket.
//
// It deliberately does not colour anything yet. Colour is a terminal capability
// (`--no-color`, `isatty`), and inventing an escape-code vocabulary before there is
// output worth colouring would be a decision made in the wrong order.

use crate::app::report::Report;

/// What goes to stdout: one line per fact, in the order the command produced them.
pub fn stdout(report: &Report) -> String {
    let mut out = String::new();
    for line in &report.lines {
        out.push_str(line);
        out.push('\n');
    }
    if let Some(error) = &report.error {
        // The id-carrying lines still print on failure: a refusal that changed
        // nothing is not a refusal that did nothing (`el rm` names the ids it kept).
        out.push_str(&format!("el: {}\n", error.message));
    }
    out
}

/// What goes to stderr: prose, warnings, notes.
pub fn stderr(report: &Report, quiet: bool) -> String {
    if quiet {
        return String::new();
    }
    let mut out = String::new();
    for note in &report.notes {
        out.push_str(note);
        out.push('\n');
    }
    out
}
