// SPDX-License-Identifier: MIT

//! `flight --version`: which release and which commit this binary is.

/// The text printed for `--version`.
pub fn line() -> String {
    format!(
        "flight {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("FLIGHT_BUILD_COMMIT")
    )
}

/// True when the first argument asks for the version.
pub fn asked(args: &[String]) -> bool {
    matches!(args.first().map(String::as_str), Some("--version" | "-V"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_names_the_version_and_a_commit() {
        let line = line();
        let (name, rest) = line.split_once(' ').unwrap();
        assert_eq!(name, "flight");
        let (version, commit) = rest.split_once(" (").unwrap();
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        let commit = commit.strip_suffix(')').unwrap();
        assert!(commit == "unknown" || commit.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn only_a_leading_flag_asks() {
        let a = |s: &[&str]| s.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        assert!(asked(&a(&["--version"])));
        assert!(asked(&a(&["-V"])));
        assert!(!asked(&a(&["node", "--version"])));
        assert!(!asked(&a(&[])));
    }
}
