// Parsing `cargo test` output into execution evidence: per-test results
// from the `test ... ok` / `test ... FAILED` lines, and passed/failed
// counts summed across the `test result:` summaries (one per target).
// Counts come from the summaries, never from the per-test lines, so a
// target that runs no tests still reports its zeros honestly.

use super::super::error::Error;
use super::super::{Exit, Report};
use crate::core::text::json::Json;

/// One executed test, as cargo printed it.
struct Run {
    name: String,
    result: &'static str,
}

fn summarize(stdout: &str) -> (u64, u64, Vec<Run>) {
    let mut passed = 0u64;
    let mut failed = 0u64;
    let mut tests = Vec::new();
    let mut saw_summary = false;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("test result: ") {
            saw_summary = true;
            passed += count(rest, "passed");
            failed += count(rest, "failed");
            continue;
        }
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        if let Some(name) = rest.strip_suffix(" ... ok") {
            tests.push(Run { name: name.to_string(), result: "ok" });
        } else if let Some(name) = rest.strip_suffix(" ... FAILED") {
            tests.push(Run { name: name.to_string(), result: "failed" });
        }
    }
    if !saw_summary {
        return (0, 0, tests);
    }
    (passed, failed, tests)
}

fn count(summary: &str, word: &str) -> u64 {
    let tokens: Vec<&str> = summary.split_whitespace().collect();
    tokens
        .windows(2)
        .find_map(|pair| {
            if pair[1].trim_end_matches(';') == word {
                pair[0].parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

pub(super) fn report(stdout: &str, code: i32, ms: u64) -> Report {
    let (passed, failed, tests) = summarize(stdout);
    let lines: Vec<String> = tests
        .iter()
        .map(|test| format!("{} ... {}", test.name, test.result))
        .collect();
    let payload = Json::Object(vec![
        ("passed".to_string(), Json::Number(passed.to_string())),
        ("failed".to_string(), Json::Number(failed.to_string())),
        (
            "tests".to_string(),
            Json::Array(
                tests
                    .iter()
                    .map(|test| {
                        Json::Object(vec![
                            ("name".to_string(), Json::Str(test.name.clone())),
                            ("result".to_string(), Json::Str(test.result.to_string())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]);
    if code == 0 {
        let mut report = Report::new("test");
        report.notes = vec![format!("el: test ok ({passed} passed, {failed} failed, {ms} ms)")];
        report.lines = lines;
        report.payload = Some(payload);
        return report;
    }
    if failed == 0 && !stdout.contains("test result:") {
        let first = stdout.lines().next().unwrap_or("").trim();
        let mut report = Report::fail(
            "test",
            Error {
                code: "test.compile_failed",
                exit: Exit::Gate,
                message: if first.is_empty() {
                    format!("cargo test exited {code} without running tests")
                } else {
                    format!("cargo test exited {code} without running tests: {first}")
                },
            },
        );
        report.payload = Some(payload);
        return report;
    }
    let mut report = Report::fail(
        "test",
        Error {
            code: "test.failed",
            exit: Exit::Gate,
            message: format!("test failed ({passed} passed, {failed} failed)"),
        },
    );
    report.notes = vec![format!("el: test failed ({passed} passed, {failed} failed, {ms} ms)")];
    report.lines = lines;
    report.payload = Some(payload);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_sum_across_targets_and_failures_read_failed() {
        let stdout = "test one ... ok\ntest two ... FAILED\n\
            test result: ok. 1 passed; 0 failed;\n\
            test result: FAILED. 0 passed; 1 failed;\n";
        let (passed, failed, tests) = summarize(stdout);

        assert_eq!((passed, failed), (1, 1));
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[1].result, "failed");
    }

    #[test]
    fn no_summary_means_nothing_ran() {
        let (passed, failed, tests) = summarize("Compiling foo\n");

        assert_eq!((passed, failed), (0, 0));
        assert!(tests.is_empty());
    }
}
