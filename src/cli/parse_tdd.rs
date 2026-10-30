// The `tdd` verb's argument shape: a phase word, with `--target`
// only on `start`. It lives apart from `parse` for the same reason
// the other rich verbs do: the dispatch table answers which verb,
// this answers what its arguments mean.

use super::args::{Args, Usage};
use super::request::Command;

pub fn tdd(args: &Args) -> Result<Command, Usage> {
    let phase = args.required("tdd")?;
    if !matches!(
        phase.as_str(),
        "start" | "red" | "green" | "impacted" | "all" | "verify" | "abandon"
    ) {
        return Err(Usage::new(
            "usage.bad_value",
            format!(
                "{phase:?} is not a tdd phase; el knows start red green impacted all verify abandon"
            ),
        ));
    }
    let target = args.value("--target")?;
    if phase == "start" && target.is_none() {
        return Err(Usage::new(
            "usage.missing_argument",
            "tdd start needs --target <test-id>",
        ));
    }
    if phase != "start" && target.is_some() {
        return Err(Usage::new(
            "usage.conflict",
            "only tdd start takes --target; the run already names its target",
        ));
    }
    Ok(Command::Tdd { phase, target })
}
