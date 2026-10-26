// A refusal has to be as precise as the instruction it refuses: which rule, which
// element, how far over, and the smallest action that would satisfy the rule.
// That is the whole of this module — one closed set of codes, one record, and the
// one place their text is spelled (FR-18, GATE.md §5).
//
// The codes are an enum rather than strings so that adding a rule without wiring
// its name fails the suite instead of printing a code nobody can look up. The
// remedy is free text because it is advice about *this* catalog, not a constant.

use crate::core::model::id::ElId;
use std::fmt;

/// Every L0 rule, one code each. `ALL` is exhaustive by the test below.
pub const ALL: [Code; 12] = [
    Code::DuplicateAlias,
    Code::UnknownRef,
    Code::CrossBodyMove,
    Code::Unbalanced,
    Code::BadIdent,
    Code::Ceiling,
    Code::FnBudget,
    Code::Preamble,
    Code::Forbidden,
    Code::DepDenied,
    Code::DanglingRef,
    Code::BadStmt,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Code {
    DuplicateAlias,
    UnknownRef,
    CrossBodyMove,
    Unbalanced,
    BadIdent,
    Ceiling,
    FnBudget,
    Preamble,
    Forbidden,
    DepDenied,
    DanglingRef,
    BadStmt,
}

impl Code {
    pub fn all() -> &'static [Code] {
        &ALL
    }

    /// The stable, greppable code a user sees. Never reused for another rule.
    pub fn as_str(self) -> &'static str {
        match self {
            Code::DuplicateAlias => "model.duplicate_alias",
            Code::UnknownRef => "model.unknown_ref",
            Code::CrossBodyMove => "model.cross_body_move",
            Code::Unbalanced => "model.unbalanced",
            Code::BadIdent => "model.bad_ident",
            Code::Ceiling => "model.ceiling",
            Code::FnBudget => "model.fn_budget",
            Code::Preamble => "model.preamble",
            Code::Forbidden => "model.forbidden",
            Code::DepDenied => "model.dep_denied",
            Code::DanglingRef => "model.dangling_ref",
            Code::BadStmt => "model.bad_stmt",
        }
    }
}

/// One refusal, with the evidence that makes it actionable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub code: Code,
    pub id: Option<ElId>,
    /// The measured quantity the rule compares (`ceiling`, `max_fn_lines`), when
    /// there is one. Named in the message so a user can check the arithmetic.
    pub size: Option<usize>,
    pub remedy: String,
}

impl Violation {
    pub fn new(code: Code, remedy: impl Into<String>) -> Self {
        Self {
            code,
            id: None,
            size: None,
            remedy: remedy.into(),
        }
    }

    pub fn at(code: Code, id: ElId, remedy: impl Into<String>) -> Self {
        Self {
            id: Some(id),
            ..Self::new(code, remedy)
        }
    }

    /// Attach the measured size; the last `sized` call wins, which is why every
    /// caller sets one value.
    pub fn sized(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code.as_str())?;
        if let Some(id) = self.id {
            write!(f, " {id}")?;
        }
        if let Some(size) = self.size {
            write!(f, " ({size})")?;
        }
        write!(f, ": {}", self.remedy)
    }
}

/// A non-empty set of refusals. Empty is not representable, so a caller that
/// holds one knows the change was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violations(Vec<Violation>);

impl Violations {
    /// `None` for an empty set: a change with no violations has no refusals.
    pub fn new(violations: Vec<Violation>) -> Option<Self> {
        if violations.is_empty() {
            None
        } else {
            Some(Self(violations))
        }
    }

    pub fn one(violation: Violation) -> Self {
        Self(vec![violation])
    }

    pub fn first(&self) -> &Violation {
        &self.0[0]
    }

    pub fn codes(&self) -> Vec<Code> {
        self.0.iter().map(|violation| violation.code).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Violation> {
        self.0.iter()
    }
}

impl fmt::Display for Violations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, violation) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{violation}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "violation_tests.rs"]
mod violation_tests;
