// The gate tiers M6 can honour. The default is L0 (`model`): the plan already ran
// it, and M9 owns L1. `--strict` adds the lint the user asked for, which is the
// cheapest way to keep generated code clean before the compiler tier exists.

use super::super::{error, Env, Exit};

pub fn gate(env: &Env<'_>, strict: bool) -> Result<bool, error::Error> {
    if !strict {
        return Ok(true);
    }
    let argv = ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"].map(str::to_string);
    let finished = env.tools.run(&argv, None).map_err(|failure| error::Error {
        code: "env.missing_tool",
        exit: Exit::Env,
        message: failure.to_string(),
    })?;
    Ok(finished.code == 0)
}
