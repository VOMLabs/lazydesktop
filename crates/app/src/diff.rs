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

/// Kind of a word-level span inside a changed line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WordSpanKind {
    /// Text present on both sides of the change.
    Same,
    /// Text only in the new (added) line.
    Insert,
    /// Text only in the old (removed) line.
    Delete,
}

/// A highlighted run of text within a changed line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordSpan {
    pub kind: WordSpanKind,
    pub text: String,
}

/// Word-level alignment of one removed line with its added counterpart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordDiff {
    /// Spans for the removed line (`Same`/`Delete`), concatenating to its content.
    pub removed: Vec<WordSpan>,
    /// Spans for the added line (`Same`/`Insert`), concatenating to its content.
    pub added: Vec<WordSpan>,
}

/// One aligned change unit: the removed and/or added line indices into the
/// parsed diff (`None` when that side has no line — a pure insertion/deletion).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangePair {
    pub deleted: Option<usize>,
    pub added: Option<usize>,
}

/// Maximum token count per side before falling back to whole-line coloring.
const MAX_WORD_TOKENS: usize = 256;

/// Group contiguous `+`/`-` lines into change pairs (GitHub-style alignment):
/// the i-th removal pairs with the i-th addition within a block; leftovers
/// become single-sided pairs. Context and hunk lines are skipped.
pub fn pair_changes(lines: &[DiffLine]) -> Vec<ChangePair> {
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind != DiffLineKind::Addition && lines[i].kind != DiffLineKind::Deletion {
            i += 1;
            continue;
        }
        let mut dels = Vec::new();
        let mut adds = Vec::new();
        while i < lines.len() {
            match lines[i].kind {
                DiffLineKind::Deletion => dels.push(i),
                DiffLineKind::Addition => adds.push(i),
                _ => break,
            }
            i += 1;
        }
        let paired = dels.len().min(adds.len());
        for k in 0..paired {
            pairs.push(ChangePair {
                deleted: Some(dels[k]),
                added: Some(adds[k]),
            });
        }
        for &d in &dels[paired..] {
            pairs.push(ChangePair {
                deleted: Some(d),
                added: None,
            });
        }
        for &a in &adds[paired..] {
            pairs.push(ChangePair {
                deleted: None,
                added: Some(a),
            });
        }
    }
    pairs
}

/// Compute the word-level alignment of a removed line with its added line.
///
/// Tokens are runs of alphanumeric/underscore characters split at other
/// characters (whitespace, punctuation), so a classic LCS over tokens yields
/// per-line spans that concatenate back to the original content. Lines longer
/// than [`MAX_WORD_TOKENS`] tokens fall back to whole-line spans.
pub fn word_diff(old: &str, new: &str) -> WordDiff {
    let old_tokens = word_tokens(old);
    let new_tokens = word_tokens(new);

    if old_tokens.len() > MAX_WORD_TOKENS || new_tokens.len() > MAX_WORD_TOKENS {
        return WordDiff {
            removed: vec![WordSpan {
                kind: WordSpanKind::Delete,
                text: old.to_string(),
            }],
            added: vec![WordSpan {
                kind: WordSpanKind::Insert,
                text: new.to_string(),
            }],
        };
    }

    let n = old_tokens.len();
    let m = new_tokens.len();
    // `lcs[i][j]` = LCS length of `old_tokens[i..]` and `new_tokens[j..]`.
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if old_tokens[i] == new_tokens[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut removed = Vec::new();
    let mut added = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old_tokens[i] == new_tokens[j] {
            removed.push(WordSpan {
                kind: WordSpanKind::Same,
                text: old_tokens[i].clone(),
            });
            added.push(WordSpan {
                kind: WordSpanKind::Same,
                text: new_tokens[j].clone(),
            });
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            removed.push(WordSpan {
                kind: WordSpanKind::Delete,
                text: old_tokens[i].clone(),
            });
            i += 1;
        } else {
            added.push(WordSpan {
                kind: WordSpanKind::Insert,
                text: new_tokens[j].clone(),
            });
            j += 1;
        }
    }
    while i < n {
        removed.push(WordSpan {
            kind: WordSpanKind::Delete,
            text: old_tokens[i].clone(),
        });
        i += 1;
    }
    while j < m {
        added.push(WordSpan {
            kind: WordSpanKind::Insert,
            text: new_tokens[j].clone(),
        });
        j += 1;
    }

    WordDiff { removed, added }
}

/// Split text into word tokens: runs of alphanumeric/underscore characters
/// alternated with runs of everything else, so concatenation reproduces input.
fn word_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_is_word = false;
    for c in text.chars() {
        let is_word = c.is_alphanumeric() || c == '_';
        if current.is_empty() {
            current.push(c);
            current_is_word = is_word;
        } else if current_is_word == is_word {
            current.push(c);
        } else {
            tokens.push(std::mem::take(&mut current));
            current.push(c);
            current_is_word = is_word;
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Raster image extensions the diff viewer can render inline.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/// Whether a repo-relative path points at a raster image gpui decodes inline
/// (`image` crate: png/jpg/gif/webp/bmp).
pub fn is_image_path(path: &str) -> bool {
    let Some(ext) = path.rsplit('.').next() else {
        return false;
    };
    IMAGE_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e))
}

