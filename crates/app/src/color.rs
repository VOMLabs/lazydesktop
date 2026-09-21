//! Color helpers, separated from the GPUI element machinery so they can be
//! unit-tested in the library target (see `lib.rs`).

/// Parse a hex color string like `#RRGGBB` (the `#` is optional) into a
/// `0xRRGGBB` value suitable for `gpui::rgb(u32)`.
///
/// Returns `None` for anything that is not exactly six hex digits.
pub fn parse_hex_color(input: &str) -> Option<u32> {
    let trimmed = input.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 {
        return None;
    }
    let Ok(value) = u32::from_str_radix(hex, 16) else {
        return None;
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hashed_hex() {
        assert_eq!(parse_hex_color("#1e1e1e"), Some(0x1e1e1e));
        assert_eq!(parse_hex_color("#2ea043"), Some(0x2ea043));
    }

    #[test]
    fn parses_bare_hex() {
        assert_eq!(parse_hex_color("ffffff"), Some(0xffffff));
        assert_eq!(parse_hex_color("000000"), Some(0x000000));
    }

    #[test]
    fn rejects_invalid_lengths() {
        assert_eq!(parse_hex_color("#fff"), None);
        assert_eq!(parse_hex_color("#12345"), None);
        assert_eq!(parse_hex_color("#1234567"), None);
        assert_eq!(parse_hex_color(""), None);
    }

    #[test]
    fn rejects_non_hex_digits() {
        assert_eq!(parse_hex_color("#zzzzzz"), None);
        assert_eq!(parse_hex_color("#12g456"), None);
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(parse_hex_color("  #1e1e1e  "), Some(0x1e1e1e));
    }
}
