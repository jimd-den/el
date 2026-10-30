// `el fmt --check`: non-mutating format evidence. It runs the
// configured `cargo fmt --all -- --check` and reports the diff; it
// never writes, and there is no `--fix` — formatting writes need a
// distinct, previewed action, and this verb is not it.

use super::super::{error, Env, Exit, Report};
use super::scope::evidence_scope;
use crate::core::text::json::Json;
use std::time::Instant;

pub fn fmt(env: &Env<'_>) -> Report {
    if !env.fs.exists("Cargo.toml").unwrap_or(false) {
        return Report::fail(
            "fmt",
            error::Error {
                code: "env.not_a_project",
                exit: Exit::Env,
                message: format!("{} has no Cargo.toml", env.root),
            },
        );
    }
    let argv = vec![
        "cargo".to_string(),
        "fmt".to_string(),
        "--all".to_string(),
        "--".to_string(),
        "--check".to_string(),
    ];
    let started = Instant::now();
    let finished = match env.tools.run_timed(&argv, None, env.cargo_timeout) {
        Ok(finished) => finished,
        Err(failure) => {
            return Report::fail(
                "fmt",
                if failure.missing {
                    error::unavailable(failure.to_string())
                } else {
                    error::Error {
                        code: "env.tool_failed",
                        exit: Exit::Env,
                        message: failure.to_string(),
                    }
                },
            );
        }
    };
    if finished.timed_out {
        return Report::fail(
            "fmt",
            error::Error {
                code: "env.timeout",
                exit: Exit::Env,
                message: format!("cargo fmt timed out after {}s", env.cargo_timeout),
            },
        );
    }
    let ms = started.elapsed().as_millis() as u64;
    let result = if finished.code == 0 { "passed" } else { "failed" };
    let diff: Vec<String> = finished.stdout.lines().take(20).map(str::to_string).collect();
    let mut report = if finished.code == 0 {
        Report::new("fmt")
    } else {
        Report::fail(
            "fmt",
            error::Error {
                code: "gate.failed",
                exit: Exit::Gate,
                message: "fmt --check found differences".to_string(),
            },
        )
    };
    report.lines = diff.clone();
    report.notes = vec![format!("el: fmt {result} ({ms} ms)")];
    report.payload = Some(Json::Object(vec![
        ("fmt".to_string(), Json::Str(result.to_string())),
        ("check".to_string(), Json::Str("not_run".to_string())),
        ("test".to_string(), Json::Str("not_run".to_string())),
        ("lint".to_string(), Json::Str("not_run".to_string())),
        (
            "diff".to_string(),
            Json::Array(diff.iter().map(|line| Json::Str(line.clone())).collect()),
        ),
    ]));
    report.evidence = Some(Json::Object(vec![
        ("kind".to_string(), Json::Str("fmt".to_string())),
        (
            "command".to_string(),
            Json::Array(argv.iter().map(|arg| Json::Str(arg.clone())).collect()),
        ),
        ("scope".to_string(), evidence_scope(env, None, &["all"], &argv)),
        ("exit_code".to_string(), Json::Number(finished.code.to_string())),
        ("elapsed_ms".to_string(), Json::Number(ms.to_string())),
        ("result".to_string(), Json::Str(result.to_string())),
    ]));
    report
}
