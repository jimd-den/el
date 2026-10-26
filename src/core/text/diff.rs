// A line diff for `el diff`, built on the longest common subsequence. It is
// deliberately small: files are rendered source, not binaries, and a diff that
// reports the statement id is the useful one; this module supplies the line view
// inside that report.
//
// It does not compute character-level diffs or rename detection. Those are editor
// features, and a line diff is what `el diff` promises.

/// One line's fate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Same(String),
    Remove(String),
    Add(String),
}

/// The shortest line edit script between `old` and `new`.
pub fn lines(old: &str, new: &str) -> Vec<Line> {
    let left: Vec<&str> = old.lines().collect();
    let right: Vec<&str> = new.lines().collect();
    let mut table = vec![vec![0usize; right.len() + 1]; left.len() + 1];

    for i in (0..left.len()).rev() {
        for j in (0..right.len()).rev() {
            table[i][j] = if left[i] == right[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < left.len() && j < right.len() {
        if left[i] == right[j] {
            out.push(Line::Same(left[i].to_string()));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            out.push(Line::Remove(left[i].to_string()));
            i += 1;
        } else {
            out.push(Line::Add(right[j].to_string()));
            j += 1;
        }
    }
    out.extend(left[i..].iter().map(|line| Line::Remove(line.to_string())));
    out.extend(right[j..].iter().map(|line| Line::Add(line.to_string())));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_is_all_same() {
        assert_eq!(
            lines("a\nb\n", "a\nb\n"),
            vec![Line::Same("a".to_string()), Line::Same("b".to_string())]
        );
    }

    #[test]
    fn one_changed_line_is_one_remove_and_one_add() {
        assert_eq!(
            lines("a\nb\n", "a\nc\n"),
            vec![
                Line::Same("a".to_string()),
                Line::Remove("b".to_string()),
                Line::Add("c".to_string())
            ]
        );
    }

    #[test]
    fn an_empty_side_is_all_added_or_removed() {
        assert_eq!(lines("", "a\n"), vec![Line::Add("a".to_string())]);
        assert_eq!(lines("a\n", ""), vec![Line::Remove("a".to_string())]);
    }
}
