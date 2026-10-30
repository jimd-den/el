// Enforced TDD state machine for the fork: a strict run moves
// start -> red -> green -> impacted -> all -> verify, and every
// transition checks the element-revision locks captured at start.
// Locks are element revisions (see fork-tdd-design.md §1), because el
// renders tests into the same files as production code.
// What this module deliberately does not do: run cargo, touch the
// catalog, or parse CLI input — those live in the verb layer.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Start,
    Red,
    Green,
    Impacted,
    All,
    Verify,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub phase: Phase,
    pub mode: String,
    pub target: String,
    pub test_lock: Vec<(String, String)>,
    pub prod_baseline: Vec<(String, String)>,
    pub config_lock: Vec<(String, String)>,
}

impl Record {
    pub fn new() -> Record {
        Record::default()
    }
}

impl Default for Record {
    fn default() -> Record {
        Record {
            phase: Phase::Start,
            mode: "strict".to_string(),
            target: String::new(),
            test_lock: Vec::new(),
            prod_baseline: Vec::new(),
            config_lock: Vec::new(),
        }
    }
}

/// Move a run to the next phase, enforcing the strict order.
/// Only forward, one-step transitions are allowed; anything else is
/// refused with the plan's phase code.
pub fn transition(record: &Record, next: Phase) -> Result<Record, &'static str> {
    let allowed = matches!(
        (record.phase, next),
        (Phase::Start, Phase::Red)
            | (Phase::Red, Phase::Green)
            | (Phase::Green, Phase::Impacted)
            | (Phase::Impacted, Phase::All)
            | (Phase::All, Phase::Verify)
    );
    if !allowed {
        return Err("EL_TDD_PHASE");
    }
    let mut moved = record.clone();
    moved.phase = next;
    Ok(moved)
}

/// A locked set is frozen: same ids, same revs, no additions, no
/// deletions. Any difference is refused with the caller's code.
pub fn verify_frozen(
    locked: &[(String, String)],
    current: &[(String, String)],
    code: &'static str,
) -> Result<(), &'static str> {
    let mut locked_sorted = locked.to_vec();
    let mut current_sorted = current.to_vec();
    locked_sorted.sort();
    current_sorted.sort();
    if locked_sorted == current_sorted {
        Ok(())
    } else {
        Err(code)
    }
}

/// Weakening markers in test source: `#[ignore]` (and, by the same
/// rule, skip-style annotations) mean the test no longer proves what
/// it proved at red.
pub fn weakened(source: &str) -> bool {
    source.contains("#[ignore]")
}

/// What a `cargo test` run for the target test actually proved.
/// Only `Assertion` counts as red: a setup failure, a timeout, or an
/// already-passing test is an invalid red, never evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedVerdict {
    Assertion,
    Setup,
    Passing,
}

/// Classify `cargo test` output for one target test. A timeout is
/// never red, even if the partial output looks like a failure. A
/// failure only counts when it is assertion-shaped: the target's
/// FAILED line plus a panic/assertion trace, with no compile errors.
pub fn classify_red(output: &str, target: &str, timed_out: bool) -> RedVerdict {
    if timed_out {
        return RedVerdict::Setup;
    }
    if output.contains("could not compile") || output.contains("error[") {
        return RedVerdict::Setup;
    }
    let failed_line = format!("test {} ... FAILED", target);
    if output.contains(&failed_line) && (output.contains("panicked") || output.contains("assertion")) {
        return RedVerdict::Assertion;
    }
    let ok_line = format!("test {} ... ok", target);
    if output.contains(&ok_line) {
        return RedVerdict::Passing;
    }
    RedVerdict::Setup
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_before_red_is_refused() {
        let record = Record::new();

        let moved = transition(&record, Phase::Green);

        assert!(moved.is_err(), "Start -> Green must be refused");
    }

    #[test]
    fn green_refuses_changed_test_rev() {
        let locked = vec![("el-0001".to_string(), "sha256:aaa".to_string())];
        let current = vec![("el-0001".to_string(), "sha256:bbb".to_string())];

        let checked = verify_frozen(&locked, &current, "EL_TEST_LOCKED");

        assert!(checked.is_err(), "a changed test rev must be refused");
    }

    #[test]
    fn weakening_marker_is_detected() {
        let source = "#[test]\n#[ignore]\nfn skipped() {}";

        assert!(weakened(source), "#[ignore] is a weakening marker");
    }

    #[test]
    fn assertion_failure_is_red() {
        let output = "test answer_returns_42 ... FAILED\npanicked at src/lib.rs:3:5:\nassertion failed: answer() == 42";

        assert_eq!(classify_red(output, "answer_returns_42", false), RedVerdict::Assertion);
    }

    #[test]
    fn compile_error_is_not_red() {
        let output = "error[E0308]: mismatched types\nerror: could not compile `hello`";

        assert_eq!(classify_red(output, "answer_returns_42", false), RedVerdict::Setup);
    }

    #[test]
    fn timeout_is_not_red() {
        let output = "test answer_returns_42 ... FAILED";

        assert_eq!(classify_red(output, "answer_returns_42", true), RedVerdict::Setup);
    }

    #[test]
    fn passing_test_is_not_red() {
        let output = "test answer_returns_42 ... ok";

        assert_eq!(classify_red(output, "answer_returns_42", false), RedVerdict::Passing);
    }
}
