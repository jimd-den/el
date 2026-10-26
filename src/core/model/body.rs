// The body tree the grammar produces: each statement is one line of source,
// with the statements of the block it opened nested underneath. It is the parsed
// form that exists before the model flattens it into `Stmt` records.
//
// It deliberately carries no identity or position: ownership, parent links and
// indices are the model's, assigned when the body is inserted. Here a statement
// is only its text and its children.

/// A parsed function body, in source order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Body {
    pub statements: Vec<BodyStmt>,
}

/// One line of a body, with the lines of the block it opens.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BodyStmt {
    pub text: String,
    pub children: Vec<BodyStmt>,
}
