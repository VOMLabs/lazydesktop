//! `.lzdignore` parsing and gitignore-style matching (ADDON_SPEC §7).
//!
//! Semantics follow gitignore closely enough for addon packaging:
//! - one pattern per line, `#` comments, `!` negation
//! - last matching pattern wins
//! - trailing `/` matches directories only
//! - a pattern containing `/` (other than a trailing one) is anchored to the
//!   package root; otherwise it matches at any level
//! - `**` crosses directory boundaries; `*`/`?`/`[...]` match within a
//!   component
//! - a file cannot be re-included if a parent directory is excluded
//! - `\` escapes the next character

/// Default ignores applied before any user patterns (§7.2).
pub const DEFAULT_IGNORES: &[&str] = &[".git/", ".hg/", ".svn/", ".DS_Store", ".lzdignore"];

/// Maximum length of a single pattern line.
pub const MAX_PATTERN_LINE: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
enum SegmentPattern {
    Literal(String),
    Glob(String),
    DoubleStar,
}

#[derive(Clone, Debug)]
struct Pattern {
    negated: bool,
    dir_only: bool,
    anchored: bool,
    segments: Vec<SegmentPattern>,
}

/// Compiled set of ignore rules.
#[derive(Clone, Debug, Default)]
pub struct IgnoreMatcher {
    patterns: Vec<Pattern>,
}

impl IgnoreMatcher {
    /// Build a matcher from built-in defaults plus the contents of a
    /// `.lzdignore` file. `content` may be empty.
    pub fn from_lines(content: &str) -> Self {
        let mut patterns = Vec::new();
        for line in DEFAULT_IGNORES {
            if let Some(p) = parse_line(line) {
                patterns.push(p);
            }
        }
        for line in content.lines() {
            if let Some(p) = parse_line(line) {
                patterns.push(p);
            }
        }
        Self { patterns }
    }

    /// Is `path` (a normalized, `/`-separated package-relative path) ignored?
    pub fn is_ignored(&self, path: &str, is_dir: bool) -> bool {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.is_empty() {
            return false;
        }
        let mut excluded_ancestor = false;
        for i in 0..parts.len() {
            let prefix_is_dir = i < parts.len() - 1;
            let dir = prefix_is_dir || is_dir;
            let last = self
                .patterns
                .iter()
                .rev()
                .find(|p| !(p.dir_only && !dir) && matches_pattern(p, &parts[..=i]));
            if prefix_is_dir {
                // Only directory-only patterns may exclude a whole subtree.
                // A non-dir-only pattern (e.g. `a/**/b`) matches a path whose
                // leaf happens to be a directory, but it must not claim the
                // directory's children.
                if let Some(p) = last.filter(|p| p.dir_only) {
                    excluded_ancestor = !p.negated;
                }
            } else {
                return match last {
                    Some(p) => {
                        if excluded_ancestor {
                            true
                        } else {
                            !p.negated
                        }
                    }
                    None => excluded_ancestor,
                };
            }
        }
        excluded_ancestor
    }
}

fn parse_line(line: &str) -> Option<Pattern> {
    if line.len() > MAX_PATTERN_LINE {
        return None;
    }
    let mut line = line.trim_end_matches(' ').trim_end_matches('\r');
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut negated = false;
    if line.starts_with('!') {
        negated = true;
        line = &line[1..];
    }
    if line.is_empty() {
        return None;
    }
    let mut dir_only = false;
    if line.len() > 1 && line.ends_with('/') {
        dir_only = true;
        line = &line[..line.len() - 1];
    }
    if line.is_empty() || line == "/" {
        return None;
    }
    let anchored = line.contains('/');
    let segments: Vec<SegmentPattern> = line
        .split('/')
        .filter(|s| !s.is_empty())
        .map(parse_segment)
        .collect();
    if segments.is_empty() {
        return None;
    }
    Some(Pattern {
        negated,
        dir_only,
        anchored,
        segments,
    })
}

fn parse_segment(seg: &str) -> SegmentPattern {
    if seg == "**" {
        return SegmentPattern::DoubleStar;
    }
    if seg.contains('*') || seg.contains('?') || seg.contains('[') {
        SegmentPattern::Glob(seg.to_string())
    } else {
        SegmentPattern::Literal(seg.to_string())
    }
}

/// Match a compiled pattern against a prefix of normalized path parts.
fn matches_pattern(p: &Pattern, parts: &[&str]) -> bool {
    if p.anchored {
        segments_match(&p.segments, parts)
    } else {
        (0..=parts.len()).any(|start| segments_match(&p.segments, &parts[start..]))
    }
}

