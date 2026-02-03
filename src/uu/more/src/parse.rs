// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;

#[derive(Debug, PartialEq, Eq)]
pub enum ExtraArg {
    /// `-number` (also available as `--lines`)
    Lines(u16),
    /// `+number` (also available as `--from-line`)
    FromLine(usize),
    /// `+/string` (also available as `--pattern`)
    Pattern(String),
}

pub fn parse_extra_arg(src: &str) -> Option<ExtraArg> {
    if let Some(rest) = src.strip_prefix("+/") {
        if !rest.is_empty() {
            return Some(ExtraArg::Pattern(rest.to_string()));
        }
    } else if let Some(rest) = src.strip_prefix('+') {
        if let Ok(n) = rest.parse::<usize>() {
            return Some(ExtraArg::FromLine(n));
        }
    } else if let Some(rest) = src.strip_prefix('-') {
        if let Ok(n) = rest.parse::<u16>() {
            return Some(ExtraArg::Lines(n));
        }
    }
    None
}

fn expand_extra_arg(result: &mut Vec<OsString>, arg: ExtraArg) {
    match arg {
        ExtraArg::Lines(n) => {
            result.push("--lines".into());
            result.push(n.to_string().into());
        }
        ExtraArg::FromLine(n) => {
            result.push("--from-line".into());
            result.push(n.to_string().into());
        }
        ExtraArg::Pattern(p) => {
            result.push("--pattern".into());
            result.push(p.into());
        }
    }
}

pub fn preprocess_args(args: impl Iterator<Item = OsString>) -> Vec<OsString> {
    let mut result = Vec::new();
    for (i, arg) in args.enumerate() {
        if let Some(extra) = arg.to_str().and_then(parse_extra_arg) {
            if i == 0 {
                result.push(uucore::util_name().into());
            }
            expand_extra_arg(&mut result, extra);
        } else {
            result.push(arg);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minus_number() {
        assert_eq!(parse_extra_arg("-10"), Some(ExtraArg::Lines(10)));
        assert_eq!(parse_extra_arg("-1"), Some(ExtraArg::Lines(1)));
        assert_eq!(parse_extra_arg("-abc"), None);
        assert_eq!(parse_extra_arg("-"), None);
    }

    #[test]
    fn test_parse_plus_number() {
        assert_eq!(parse_extra_arg("+5"), Some(ExtraArg::FromLine(5)));
        assert_eq!(parse_extra_arg("+100"), Some(ExtraArg::FromLine(100)));
        assert_eq!(parse_extra_arg("+abc"), None);
    }

    #[test]
    fn test_parse_plus_pattern() {
        assert_eq!(
            parse_extra_arg("+/foo"),
            Some(ExtraArg::Pattern("foo".into()))
        );
        assert_eq!(
            parse_extra_arg("+/hello world"),
            Some(ExtraArg::Pattern("hello world".into()))
        );
        assert_eq!(parse_extra_arg("+/"), None);
    }

    #[test]
    fn test_preprocess_args() {
        // Test -number
        let args = ["more", "-5"];
        let result = preprocess_args(args.iter().map(OsString::from));
        assert_eq!(
            result,
            vec![
                OsString::from("more"),
                OsString::from("--lines"),
                OsString::from("5")
            ]
        );

        // Test +number
        let args = ["more", "+10"];
        let result = preprocess_args(args.iter().map(OsString::from));
        assert_eq!(
            result,
            vec![
                OsString::from("more"),
                OsString::from("--from-line"),
                OsString::from("10")
            ]
        );

        // Test +/pattern
        let args = ["more", "+/hello"];
        let result = preprocess_args(args.iter().map(OsString::from));
        assert_eq!(
            result,
            vec![
                OsString::from("more"),
                OsString::from("--pattern"),
                OsString::from("hello")
            ]
        );

        // Test regular args unchanged
        let args = ["more", "file.txt"];
        let result = preprocess_args(args.iter().map(OsString::from));
        assert_eq!(result, args.iter().map(OsString::from).collect::<Vec<_>>());
    }
}
