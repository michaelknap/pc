/// Conservatively strip full-line comments and blank lines.
///
/// - For known extensions, drop lines where the first non-whitespace chars
///   match the comment leader (#, //, --, etc.).
/// - For all files, drop completely blank lines.
/// - Does NOT touch inline/trailing comments or block comments.
pub fn strip_comments_for_ext(src: &str, ext: &str) -> String {
    let ext = ext.to_ascii_lowercase();

    let leaders: &[&str] = match ext.as_str() {
        // Hash comments
        "py" | "sh" | "bash" | "zsh" | "rb" | "yaml" | "yml" | "toml" => &["#"],
        // C-like // comments
        "rs" | "c" | "h" | "cpp" | "hpp" | "cc" | "js" | "ts" | "java" | "go" | "cs" | "swift"
        | "kt" => &["//"],
        // SQL-ish
        "sql" => &["--"],
        _ => &[],
    };

    let mut out = String::with_capacity(src.len());

    for line in src.lines() {
        let trimmed = line.trim_start();

        // Drop purely blank lines.
        if trimmed.is_empty() {
            continue;
        }

        if !leaders.is_empty() && leaders.iter().any(|leader| trimmed.starts_with(leader)) {
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_comments_py_removes_full_line_hash_comments_and_blanks() {
        let src = r#"
# top comment
print("hello")  # inline

    # indented comment

print("world")
"#;

        let out = strip_comments_for_ext(src, "py");
        let expected = "print(\"hello\")  # inline\nprint(\"world\")\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn strip_comments_unknown_only_removes_blank_lines() {
        let src = "  # not a comment for unknown\n\nx\n";
        let out = strip_comments_for_ext(src, "foo");
        let expected = "  # not a comment for unknown\nx\n";
        assert_eq!(out, expected);
    }
}
