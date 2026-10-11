// SPDX-License-Identifier: MIT

use std::fmt;

/// A tmux version as `tmux -V` reports it: `tmux 3.3a`, `tmux next-3.5`. Only the numbers
/// count; a letter suffix is a patch release of the same minor version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TmuxVersion {
    pub major: u32,
    pub minor: u32,
}

impl TmuxVersion {
    /// The oldest tmux Flight supports (docs/releases/v1.0-contract.md).
    pub const MINIMUM: Self = Self { major: 3, minor: 1 };

    /// The version in the output of `tmux -V`, or `None` for a build that names none
    /// (`tmux master`) or anything else this cannot read.
    pub fn parse(output: &str) -> Option<Self> {
        let word = output.split_whitespace().nth(1)?;
        let word = word.strip_prefix("next-").unwrap_or(word);
        let (major, rest) = split_number(word)?;
        let (minor, _suffix) = split_number(rest.strip_prefix('.')?)?;
        Some(Self { major, minor })
    }

    pub fn is_supported(self) -> bool {
        self >= Self::MINIMUM
    }
}

impl fmt::Display for TmuxVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

fn split_number(text: &str) -> Option<(u32, &str)> {
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, rest) = text.split_at_checked(end)?;
    Some((digits.parse().ok()?, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u32, minor: u32) -> Option<TmuxVersion> {
        Some(TmuxVersion { major, minor })
    }

    #[test]
    fn reads_released_and_development_versions() {
        assert_eq!(TmuxVersion::parse("tmux 3.3a\n"), v(3, 3));
        assert_eq!(TmuxVersion::parse("tmux 3.1c"), v(3, 1));
        assert_eq!(TmuxVersion::parse("tmux 3.7b"), v(3, 7));
        assert_eq!(TmuxVersion::parse("tmux next-3.5"), v(3, 5));
        assert_eq!(TmuxVersion::parse("tmux 2.9"), v(2, 9));
    }

    #[test]
    fn a_build_without_a_number_is_unknown_not_old() {
        for text in [
            "tmux master",
            "tmux",
            "",
            "something 3",
            "tmux 3",
            "tmux x.y",
        ] {
            assert_eq!(TmuxVersion::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn the_minimum_is_inclusive_and_ordered_by_number_not_text() {
        assert!(v(3, 1).unwrap().is_supported());
        assert!(v(3, 10).unwrap().is_supported());
        assert!(v(4, 0).unwrap().is_supported());
        assert!(!v(3, 0).unwrap().is_supported());
        assert!(!v(2, 9).unwrap().is_supported());
    }
}
