// `el lint`: clippy evidence under the project's own lints. el does
// not inject `-D warnings` here: that strictness belongs to the L2
// gate, and a lint verb that silently enabled restriction lints would
// report failures the project never asked for. A run whose warnings
// exit zero passes with a warnings count; errors fail. Nothing is
// ever fixed: clippy has no fix mode in this verb.

use super::super::{error, Env, Exit, Report};
use super::scope::evidence_scope;
use crate::core::gate::{parse_cargo_json, Level};
use crate::core::text::json::Json;
use std::time::Instant;

pub fn lint(env: &Env<'_>, scope: Option<&str>) -> Report {
    if !env.fs.exists("Cargo.toml").unwrap_or(false) {
        return Report::fail(
            "lint",
            error::Error {
                code: "env.not_a_project",
                exit: Exit::Env,
                message: format!("{} has no Cargo.toml", env.root),
            },
        );
    }
    let argv = vec![
        "cargo".to_string(),
        "clippy".to_string(),
        "--all-targets".to_string(),
        "--message-format=json".to_string(),
    ];
    let started = Instant::now();
    let finished = match env.tools.run_timed(&argv, None, env.cargo_timeout) {
        Ok(finished) => finished,
        Err(failure) => {
            return Report::fail(
                "lint",
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
            "lint",
            error::Error {
                code: "env.timeout",
                exit: Exit::Env,
                message: format!("cargo clippy timed out after {}s", env.cargo_timeout),
            },
        );
    }
    let ms = started.elapsed().as_millis() as u64;
    let parsed = parse_cargo_json(&finished.stdout, &|_, _| None);
    let errors = parsed.diagnostics.iter().filter(|diag| diag.level == Level::Error).count();
    let warnings = parsed.diagnostics.iter().filter(|diag| diag.level == Level::Warning).count();
    let result = if errors == 0 { "passed" } else { "failed" };
    let mut report = if errors == 0 {
        Report::new("lint")
    } else {
        Report::fail(
            "lint",
            error::Error {
                code: "gate.failed",
                exit: Exit::Gate,
                message: format!("lint found {errors} error{}", if errors == 1 { "" } else { "s" }),
            },
        )
    };
    report.lines = parsed
        .diagnostics
        .iter()
        .map(|diag| format!("{} {}:{} {}", diag.code, diag.path, diag.line, diag.message))
        .collect();
    report.notes = vec![format!("el: lint {result} ({errors} errors, {warnings} warnings, {ms} ms)")];
    report.payload = Some(Json::Object(vec![
        ("lint".to_string(), Json::Str(result.to_string())),
        ("check".to_string(), Json::Str("not_run".to_string())),
        ("test".to_string(), Json::Str("not_run".to_string())),
        ("fmt".to_string(), Json::Str("not_run".to_string())),
        ("errors".to_string(), Json::Number(errors.to_string())),
        ("warnings".to_string(), Json::Number(warnings.to_string())),
        (
            "diagnostics".to_string(),
            Json::Array(
                parsed
                    .diagnostics
                    .iter()
                    .take(50)
                    .map(|diag| {
                        Json::Object(vec![
                            ("code".to_string(), Json::Str(diag.code.clone())),
                            ("level".to_string(), Json::Str(diag.level.as_str().to_string())),
                            ("path".to_string(), Json::Str(diag.path.clone())),
                            ("line".to_string(), Json::Number(diag.line.to_string())),
                            ("message".to_string(), Json::Str(diag.message.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "non_json_lines".to_string(),
            Json::Array(
                parsed
                    .non_json
                    .iter()
                    .take(20)
                    .map(|line| Json::Str(line.clone()))
                    .collect(),
            ),
        ),
    ]));
    report.evidence = Some(Json::Object(vec![
        ("kind".to_string(), Json::Str("lint".to_string())),
        (
            "command".to_string(),
            Json::Array(argv.iter().map(|arg| Json::Str(arg.clone())).collect()),
        ),
        ("scope".to_string(), evidence_scope(env, scope, &["all-targets"], &argv)),
        ("exit_code".to_string(), Json::Number(finished.code.to_string())),
        ("elapsed_ms".to_string(), Json::Number(ms.to_string())),
        ("result".to_string(), Json::Str(result.to_string())),
    ]));
    report
}
