// `el test`: execute the crate's tests and report execution, not compilation.
//
// This is on-demand evidence, not a gate tier: it never refuses a commit and
// never changes what the gate checks. A failing test is a test failure
// (exit 3, code `test.failed`), distinct from a compile error, because the
// command ran and the tests ran — the code under test is what failed. A
// suite that does not compile is reported separately (`test.compile_failed`,
// next: `el check`), so a compile error can never masquerade as behavior
// evidence. Missing cargo is an environment failure (`EL_ENV_UNAVAILABLE`),
// never a test verdict.
//
// `--target <id>` runs one test element by id, filtered by its bare name
// (a substring filter, so it matches in any module); `--all`, or no flag,
// runs the suite. Both together are refused. Like `run`, a drifted catalog
// is refused first, so executed tests are never silently misattributed to
// catalog elements they no longer match.

use super::super::error::{self, Error};
use super::super::{load, projection, Env, Exit, Report};
use super::scope::evidence_scope;
use crate::core::model::kind::Kind;
use crate::core::text::json::Json;
use std::time::Instant;

pub fn test(
    env: &Env<'_>,
    target: Option<&str>,
    all: bool,
    scope: Option<&str>,
    impacted: bool,
) -> Report {
    if target.is_some() && all {
        return Report::fail(
            "test",
            Error {
                code: "usage.conflict",
                exit: Exit::Usage,
                message: "--target and --all select different runs; give one".to_string(),
            },
        );
    }
    if target.is_some() && scope.is_some() {
        return Report::fail(
            "test",
            Error {
                code: "usage.conflict",
                exit: Exit::Usage,
                message: "--target and --scope select different filters; give one".to_string(),
            },
        );
    }
    if !env.fs.exists("Cargo.toml").unwrap_or(false) {
        return Report::fail(
            "test",
            Error {
                code: "env.not_a_project",
                exit: Exit::Env,
                message: format!("{} has no Cargo.toml", env.root),
            },
        );
    }
    let filter = match target {
        Some(text) => match filter_for(env, text) {
            Ok(name) => Some(name),
            Err(error) => return Report::fail("test", error),
        },
        None => scope.map(str::to_string),
    };
    if env.fs.exists(".el/version").unwrap_or(false) {
        let catalog = match load::catalog(env) {
            Ok(catalog) => catalog,
            Err(error) => return Report::fail("test", error),
        };
        if let Err(error) = projection::check_drift(env, &catalog, &env.crate_name) {
            return Report::fail("test", error);
        }
    }
    let mut argv = vec!["cargo".to_string(), "test".to_string()];
    if let Some(name) = &filter {
        argv.push("--".to_string());
        argv.push(name.clone());
    }
    let started = Instant::now();
    let finished = match env.tools.run_timed(&argv, None, env.cargo_timeout) {
        Ok(finished) => finished,
        Err(failure) => {
            if failure.missing {
                return Report::fail("test", error::unavailable(failure.to_string()));
            }
            return Report::fail(
                "test",
                Error {
                    code: "env.tool_failed",
                    exit: Exit::Env,
                    message: failure.to_string(),
                },
            );
        }
    };
    if finished.timed_out {
        return Report::fail(
            "test",
            Error {
                code: "env.timeout",
                exit: Exit::Env,
                message: format!("cargo test timed out after {}s", env.cargo_timeout),
            },
        );
    }
    let ms = started.elapsed().as_millis() as u64;
    let mut report = super::test_report::report(&finished.stdout, finished.code, ms);
    let result = if report.error.is_some() { "failed" } else { "passed" };
    let mut payload = report.payload.take().unwrap_or(Json::Object(vec![]));
    if let Json::Object(fields) = &mut payload {
        fields.push(("test".to_string(), Json::Str(result.to_string())));
        fields.push(("check".to_string(), Json::Str("not_run".to_string())));
        fields.push(("fmt".to_string(), Json::Str("not_run".to_string())));
        fields.push(("lint".to_string(), Json::Str("not_run".to_string())));
        if impacted {
            fields.push(("fallback".to_string(), Json::Str("all".to_string())));
        }
    }
    report.payload = Some(payload);
    report.evidence = Some(Json::Object(vec![
        ("kind".to_string(), Json::Str("test".to_string())),
        (
            "command".to_string(),
            Json::Array(argv.iter().map(|arg| Json::Str(arg.clone())).collect()),
        ),
        ("scope".to_string(), evidence_scope(env, scope, &["default"], &argv)),
        ("exit_code".to_string(), Json::Number(finished.code.to_string())),
        ("elapsed_ms".to_string(), Json::Number(ms.to_string())),
        ("result".to_string(), Json::Str(result.to_string())),
    ]));
    report
}

/// The bare test name for a `--target` id, from the catalog.
pub(super) fn filter_for(env: &Env<'_>, text: &str) -> Result<String, Error> {
    let id = super::super::change::parse_id(text)?;
    let catalog = if env.fs.exists(".el/version").unwrap_or(false) {
        load::catalog(env)?
    } else {
        crate::core::model::catalog::Catalog::new()
    };
    let Some(element) = catalog.find(id) else {
        return Err(Error {
            code: "notfound.element",
            exit: Exit::NotFound,
            message: format!("{id} does not exist"),
        });
    };
    if element.kind != Kind::Test {
        return Err(Error {
            code: "usage.bad_value",
            exit: Exit::Usage,
            message: format!("{} is not a test; --target takes a test id", id),
        });
    }
    let name = element.name.clone().or_else(|| {
        element
            .alias
            .as_ref()
            .map(|alias| alias.as_str().rsplit('/').next().unwrap_or("").to_string())
    });
    match name.filter(|name| !name.is_empty()) {
        Some(name) => Ok(name),
        None => Err(Error {
            code: "notfound.element",
            exit: Exit::NotFound,
            message: format!("{id} has no runnable name"),
        }),
    }
}