/// Extract the new-side paths of image files from raw `git show` output
/// (`diff --git a/<old> b/<new>` / `rename to <new>` headers), deduplicated.
pub fn image_paths_from_raw(raw: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in raw.lines() {
        let candidate = if let Some(rest) = line.strip_prefix("diff --git ") {
            rest.split_once(" b/")
                .map(|(_, new_path)| new_path.to_string())
        } else {
            line.strip_prefix("rename to ").map(|p| p.to_string())
        };
        if let Some(path) = candidate {
            if is_image_path(&path) && seen.insert(path.clone()) {
                paths.push(path);
            }
        }
    }
    paths
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

    #[test]
    fn word_diff_identical_text_is_all_same() {
        let d = word_diff("let x = 1;", "let x = 1;");
        assert!(d.removed.iter().all(|s| s.kind == WordSpanKind::Same));
        assert!(d.added.iter().all(|s| s.kind == WordSpanKind::Same));
        let removed_text: String = d.removed.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(removed_text, "let x = 1;");
        let added_text: String = d.added.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(added_text, "let x = 1;");
    }

    #[test]
    fn word_diff_insertion_marks_new_words() {
        let d = word_diff("fn foo()", "fn bar()");
        let added_text: String = d.added.iter().map(|s| s.text.as_str()).collect();
        let removed_text: String = d.removed.iter().map(|s| s.text.as_str()).collect();
        // Spans are lossless: they reconstruct the input lines exactly.
        assert_eq!(added_text, "fn bar()");
        assert_eq!(removed_text, "fn foo()");
        let inserted: Vec<&str> = d
            .added
            .iter()
            .filter(|s| s.kind == WordSpanKind::Insert)
            .map(|s| s.text.as_str())
            .collect();
        let deleted: Vec<&str> = d
            .removed
            .iter()
            .filter(|s| s.kind == WordSpanKind::Delete)
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(inserted, vec!["bar"]);
        assert_eq!(deleted, vec!["foo"]);
    }

    #[test]
    fn word_diff_deletion_marks_removed_words() {
        let d = word_diff("let value = 1;", "let = 1;");
        let removed_text: String = d.removed.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(removed_text, "let value = 1;");
        let deleted: Vec<&str> = d
            .removed
            .iter()
            .filter(|s| s.kind == WordSpanKind::Delete)
            .map(|s| s.text.as_str())
            .collect();
        assert!(
            deleted.contains(&"value"),
            "value should be marked: {deleted:?}"
        );
        // Nothing was added: the new side is entirely unchanged tokens.
        assert!(d.added.iter().all(|s| s.kind == WordSpanKind::Same));
    }

    #[test]
    fn word_diff_replace_marks_both_sides() {
        let d = word_diff("let x = 1;", "let y = 2;");
        let deleted: Vec<&str> = d
            .removed
            .iter()
            .filter(|s| s.kind == WordSpanKind::Delete)
            .map(|s| s.text.as_str())
            .collect();
        let inserted: Vec<&str> = d
            .added
            .iter()
            .filter(|s| s.kind == WordSpanKind::Insert)
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(deleted, vec!["x", "1"]);
        assert_eq!(inserted, vec!["y", "2"]);
    }

    #[test]
    fn pair_changes_pairs_adjacent_adds_and_dels() {
        let lines = parse_diff("-a\n+a2\n-b\n+b2\n");
        let pairs = pair_changes(&lines);
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs[0],
            ChangePair {
                deleted: Some(0),
                added: Some(1),
            }
        );
        assert_eq!(
            pairs[1],
            ChangePair {
                deleted: Some(2),
                added: Some(3),
            }
        );
    }

    #[test]
    fn pair_changes_handles_uneven_blocks() {
        let lines = parse_diff("-a\n-b\n+a2\n");
        let pairs = pair_changes(&lines);
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs[0],
            ChangePair {
                deleted: Some(0),
                added: Some(2),
            }
        );
        assert_eq!(
            pairs[1],
            ChangePair {
                deleted: Some(1),
                added: None,
            }
        );
    }

    #[test]
    fn pair_changes_skips_context_and_hunks() {
        let raw = "@@ -1,4 +1,4 @@\n ctx1\n-a\n+a2\n ctx2\n";
        let lines = parse_diff(raw);
        let pairs = pair_changes(&lines);
        assert_eq!(pairs.len(), 1);
        assert_eq!(
            pairs[0],
            ChangePair {
                deleted: Some(2),
                added: Some(3),
            }
        );
    }

    #[test]
    fn is_image_path_matches_common_formats() {
        assert!(is_image_path("assets/logo.png"));
        assert!(is_image_path("photo.JPG"));
        assert!(is_image_path("img/animation.gif"));
        assert!(is_image_path("photo.jpeg"));
        assert!(is_image_path("web/image.webp"));
        assert!(is_image_path("favicon.bmp"));
        assert!(!is_image_path("src/main.rs"));
        assert!(!is_image_path("Cargo.toml"));
        assert!(!is_image_path("README"));
    }

    #[test]
    fn image_paths_from_raw_extracts_new_side_paths() {
        let raw = "diff --git a/logo.png b/logo.png\n\
index 111..222 100644\n\
Binary files a/logo.png and b/logo.png differ\n\
diff --git a/src/main.rs b/src/main.rs\n\
@@ -1 +1 @@\n\
-old\n\
+new\n\
diff --git a/old.bmp b/new.bmp\n\
similarity index 100%\n\
rename from old.bmp\n\
rename to new.bmp\n";
        let paths = image_paths_from_raw(raw);
        assert_eq!(paths, vec!["logo.png".to_string(), "new.bmp".to_string()]);
    }
}
