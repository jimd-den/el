use std::process::Command;

#[test]
fn no_source_file_exceeds_ceiling() {
    let status = Command::new("python3")
        .args(["tools/check_line_budget.py"])
        .status()
        .expect("run checker");
    assert!(status.success(), "line budget check failed");
}
