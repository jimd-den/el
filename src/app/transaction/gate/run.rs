// Running the tier's tools: which commands, and what a failure means. A
// command that fails with no diagnostics is a tool failure, not a verdict;
// a command that times out is an environment error. Both abort the
// transaction, and neither is ever read as a pass.

use crate::app::error::Error;
use crate::app::{Env, Exit};
use crate::core::gate::{parse_cargo_json, Diagnostic, Tier};
use crate::core::model::catalog::Catalog;
use crate::core::render::Projection;
use crate::ports::tools::Finished;

fn check_argv() -> Vec<String> {
    ["cargo", "check", "--all-targets", "--message-format=json"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn clippy_argv() -> Vec<String> {
    [
        "cargo",
        "clippy",
        "--all-targets",
        "--message-format=json",
        "--",
        "-D",
        "warnings",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn test_argv() -> Vec<String> {
    ["cargo", "test", "--no-run", "--message-format=json"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Runs every command the tier names and unions their diagnostics.
pub(super) fn run_tier(
    env: &Env,
    tier: Tier,
    catalog: &Catalog,
    projection: &Projection,
) -> Result<Vec<Diagnostic>, Error> {
    let mut commands = vec![check_argv()];
    if tier == Tier::Strict {
        commands.push(clippy_argv());
        commands.push(test_argv());
    }
    let mut found = Vec::new();
    for argv in &commands {
        let finished = env
            .tools
            .run_timed(argv, None, env.cargo_timeout)
            .map_err(tool_error)?;
        if finished.timed_out {
            return Err(Error {
                code: "env.timeout",
                exit: Exit::Env,
                message: format!(
                    "cargo {} timed out after {}s; the tree was restored",
                    argv.get(1).map(String::as_str).unwrap_or(""),
                    env.cargo_timeout
                ),
            });
        }
        let mut diags = parse_cargo_json(&finished.stdout, &|path, line| {
            projection.element_at(path, line).map(|el| {
                let owner = catalog.find_stmt(el).map(|stmt| stmt.owner);
                (el, owner)
            })
        });
        if finished.code != 0 && diags.is_empty() {
            return Err(tool_failed(&finished, argv));
        }
        found.append(&mut diags);
    }
    Ok(found)
}

fn tool_error(failure: crate::ports::tools::Error) -> Error {
    if failure.missing {
        Error {
            code: "env.missing_tool",
            exit: Exit::Env,
            message: failure.to_string(),
        }
    } else {
        Error {
            code: "env.tool_failed",
            exit: Exit::Env,
            message: failure.to_string(),
        }
    }
}

fn tool_failed(finished: &Finished, argv: &[String]) -> Error {
    let subcommand = argv.get(1).map(String::as_str).unwrap_or("");
    let first = finished.stderr.lines().next().unwrap_or("").trim();
    let message = if first.is_empty() {
        format!("cargo {subcommand} failed with no diagnostics")
    } else {
        format!("cargo {subcommand} failed: {first}")
    };
    Error { code: "env.tool_failed", exit: Exit::Env, message }
}
