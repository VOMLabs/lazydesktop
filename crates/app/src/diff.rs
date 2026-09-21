//! Pure unified-diff parsing for the diff viewer.
//!
//! Extracted into the library target so it can be unit-tested without the GPUI
//! element machinery — compiling `#[test]` over the binary's deeply-nested
//! element types overflows the compiler stack (see the crate README / lib.rs).
//! No GPUI imports here: logic only.

/// Classification of a single diff line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    Hunk,
}

/// A single parsed line of a unified diff.
#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

/// Parse raw unified diff output into typed lines.
///
/// Mirrors the Qt `DiffViewer::setDiff` filtering (`src/diffviewer.cpp`):
/// header lines (`---`, `+++`, `diff --git`, `\ No newline ...`) are dropped;
/// every other line is classified by its leading character.
pub fn parse_diff(raw: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    for line in raw.lines() {
        if line.starts_with("---")
            || line.starts_with("+++")
            || line.starts_with("diff --git")
            || line.starts_with("\\ ")
        {
            continue;
        }
        let kind = if line.starts_with('+') {
            DiffLineKind::Addition
        } else if line.starts_with('-') {
            DiffLineKind::Deletion
        } else if line.starts_with("@@") {
            DiffLineKind::Hunk
        } else {
            DiffLineKind::Context
        };
        lines.push(DiffLine {
            kind,
            text: line.to_string(),
        });
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_headers_and_classifies_lines() {
        let raw = "diff --git a/src/main.cpp b/src/main.cpp\n\
index 1234567..89abcde 100644\n\
--- a/src/main.cpp\n\
+++ b/src/main.cpp\n\
@@ -1,3 +1,4 @@\n\
 context line\n\
+added line\n\
-removed line\n\
\\ No newline at end of file\n";

        let lines = parse_diff(raw);
        // `index ...` is not filtered (matches Qt) — it renders as context.
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].kind, DiffLineKind::Context);
        assert_eq!(lines[0].text, "index 1234567..89abcde 100644");
        assert_eq!(lines[1].kind, DiffLineKind::Hunk);
        assert_eq!(lines[1].text, "@@ -1,3 +1,4 @@");
        assert_eq!(lines[2].kind, DiffLineKind::Context);
        assert_eq!(lines[3].kind, DiffLineKind::Addition);
        assert_eq!(lines[4].kind, DiffLineKind::Deletion);
    }

    #[test]
    fn header_only_diff_yields_no_lines() {
        assert!(parse_diff("").is_empty());
        assert!(parse_diff("diff --git a/x b/x\n--- a/x\n+++ b/x\n").is_empty());
    }

    #[test]
    fn plus_prefix_marker_is_not_a_math_sign() {
        // `+`/`-` prefixes delimit content, not arithmetic.
        let lines = parse_diff("+1 + 1 = 2\n-café\n");
        assert_eq!(lines[0].kind, DiffLineKind::Addition);
        assert_eq!(lines[1].kind, DiffLineKind::Deletion);
    }

    #[test]
    fn binary_file_diff_falls_back_to_context() {
        let raw = "diff --git a/logo.png b/logo.png\n\
index 1111111..2222222 100644\n\
Binary files a/logo.png and b/logo.png differ\n";
        let lines = parse_diff(raw);
        // `index ...` is not filtered (matches Qt) — it renders as context.
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].kind, DiffLineKind::Context);
        assert!(lines[0].text.starts_with("index "));
        assert_eq!(lines[1].kind, DiffLineKind::Context);
        assert_eq!(
            lines[1].text,
            "Binary files a/logo.png and b/logo.png differ"
        );
    }
}