fn segments_match(pats: &[SegmentPattern], parts: &[&str]) -> bool {
    match (pats, parts) {
        ([], []) => true,
        ([SegmentPattern::DoubleStar, rest @ ..], parts) => {
            segments_match(rest, parts) || (!parts.is_empty() && segments_match(pats, &parts[1..]))
        }
        ([p, rest @ ..], [part, parts_rest @ ..]) => {
            segment_match(p, part) && segments_match(rest, parts_rest)
        }
        _ => false,
    }
}

fn segment_match(p: &SegmentPattern, part: &str) -> bool {
    match p {
        SegmentPattern::Literal(l) => l == part,
        SegmentPattern::Glob(g) => glob_segment_match(g, part),
        SegmentPattern::DoubleStar => true,
    }
}

/// Classic glob match within a single path component. Supports `*`, `?`,
/// `[...]` (with `!`/`^` negation and `a-z` ranges), and `\` escapes.
fn glob_segment_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star_p = usize::MAX;
    let mut star_t = 0usize;

    while ti < t.len() {
        if pi < p.len() {
            let pc = p[pi];
            match pc {
                '*' => {
                    star_p = pi;
                    star_t = ti;
                    pi += 1;
                    continue;
                }
                '?' => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                '\\' if pi + 1 < p.len() => {
                    if p[pi + 1] == t[ti] {
                        pi += 2;
                        ti += 1;
                        continue;
                    }
                }
                '[' => {
                    if let Some((matched, next)) = match_class(&p, pi, t[ti]) {
                        if matched {
                            pi = next;
                            ti += 1;
                            continue;
                        }
                    }
                }
                c if c == t[ti] => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                _ => {}
            }
        }
        // Mismatch: backtrack to the last '*'.
        if star_p != usize::MAX {
            star_t += 1;
            ti = star_t;
            pi = star_p + 1;
            continue;
        }
        return false;
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Parse a character class starting at `start` (the `[`). Returns whether
/// `ch` matches and the index just past the closing `]`.
fn match_class(p: &[char], start: usize, ch: char) -> Option<(bool, usize)> {
    let mut i = start + 1;
    let mut neg = false;
    if i < p.len() && (p[i] == '!' || p[i] == '^') {
        neg = true;
        i += 1;
    }
    let mut matched = false;
    let mut first = true;
    while i < p.len() && (p[i] != ']' || first) {
        if p[i] == '\\' && i + 1 < p.len() {
            if p[i + 1] == ch {
                matched = true;
            }
            i += 2;
        } else if i + 2 < p.len() && p[i + 1] == '-' && p[i + 2] != ']' {
            let lo = p[i];
            let hi = p[i + 2];
            if lo <= ch && ch <= hi {
                matched = true;
            }
            i += 3;
        } else {
            if p[i] == ch {
                matched = true;
            }
            i += 1;
        }
        first = false;
    }
    if i >= p.len() || p[i] != ']' {
        // Unterminated class: treat as a literal '[' (no match).
        return None;
    }
    Some((if neg { !matched } else { matched }, i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(content: &str) -> IgnoreMatcher {
        IgnoreMatcher::from_lines(content)
    }

    #[test]
    fn default_ignores_apply_without_user_patterns() {
        let m = matcher("");
        assert!(m.is_ignored(".git/config", false));
        assert!(m.is_ignored(".lzdignore", false));
        assert!(m.is_ignored("a/.DS_Store", false));
        assert!(!m.is_ignored("config.toml", false));
        assert!(!m.is_ignored("src/main.lua", false));
    }

    #[test]
    fn wildcard_matches_at_any_level() {
        let m = matcher("*.log");
        assert!(m.is_ignored("app.log", false));
        assert!(m.is_ignored("logs/app.log", false));
        assert!(m.is_ignored("a/b/c/app.log", false));
        assert!(!m.is_ignored("app.lua", false));
    }

    #[test]
    fn directory_pattern_excludes_children() {
        let m = matcher("build/");
        assert!(m.is_ignored("build", true));
        assert!(m.is_ignored("build/x.txt", false));
        assert!(m.is_ignored("a/build/y.txt", false));
        assert!(!m.is_ignored("build.txt", false));
    }

    #[test]
    fn negation_reincludes() {
        let m = matcher("*.log\n!important.log");
        assert!(m.is_ignored("app.log", false));
        assert!(!m.is_ignored("important.log", false));
    }

    #[test]
    fn anchored_pattern_matches_root_only() {
        let m = matcher("/rooted.txt");
        assert!(m.is_ignored("rooted.txt", false));
        assert!(!m.is_ignored("sub/rooted.txt", false));
    }

    #[test]
    fn double_star_crosses_directories() {
        let m = matcher("a/**/b");
        assert!(m.is_ignored("a/b", false));
        assert!(m.is_ignored("a/x/b", false));
        assert!(m.is_ignored("a/x/y/b", false));
        assert!(!m.is_ignored("a/x/b/y", false));
    }

    #[test]
    fn parent_exclusion_cannot_be_negated() {
        let m = matcher("/build/\n!/build/keep.txt");
        // Per gitignore: re-include of a file under an excluded dir fails.
        assert!(m.is_ignored("build", true));
        assert!(m.is_ignored("build/keep.txt", false));
    }

    #[test]
    fn comments_and_blank_lines_ignored() {
        let m = matcher("# comment\n\n*.tmp\n");
        assert!(m.is_ignored("x.tmp", false));
        assert!(!m.is_ignored("x.txt", false));
    }

    #[test]
    fn question_and_class() {
        let m = matcher("file?.txt\n[a-c].md");
        assert!(m.is_ignored("file1.txt", false));
        assert!(!m.is_ignored("file10.txt", false));
        assert!(m.is_ignored("b.md", false));
        assert!(!m.is_ignored("d.md", false));
    }

    #[test]
    fn escaped_metacharacters() {
        let m = matcher("a\\*b");
        assert!(m.is_ignored("a*b", false));
        assert!(!m.is_ignored("axb", false));
    }

    #[test]
    fn last_match_wins() {
        let m = matcher("*.txt\n!keep.txt\nsub/keep.txt");
        assert!(m.is_ignored("x.txt", false));
        assert!(!m.is_ignored("keep.txt", false));
        assert!(m.is_ignored("sub/keep.txt", false));
    }

    #[test]
    fn config_toml_not_ignored_by_default() {
        let m = matcher("");
        assert!(!m.is_ignored("config.toml", false));
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        fn simple_pattern() -> impl Strategy<Value = String> {
            prop_oneof![
                Just("*.txt".to_string()),
                Just("build/".to_string()),
                Just("!keep.txt".to_string()),
                Just("/anchored.txt".to_string()),
                Just("logs/".to_string()),
                Just("*.md".to_string()),
                Just("!keep.md".to_string()),
            ]
        }

        fn path_strategy() -> impl Strategy<Value = String> {
            prop::collection::vec(
                prop_oneof![
                    Just("a"),
                    Just("b"),
                    Just("c"),
                    Just("build"),
                    Just("keep.txt"),
                    Just("x.txt"),
                    Just("y.md"),
                    Just("anchored.txt"),
                    Just("logs"),
                ],
                1..4,
            )
            .prop_map(|v| v.join("/"))
        }

        proptest! {
            #[test]
            fn matches_reference_implementation(lines in prop::collection::vec(simple_pattern(), 0..6), path in path_strategy(), is_dir in any::<bool>()) {
                let content = lines.join("\n");
                let m = matcher(&content);
                let ref_pats: Vec<Pattern> = DEFAULT_IGNORES
                    .iter()
                    .copied()
                    .chain(lines.iter().map(|s| s.as_str()))
                    .filter_map(parse_line)
                    .collect();
                let actual = m.is_ignored(&path, is_dir);
                let expected = reference_is_ignored(&ref_pats, &path, is_dir);
                prop_assert_eq!(
                    actual,
                    expected,
                    "mismatch for patterns={:?} path={:?} is_dir={}",
                    content,
                    path,
                    is_dir
                );
            }
        }

        /// Reference implementation mirroring production semantics: single
        /// star/glob segments use the real segment matcher, and only
        /// directory-only patterns may claim ancestor directories.
        fn reference_is_ignored(patterns: &[Pattern], path: &str, is_dir: bool) -> bool {
            let parts: Vec<&str> = path.split('/').collect();
            let mut excluded_ancestor = false;
            for i in 0..parts.len() {
                let prefix_is_dir = i < parts.len() - 1;
                let dir = prefix_is_dir || is_dir;
                let last = patterns.iter().rev().find(|p| {
                    if p.dir_only && !dir {
                        return false;
                    }
                    if p.anchored {
                        ref_segments_match(&p.segments, &parts[..=i])
                    } else {
                        (0..=i).any(|s| ref_segments_match(&p.segments, &parts[s..=i]))
                    }
                });
                if prefix_is_dir {
                    if let Some(p) = last.filter(|p| p.dir_only) {
                        excluded_ancestor = !p.negated;
                    }
                } else {
                    return match last {
                        Some(p) => {
                            if excluded_ancestor {
                                true
                            } else {
                                !p.negated
                            }
                        }
                        None => excluded_ancestor,
                    };
                }
            }
            excluded_ancestor
        }

        fn ref_segments_match(pats: &[SegmentPattern], parts: &[&str]) -> bool {
            pats.len() == parts.len()
                && pats
                    .iter()
                    .zip(parts)
                    .all(|(p, s)| match p {
                        SegmentPattern::Literal(l) => l == s,
                        SegmentPattern::Glob(g) => glob_segment_match(g, s),
                        SegmentPattern::DoubleStar => false,
                    })
        }
    }
}
