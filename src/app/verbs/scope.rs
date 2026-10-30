// Scope evidence: what a runner verb ran under, recorded in the
// envelope's `evidence.scope`. A scope is a selector the caller names;
// `check` and `lint` run crate-wide whatever it names (cargo cannot
// limit rustc to one module), so the scope is recorded, not used to
// filter the verdict — filtering the diagnostics while keeping the
// crate-wide verdict would make the two disagree. `test` is different:
// cargo's own filter limits execution, so a test scope does narrow the
// run. A workspace is out of el's model, so the package list is the
// one crate; the toolchain and triple come from one `rustc -vV` probe
// and read `unknown` when the probe fails.

use super::super::Env;
use crate::core::text::json::Json;

pub fn evidence_scope(
    env: &Env<'_>,
    requested: Option<&str>,
    targets: &[&str],
    command: &[String],
) -> Json {
    let (toolchain, triple) = rustc(env);
    Json::Object(vec![
        ("manifest".to_string(), Json::Str("Cargo.toml".to_string())),
        ("packages".to_string(), Json::Array(vec![Json::Str(env.crate_name.clone())])),
        (
            "targets".to_string(),
            Json::Array(targets.iter().map(|target| Json::Str(target.to_string())).collect()),
        ),
        ("features".to_string(), Json::Array(vec![])),
        ("toolchain".to_string(), Json::Str(toolchain)),
        ("target_triple".to_string(), Json::Str(triple)),
        ("requested".to_string(), Json::Str(requested.unwrap_or("all").to_string())),
        (
            "command".to_string(),
            Json::Array(command.iter().map(|arg| Json::Str(arg.clone())).collect()),
        ),
    ])
}

fn rustc(env: &Env<'_>) -> (String, String) {
    let finished = env.tools.run_timed(&["rustc".to_string(), "-vV".to_string()], None, 5);
    let Ok(finished) = finished else {
        return ("unknown".to_string(), "unknown".to_string());
    };
    if finished.code != 0 || finished.timed_out {
        return ("unknown".to_string(), "unknown".to_string());
    }
    let mut toolchain = "unknown".to_string();
    let mut triple = "unknown".to_string();
    for line in finished.stdout.lines() {
        if let Some(value) = line.strip_prefix("release: ") {
            toolchain = value.trim().to_string();
        }
        if let Some(value) = line.strip_prefix("host: ") {
            triple = value.trim().to_string();
        }
    }
    (toolchain, triple)
}
