// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;

use crate::options;

/// Parses the util-linux extra args `-number`, `+number` and `+/string`,
/// returning the long flag they map to and its value.
pub fn parse_extra_arg(src: &str) -> Option<(&'static str, String)> {
    if let Some(rest) = src.strip_prefix("+/") {
        (!rest.is_empty()).then(|| (options::PATTERN, rest.to_string()))
    } else if let Some(rest) = src.strip_prefix('+') {
        let n = rest.parse::<usize>().ok()?;
        Some((options::FROM_LINE, n.to_string()))
    } else {
        let n = src.strip_prefix('-')?.parse::<u16>().ok()?;
        Some((options::LINES, n.to_string()))
    }
}

/// Returns whether `arg` is an option whose value is the next argument,
/// e.g. `--pattern` or `-n`, so that value must not be rewritten.
fn takes_value(arg: &str) -> bool {
    if let Some(long) = arg.strip_prefix("--") {
        // Long args may be abbreviated (`infer_long_args`).
        !long.is_empty()
            && !long.contains('=')
            && [options::LINES, options::FROM_LINE, options::PATTERN]
                .iter()
                .any(|name| name.starts_with(long))
    } else if let Some(short) = arg.strip_prefix('-') {
        // `-n` possibly clustered with other short flags, e.g. `-sn`.
        short.ends_with('n') && short.chars().all(|c| c.is_ascii_alphabetic())
    } else {
        false
    }
}

pub fn preprocess_args(args: impl Iterator<Item = OsString>) -> Vec<OsString> {
    let mut result = Vec::new();
    let mut is_value = false;
    for arg in args {
        let s = arg.to_str();
        if is_value {
            is_value = false;
        } else if let Some((flag, value)) = s.and_then(parse_extra_arg) {
            result.push(format!("--{flag}").into());
            result.push(value.into());
            continue;
        } else {
            is_value = s.is_some_and(takes_value);
        }
        result.push(arg);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preprocess(args: &[&str]) -> Vec<OsString> {
        preprocess_args(args.iter().map(OsString::from))
    }

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn test_parse_minus_number() {
        assert_eq!(parse_extra_arg("-10"), Some((options::LINES, "10".into())));
        assert_eq!(parse_extra_arg("-1"), Some((options::LINES, "1".into())));
        assert_eq!(parse_extra_arg("-abc"), None);
        assert_eq!(parse_extra_arg("-"), None);
    }

    #[test]
    fn test_parse_plus_number() {
        assert_eq!(
            parse_extra_arg("+5"),
            Some((options::FROM_LINE, "5".into()))
        );
        assert_eq!(
            parse_extra_arg("+100"),
            Some((options::FROM_LINE, "100".into()))
        );
        assert_eq!(parse_extra_arg("+abc"), None);
    }

    #[test]
    fn test_parse_plus_pattern() {
        assert_eq!(
            parse_extra_arg("+/foo"),
            Some((options::PATTERN, "foo".into()))
        );
        assert_eq!(
            parse_extra_arg("+/hello world"),
            Some((options::PATTERN, "hello world".into()))
        );
        assert_eq!(parse_extra_arg("+/"), None);
    }

    #[test]
    fn test_preprocess_args() {
        assert_eq!(preprocess(&["more", "-5"]), os(&["more", "--lines", "5"]));
        assert_eq!(
            preprocess(&["more", "+10"]),
            os(&["more", "--from-line", "10"])
        );
        assert_eq!(
            preprocess(&["more", "+/hello"]),
            os(&["more", "--pattern", "hello"])
        );
        assert_eq!(preprocess(&["more", "file.txt"]), os(&["more", "file.txt"]));
    }

    #[test]
    fn test_preprocess_args_option_values_unchanged() {
        for args in [
            ["more", "--pattern", "-5"],
            ["more", "--pat", "+5"],
            ["more", "--lines", "-5"],
            ["more", "--from-line", "+/x"],
            ["more", "-n", "-5"],
            ["more", "-sn", "+5"],
        ] {
            assert_eq!(preprocess(&args), os(&args));
        }
        assert_eq!(
            preprocess(&["more", "--pattern=x", "-5"]),
            os(&["more", "--pattern=x", "--lines", "5"])
        );
    }
}
