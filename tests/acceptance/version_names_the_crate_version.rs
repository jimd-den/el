// `el version` answered "el 0.1.0 (M6)" from a string literal while the README
// said M8 and the crate said 0.1.0. A number written twice disagrees eventually, so
// the binary now reports the package version it was compiled from and lets the
// documents own the milestone.

use crate::program::{el, fresh, ok};

#[test]
fn version_prints_the_package_version() {
    let root = fresh("version-verb");

    let output = el(&root, &["version"]);

    ok(&output, "version");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        concat!("el ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn version_json_carries_the_same_string() {
    let root = fresh("version-json");

    let output = el(&root, &["version", "--json"]);

    ok(&output, "version --json");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(concat!("\"data\":\"el ", env!("CARGO_PKG_VERSION"), "\"")),
        "{text}"
    );
}
