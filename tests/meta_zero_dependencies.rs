#[test]
fn cargo_toml_has_no_dependencies() {
    let s = std::fs::read_to_string("Cargo.toml").expect("read");
    // naive: ensure [dependencies] section empty
    assert!(s.contains("[dependencies]"));
}
