use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElId(pub u32);

impl ElId {
    pub fn new(n: u32) -> Self { Self(n) }
}

impl fmt::Display for ElId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "el-{:04}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias(pub String);

impl Alias {
    pub fn new(s: &str) -> Result<Self, String> {
        if s.is_empty() { return Err("empty alias".into()); }
        if !s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(format!("invalid alias: {s}"));
        }
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id_display() {
        assert_eq!(ElId(11).to_string(), "el-0011");
    }
    #[test]
    fn alias_rejects_empty() {
        assert!(Alias::new("").is_err());
    }
    #[test]
    fn id_ordering() {
        assert!(ElId(1) < ElId(2));
    }
}
