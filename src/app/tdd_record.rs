// A TDD run record as JSON, for `.el/runs/<run-id>/record`.
// Pure serialization only: the verb layer owns the file, the run id,
// and the parse errors at the boundary. The record text is canonical
// `Json::write` output, so a re-save of an unchanged record is
// byte-identical.

use super::tdd::{Phase, Record};
use crate::core::text::json::Json;

pub fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Start => "start",
        Phase::Red => "red",
        Phase::Green => "green",
        Phase::Impacted => "impacted",
        Phase::All => "all",
        Phase::Verify => "verify",
    }
}

fn lock_json(lock: &[(String, String)]) -> Json {
    Json::Array(
        lock.iter()
            .map(|(id, rev)| {
                Json::Object(vec![
                    ("id".to_string(), Json::Str(id.clone())),
                    ("rev".to_string(), Json::Str(rev.clone())),
                ])
            })
            .collect(),
    )
}

pub fn to_json(record: &Record) -> Json {
    Json::Object(vec![
        ("mode".to_string(), Json::Str(record.mode.clone())),
        ("phase".to_string(), Json::Str(phase_name(record.phase).to_string())),
        ("target".to_string(), Json::Str(record.target.clone())),
        ("test_lock".to_string(), lock_json(&record.test_lock)),
        ("prod_baseline".to_string(), lock_json(&record.prod_baseline)),
        ("config_lock".to_string(), lock_json(&record.config_lock)),
    ])
}

fn phase_of(name: &str) -> Result<Phase, &'static str> {
    match name {
        "start" => Ok(Phase::Start),
        "red" => Ok(Phase::Red),
        "green" => Ok(Phase::Green),
        "impacted" => Ok(Phase::Impacted),
        "all" => Ok(Phase::All),
        "verify" => Ok(Phase::Verify),
        _ => Err("EL_TDD_PHASE"),
    }
}

fn lock_of(value: &Json) -> Result<Vec<(String, String)>, &'static str> {
    let items = value.as_array().ok_or("EL_TDD_PHASE")?;
    let mut lock = Vec::new();
    for item in items {
        let id = item.get("id").and_then(Json::as_str).ok_or("EL_TDD_PHASE")?;
        let rev = item.get("rev").and_then(Json::as_str).ok_or("EL_TDD_PHASE")?;
        lock.push((id.to_string(), rev.to_string()));
    }
    Ok(lock)
}

pub fn from_json(value: &Json) -> Result<Record, &'static str> {
    let mode = value.get("mode").and_then(Json::as_str).ok_or("EL_TDD_PHASE")?;
    let phase = value.get("phase").and_then(Json::as_str).ok_or("EL_TDD_PHASE")?;
    let target = value.get("target").and_then(Json::as_str).ok_or("EL_TDD_PHASE")?;
    Ok(Record {
        phase: phase_of(phase)?,
        mode: mode.to_string(),
        target: target.to_string(),
        test_lock: lock_of(value.get("test_lock").ok_or("EL_TDD_PHASE")?)?,
        prod_baseline: lock_of(value.get("prod_baseline").ok_or("EL_TDD_PHASE")?)?,
        config_lock: lock_of(value.get("config_lock").ok_or("EL_TDD_PHASE")?)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Record {
        Record {
            phase: Phase::Red,
            mode: "strict".to_string(),
            target: "el-0007".to_string(),
            test_lock: vec![("el-0007".to_string(), "sha256:aaa".to_string())],
            prod_baseline: vec![("el-0001".to_string(), "sha256:bbb".to_string())],
            config_lock: vec![("el.policy".to_string(), "sha256:ccc".to_string())],
        }
    }

    #[test]
    fn a_record_survives_a_json_round_trip() {
        let record = sample();

        let back = from_json(&to_json(&record)).expect("round trip");

        assert_eq!(back, record);
    }
}
