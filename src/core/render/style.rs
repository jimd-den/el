// Text layout, in one place: the projection marker, the indentation unit, and a
// line-counting output buffer. Everything that writes Rust goes through `Out`,
// which is what makes the span map possible at all — a line range is only
// knowable where the lines are actually appended.
//
// The marker is not decoration: it is the contract from ADR-0009 stated in the
// file itself, and it is the thing a human sees before hand-editing a file that
// `el` owns. Its exact bytes are pinned by a test because changing it changes
// every rendered file in every project.

/// Line 1 of every owned file. ADR-0009.
pub const MARKER: &str =
    "// @el-projection — generated from the catalog; change it with `el edit`, not here.";

/// One level of nesting. Four spaces, because that is what `cargo fmt` does and
/// a projection that disagrees with the formatter would be re-rendered forever.
pub const INDENT: &str = "    ";

/// The indentation for `depth` levels.
pub fn indent(depth: usize) -> String {
    INDENT.repeat(depth)
}

/// An accumulating file body that knows how many lines it has written.
#[derive(Debug, Default)]
pub struct Out {
    text: String,
    /// 1-based number of the next line to be written.
    next: u32,
}

impl Out {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            next: 1,
        }
    }

    /// The 1-based line number the next write will land on.
    pub fn line(&self) -> u32 {
        self.next
    }

    /// One indented line.
    pub fn line_at(&mut self, depth: usize, body: &str) {
        self.text.push_str(&indent(depth));
        self.text.push_str(body);
        self.text.push('\n');
        self.next += 1;
    }

    /// One line with no indentation of ours — a preamble or a marker.
    pub fn bare(&mut self, body: &str) {
        self.text.push_str(body);
        self.text.push('\n');
        self.next += 1;
    }

    /// A blank line, unless the last thing written was already blank or nothing.
    pub fn blank(&mut self) {
        if self.text.is_empty() || self.text.ends_with("\n\n") {
            return;
        }
        self.text.push('\n');
        self.next += 1;
    }

    /// Verbatim text whose first line is indented and whose remaining lines keep
    /// their own bytes. Used for opaque bodies, where re-indenting would change
    /// content the user wrote (FR-67).
    pub fn verbatim(&mut self, text: &str, depth: usize) {
        let mut lines = text.lines();
        if let Some(first) = lines.next() {
            self.line_at(depth, first.trim_end());
        }
        for line in lines {
            self.text.push_str(line);
            self.text.push('\n');
            self.next += 1;
        }
    }

    #[cfg(test)]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn into_text(self) -> String {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_indent_is_four_spaces_per_level() {
        assert_eq!(indent(0), "");
        assert_eq!(indent(2), "        ");
    }

    #[test]
    fn the_line_number_follows_what_was_written() {
        let mut out = Out::new();
        out.bare(MARKER);
        out.blank();
        out.line_at(1, "a();");

        assert_eq!(out.line(), 4);
        assert_eq!(out.text(), format!("{MARKER}\n\n    a();\n"));
    }

    #[test]
    fn two_blank_lines_never_stack() {
        let mut out = Out::new();
        out.bare(MARKER);
        out.blank();
        out.blank();

        assert_eq!(out.text(), format!("{MARKER}\n\n"));
    }

    #[test]
    fn a_leading_blank_line_is_never_written() {
        let mut out = Out::new();
        out.blank();

        assert_eq!(out.text(), "");
    }

    #[test]
    fn verbatim_keeps_the_tail_of_a_multiline_blob() {
        let mut out = Out::new();
        out.verbatim("if x {\n  a();\n}", 1);

        assert_eq!(out.text(), "    if x {\n  a();\n}\n");
        assert_eq!(out.line(), 4);
    }
}
