// SPDX-License-Identifier: MIT

/// The longest label a surface carries, in characters.
pub const MAX_LABEL_CHARS: usize = 32;

/// A human-readable label for a surface, as the user typed it, or `None` if it is not one:
/// surrounding space is dropped; what is left is 1 to [`MAX_LABEL_CHARS`] characters, none of
/// them a control character (labels are placed in backend metadata and drawn on a terminal).
/// A label is presentation only. Two surfaces may carry the same one: nothing is ever
/// addressed by it.
pub fn clean_label(text: &str) -> Option<String> {
    let label = text.trim();
    (!label.is_empty()
        && label.chars().count() <= MAX_LABEL_CHARS
        && !label.chars().any(char::is_control))
    .then(|| label.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_is_trimmed_text_of_bounded_length_without_control_characters() {
        assert_eq!(clean_label("  logs "), Some("logs".to_owned()));
        assert_eq!(
            clean_label("build / watch ✓"),
            Some("build / watch ✓".to_owned())
        );
        assert_eq!(clean_label(""), None);
        assert_eq!(clean_label("   "), None);
        assert_eq!(clean_label("a\tb"), None);
        assert_eq!(clean_label("a\nb"), None);
        assert_eq!(clean_label("\u{1b}[31m"), None);
        assert!(clean_label(&"x".repeat(MAX_LABEL_CHARS)).is_some());
        assert_eq!(clean_label(&"x".repeat(MAX_LABEL_CHARS + 1)), None);
    }
}
