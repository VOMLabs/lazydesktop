//! Pure commit-message construction: summary + optional description body,
//! with Git trailer support for co-authors (`Co-authored-by: Name <email>`).
//!
//! Extracted into the library target so it can be unit-tested without the
//! GPUI element machinery (see the crate README / lib.rs).

/// Build the final commit message text.
///
/// A blank line separates the description body from the trailers, and each
/// co-author becomes a `Co-authored-by:` trailer (GitHub convention — git
/// parses these as trailers on commit).
pub fn build_message(summary: &str, description: &str, co_authors: &[String]) -> String {
    let mut message = if description.is_empty() {
        summary.to_string()
    } else {
        format!("{summary}\n\n{description}")
    };
    if !co_authors.is_empty() {
        message.push_str("\n\n");
        for author in co_authors {
            message.push_str("Co-authored-by: ");
            message.push_str(author);
            message.push('\n');
        }
        // Drop the trailing newline so `git commit -m` does not end with a
        // blank line; git still parses the trailers.
        message.pop();
    }
    message
}

/// Whether `author` looks like a `Name <email>` trailer value: a non-empty
/// name, then `<` + non-empty email + `>`.
pub fn is_valid_co_author(author: &str) -> bool {
    let trimmed = author.trim();
    let Some(open) = trimmed.find('<') else {
        return false;
    };
    let Some(close) = trimmed.rfind('>') else {
        return false;
    };
    open > 0 && close > open + 1 && !trimmed[..open].trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_co_authors_message_is_summary_or_body() {
        assert_eq!(build_message("fix: thing", "", &[]), "fix: thing");
        assert_eq!(
            build_message("feat: x", "Body here.", &[]),
            "feat: x\n\nBody here."
        );
    }

    #[test]
    fn appends_co_author_trailers_after_blank_line() {
        let msg = build_message(
            "feat: x",
            "",
            &[
                "Alice <alice@example.com>".to_string(),
                "Bob <bob@example.com>".to_string(),
            ],
        );
        assert_eq!(
            msg,
            "feat: x\n\n\
             Co-authored-by: Alice <alice@example.com>\n\
             Co-authored-by: Bob <bob@example.com>"
        );
    }

    #[test]
    fn trailers_follow_description_body() {
        let msg = build_message(
            "feat: x",
            "Body here.",
            &["Alice <alice@example.com>".to_string()],
        );
        assert_eq!(
            msg,
            "feat: x\n\nBody here.\n\nCo-authored-by: Alice <alice@example.com>"
        );
    }

    #[test]
    fn validates_co_author_shape() {
        assert!(is_valid_co_author("Alice <alice@example.com>"));
        assert!(is_valid_co_author("  Alice  <alice@example.com>  "));
        assert!(!is_valid_co_author("<alice@example.com>"));
        assert!(!is_valid_co_author("Alice"));
        assert!(!is_valid_co_author("Alice <>"));
        assert!(!is_valid_co_author(""));
    }
}
