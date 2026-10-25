#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Module,
    Function,
    Struct,
    Enum,
    Trait,
    Const,
    Static,
    TypeAlias,
    Impl,
    Stmt,
}

impl Kind {
    pub fn needs_name(&self) -> bool {
        !matches!(self, Kind::Stmt)
    }
    pub fn needs_body(&self) -> bool {
        matches!(self, Kind::Function | Kind::Impl | Kind::Module)
    }
    pub fn can_nest(&self) -> bool {
        matches!(self, Kind::Module | Kind::Impl | Kind::Trait)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stmt_needs_no_name() {
        assert!(!Kind::Stmt.needs_name());
    }
    #[test]
    fn all_kinds_answer_rules() {
        for k in [Kind::Module,Kind::Function,Kind::Struct,Kind::Enum,Kind::Trait,Kind::Const,Kind::Static,Kind::TypeAlias,Kind::Impl,Kind::Stmt] {
            let _ = (k.needs_name(), k.needs_body(), k.can_nest());
        }
    }
}
