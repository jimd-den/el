pub fn split(body: &str) -> Option<Vec<String>> {
    // Minimal: split on top-level semicolons outside strings (naive).
    Some(body.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}
pub fn join(parts: &[String]) -> String {
    parts.join("; ") + if parts.is_empty() { "" } else { ";" }
}
