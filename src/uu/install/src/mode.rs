use std::fs;
use std::path::Path;
#[cfg(not(windows))]
use uucore::mode;
use uucore::translate;

/// Takes a user-supplied string and tries to parse to u16 mode bitmask.
/// Supports comma-separated mode strings like "ug+rwX,o+rX" (same as chmod).
pub fn parse(mode_string: &str, considering_dir: bool, umask: u32) -> Result<u32, String> {
    // Split by commas and process each mode part sequentially
    let mut current_mode: u32 = 0;

    for mode_part in mode_string.split(',') {
        let mode_part = mode_part.trim();
        if mode_part.is_empty() {
            continue;
        }

        current_mode = if mode_part.chars().any(|c| c.is_ascii_digit()) {
            mode::parse_numeric(current_mode, mode_part, considering_dir)?
        } else {
            mode::parse_symbolic(current_mode, mode_part, umask, considering_dir)?
        };
    }

    Ok(current_mode)
}

/// chmod a file or directory on UNIX.
///
/// Adapted from mkdir.rs.  Handles own error printing.
///
#[cfg(any(unix, target_os = "redox"))]
pub fn chmod(path: &Path, mode: u32) -> Result<(), ()> {
    use std::os::unix::fs::PermissionsExt;
    use uucore::{display::Quotable, show_error};
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|err| {
        show_error!(
            "{}",
            translate!("install-error-chmod-failed-detailed", "path" => path.maybe_quote(), "error" => err)
        );
    })
}

/// chmod a file or directory on Windows.
///
/// Adapted from mkdir.rs.
///
#[cfg(windows)]
pub fn chmod(path: &Path, mode: u32) -> Result<(), ()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}

#[cfg(test)]
#[cfg(not(windows))]
mod tests {
    use super::parse;

    #[test]
    fn test_parse_numeric_mode() {
        // Simple numeric mode
        assert_eq!(parse("644", false, 0).unwrap(), 0o644);
        assert_eq!(parse("755", false, 0).unwrap(), 0o755);
        assert_eq!(parse("777", false, 0).unwrap(), 0o777);
        assert_eq!(parse("600", false, 0).unwrap(), 0o600);
    }

    #[test]
    fn test_parse_numeric_mode_with_operator() {
        // Numeric mode with + operator
        assert_eq!(parse("+100", false, 0).unwrap(), 0o100);
        assert_eq!(parse("+644", false, 0).unwrap(), 0o644);

        // Numeric mode with - operator (starting from 0, so nothing to remove)
        assert_eq!(parse("-4", false, 0).unwrap(), 0);
        // But if we first set a mode, then remove bits
        assert_eq!(parse("644,-4", false, 0).unwrap(), 0o640);
    }

    #[test]
    fn test_parse_symbolic_mode() {
        // Simple symbolic modes
        assert_eq!(parse("u+x", false, 0).unwrap(), 0o100);
        assert_eq!(parse("g+w", false, 0).unwrap(), 0o020);
        assert_eq!(parse("o+r", false, 0).unwrap(), 0o004);
        assert_eq!(parse("a+x", false, 0).unwrap(), 0o111);
    }

    #[test]
    fn test_parse_symbolic_mode_multiple_permissions() {
        // Multiple permissions in one mode
        assert_eq!(parse("u+rw", false, 0).unwrap(), 0o600);
        assert_eq!(parse("ug+rwx", false, 0).unwrap(), 0o770);
        assert_eq!(parse("a+rwx", false, 0).unwrap(), 0o777);
    }

    #[test]
    fn test_parse_comma_separated_modes() {
        // Comma-separated mode strings (as mentioned in the doc comment)
        assert_eq!(parse("ug+rwX,o+rX", false, 0).unwrap(), 0o664);
        assert_eq!(parse("u+rwx,g+rx,o+r", false, 0).unwrap(), 0o754);
        assert_eq!(parse("u+w,g+w,o+w", false, 0).unwrap(), 0o222);
    }

    #[test]
    fn test_parse_comma_separated_with_spaces() {
        // Comma-separated with spaces (should be trimmed)
        assert_eq!(parse("u+rw, g+rw, o+r", false, 0).unwrap(), 0o664);
        assert_eq!(parse(" u+x , g+x ", false, 0).unwrap(), 0o110);
    }

    #[test]
    fn test_parse_mixed_numeric_and_symbolic() {
        // Mix of numeric and symbolic modes
        assert_eq!(parse("644,u+x", false, 0).unwrap(), 0o744);
        assert_eq!(parse("u+rw,755", false, 0).unwrap(), 0o755);
    }

    #[test]
    fn test_parse_empty_string() {
        // Empty string should return 0
        assert_eq!(parse("", false, 0).unwrap(), 0);
        assert_eq!(parse("   ", false, 0).unwrap(), 0);
        assert_eq!(parse(",,", false, 0).unwrap(), 0);
    }

    #[test]
    fn test_parse_with_umask() {
        // Test with umask (affects symbolic modes when no level is specified)
        let umask = 0o022;
        assert_eq!(parse("+w", false, umask).unwrap(), 0o200);
        // The umask should be respected for symbolic modes without explicit level
    }

    #[test]
    fn test_parse_considering_dir() {
        // Test directory vs file mode differences
        // For directories, X (capital X) should add execute permission
        assert_eq!(parse("a+X", true, 0).unwrap(), 0o111);
        // For files without execute, X should not add execute
        assert_eq!(parse("a+X", false, 0).unwrap(), 0o000);

        // Numeric modes for directories preserve setuid/setgid bits
        assert_eq!(parse("755", true, 0).unwrap(), 0o755);
    }

    #[test]
    fn test_parse_invalid_modes() {
        // Invalid numeric mode (too large)
        assert!(parse("10000", false, 0).is_err());

        // Invalid operator
        assert!(parse("u*rw", false, 0).is_err());

        // Invalid symbolic mode
        assert!(parse("invalid", false, 0).is_err());
    }

    #[test]
    fn test_parse_complex_combinations() {
        // Complex real-world examples
        assert_eq!(parse("u=rwx,g=rx,o=r", false, 0).unwrap(), 0o754);
        // To test removal, we need to first set permissions, then remove them
        assert_eq!(parse("644,a-w", false, 0).unwrap(), 0o444);
        assert_eq!(parse("644,g-r", false, 0).unwrap(), 0o604);
    }

    #[test]
    fn test_parse_sequential_application() {
        // Test that comma-separated modes are applied sequentially
        // First set to 644, then add execute for user
        assert_eq!(parse("644,u+x", false, 0).unwrap(), 0o744);

        // First add user write, then set to 755 (should override)
        assert_eq!(parse("u+w,755", false, 0).unwrap(), 0o755);
    }
}
