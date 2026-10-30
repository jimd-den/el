// `el status`: the preflight. Root, crate, catalog presence, journal
// head, policy, and tool availability — and the four component statuses,
// all `not_run`: status itself runs nothing, and it never claims tests
// passed merely because they have not run. It works on a plain Cargo
// crate too, where the catalog is absent and the head is null.

use super::super::{error, load, Env, Report};
use crate::core::text::json::Json;

pub fn status(env: &Env<'_>) -> Report {
    let catalog = env.fs.exists(".el/version").unwrap_or(false);
    let head_seq = if catalog {
        match env.journal.entries() {
            Ok(entries) => entries.iter().map(|entry| entry.seq).max(),
            Err(error) => return Report::fail("status", error::store(error)),
        }
    } else {
        None
    };
    let policy = match load::policy(env) {
        Ok(policy) => policy,
        Err(error) => return Report::fail("status", error),
    };
    let cargo = match env.tools.run_timed(
        &["cargo".to_string(), "--version".to_string()],
        None,
        5,
    ) {
        Ok(finished) if finished.code == 0 && !finished.timed_out => "present",
        _ => "absent",
    };
    let head = head_seq.map(|seq| Json::Number(seq.to_string()));
    let mut report = Report::new("status");
    report.lines = vec![
        format!("root {}", env.root),
        format!("crate {}", env.crate_name),
        format!("catalog {}", if catalog { "yes" } else { "no" }),
        format!(
            "head {}",
            head_seq.map(|seq| seq.to_string()).unwrap_or_else(|| "none".to_string())
        ),
        format!("policy gate={} split={}", policy.gate, policy.split.as_str()),
        format!("cargo {cargo}"),
        "active run none".to_string(),
    ];
    report.payload = Some(Json::Object(vec![
        ("root".to_string(), Json::Str(env.root.clone())),
        ("crate".to_string(), Json::Str(env.crate_name.clone())),
        ("catalog".to_string(), Json::Bool(catalog)),
        ("head_seq".to_string(), head.unwrap_or(Json::Null)),
        (
            "policy".to_string(),
            Json::Object(vec![
                ("gate".to_string(), Json::Str(policy.gate.clone())),
                ("split".to_string(), Json::Str(policy.split.as_str().to_string())),
            ]),
        ),
        ("cargo".to_string(), Json::Str(cargo.to_string())),
        ("active_run".to_string(), Json::Null),
        ("check".to_string(), Json::Str("not_run".to_string())),
        ("fmt".to_string(), Json::Str("not_run".to_string())),
        ("lint".to_string(), Json::Str("not_run".to_string())),
        ("test".to_string(), Json::Str("not_run".to_string())),
    ]));
    report
}
