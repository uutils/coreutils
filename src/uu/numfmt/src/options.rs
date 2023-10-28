// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::str::FromStr;

use crate::units::Unit;
use uucore::ranges::Range;

pub const DELIMITER: &str = "delimiter";
pub const FIELD: &str = "field";
pub const FIELD_DEFAULT: &str = "1";
pub const FORMAT: &str = "format";
pub const FROM: &str = "from";
pub const FROM_DEFAULT: &str = "none";
pub const FROM_UNIT: &str = "from-unit";
pub const FROM_UNIT_DEFAULT: &str = "1";
pub const HEADER: &str = "header";
pub const HEADER_DEFAULT: &str = "1";
pub const INVALID: &str = "invalid";
pub const NUMBER: &str = "NUMBER";
pub const PADDING: &str = "padding";
pub const ROUND: &str = "round";
pub const SUFFIX: &str = "suffix";
pub const TO: &str = "to";
pub const TO_DEFAULT: &str = "none";
pub const TO_UNIT: &str = "to-unit";
pub const TO_UNIT_DEFAULT: &str = "1";

pub struct TransformOptions {
    pub from: Unit,
    pub from_unit: usize,
    pub to: Unit,
    pub to_unit: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidModes {
    Abort,
    Fail,
    Warn,
    Ignore,
}

pub struct NumfmtOptions {
    pub transform: TransformOptions,
    pub padding: isize,
    pub header: usize,
    pub fields: Vec<Range>,
    pub delimiter: Option<String>,
    pub round: RoundMethod,
    pub suffix: Option<String>,
    pub format: FormatOptions,
    pub invalid: InvalidModes,
}

#[derive(Clone, Copy)]
pub enum RoundMethod {
    Up,
    Down,
    FromZero,
    TowardsZero,
    Nearest,
}

impl RoundMethod {
    pub fn round(&self, f: f64) -> f64 {
        match self {
            Self::Up => f.ceil(),
            Self::Down => f.floor(),
            Self::FromZero => {
                if f < 0.0 {
                    f.floor()
                } else {
                    f.ceil()
                }
            }
            Self::TowardsZero => {
                if f < 0.0 {
                    f.ceil()
                } else {
                    f.floor()
                }
            }
            Self::Nearest => f.round(),
        }
    }
}

// Represents the options extracted from the --format argument provided by the user.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FormatOptions {
    pub grouping: bool,
    pub padding: Option<isize>,
    pub precision: Option<usize>,
    pub prefix: String,
    pub suffix: String,
    pub zero_padding: bool,
}

impl FromStr for FormatOptions {
    type Err = String;

    // The recognized format is: [PREFIX]%[0]['][-][N][.][N]f[SUFFIX]
    //
    // The format defines the printing of a floating point argument '%f'.
    // An optional quote (%'f) enables --grouping.
    // An optional width value (%10f) will pad the number.
    // An optional zero (%010f) will zero pad the number.
    // An optional negative value (%-10f) will left align.
    // An optional precision (%.1f) determines the precision of the number.
    #[allow(clippy::cognitive_complexity)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.chars().peekable();
        let mut options = Self::default();

        let mut padding = String::new();
        let mut precision = String::new();
        let mut double_percentage_counter = 0;

        // '%' chars in the prefix, if any, must appear in blocks of even length, for example: "%%%%" and
        // "%% %%" are ok, "%%% %" is not ok. A single '%' is treated as the beginning of the
        // floating point argument.
        while let Some(c) = iter.next() {
            match c {
                '%' if iter.peek() == Some(&'%') => {
                    iter.next();
                    double_percentage_counter += 1;

                    for _ in 0..2 {
                        options.prefix.push('%');
                    }
                }
                '%' => break,
                _ => options.prefix.push(c),
            }
        }

        // GNU numfmt drops a char from the prefix for every '%%' in the prefix, so we do the same
        for _ in 0..double_percentage_counter {
            options.prefix.pop();
        }

        if iter.peek().is_none() {
            return if options.prefix == s {
                Err(format!("format '{s}' has no % directive"))
            } else {
                Err(format!("format '{s}' ends in %"))
            };
        }

        // GNU numfmt allows to mix the characters " ", "'", and "0" in any way, so we do the same
        while matches!(iter.peek(), Some(' ' | '\'' | '0')) {
            match iter.next().unwrap() {
                ' ' => (),
                '\'' => options.grouping = true,
                '0' => options.zero_padding = true,
                _ => unreachable!(),
            }
        }

        if let Some('-') = iter.peek() {
            iter.next();

            match iter.peek() {
                Some(c) if c.is_ascii_digit() => padding.push('-'),
                _ => {
                    return Err(format!(
                        "invalid format '{s}', directive must be %[0]['][-][N][.][N]f"
                    ))
                }
            }
        }

        while let Some(c) = iter.peek() {
            if c.is_ascii_digit() {
                padding.push(*c);
                iter.next();
            } else {
                break;
            }
        }

        if !padding.is_empty() {
            if let Ok(p) = padding.parse() {
                options.padding = Some(p);
            } else {
                return Err(format!("invalid format '{s}' (width overflow)"));
            }
        }

        if let Some('.') = iter.peek() {
            iter.next();

            if matches!(iter.peek(), Some(' ' | '+' | '-')) {
                return Err(format!("invalid precision in format '{s}'"));
            }

            while let Some(c) = iter.peek() {
                if c.is_ascii_digit() {
                    precision.push(*c);
                    iter.next();
                } else {
                    break;
                }
            }

            if precision.is_empty() {
                options.precision = Some(0);
            } else if let Ok(p) = precision.parse() {
                options.precision = Some(p);
            } else {
                return Err(format!("invalid precision in format '{s}'"));
            }
        }

        if let Some('f') = iter.peek() {
            iter.next();
        } else {
            return Err(format!(
                "invalid format '{s}', directive must be %[0]['][-][N][.][N]f"
            ));
        }

        // '%' chars in the suffix, if any, must appear in blocks of even length, otherwise
        // it is an error. For example: "%%%%" and "%% %%" are ok, "%%% %" is not ok.
        while let Some(c) = iter.next() {
            if c != '%' {
                options.suffix.push(c);
            } else if iter.peek() == Some(&'%') {
                for _ in 0..2 {
                    options.suffix.push('%');
                }
                iter.next();
            } else {
                return Err(format!("format '{s}' has too many % directives"));
            }
        }

        Ok(options)
    }
}

impl FromStr for InvalidModes {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "abort" => Ok(Self::Abort),
            "fail" => Ok(Self::Fail),
            "warn" => Ok(Self::Warn),
            "ignore" => Ok(Self::Ignore),
            unknown => Err(format!("Unknown invalid mode: {unknown}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_format() {
        assert_eq!(FormatOptions::default(), "%f".parse().unwrap());
        assert_eq!(FormatOptions::default(), "%  f".parse().unwrap());
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_parse_format_with_invalid_formats() {
        assert!("".parse::<FormatOptions>().is_err());
        assert!("hello".parse::<FormatOptions>().is_err());
        assert!("hello%".parse::<FormatOptions>().is_err());
        assert!("%-f".parse::<FormatOptions>().is_err());
        assert!("%d".parse::<FormatOptions>().is_err());
        assert!("%4 f".parse::<FormatOptions>().is_err());
        assert!("%f%".parse::<FormatOptions>().is_err());
        assert!("%f%%%".parse::<FormatOptions>().is_err());
        assert!("%%f".parse::<FormatOptions>().is_err());
        assert!("%%%%f".parse::<FormatOptions>().is_err());
        assert!("%.-1f".parse::<FormatOptions>().is_err());
        assert!("%. 1f".parse::<FormatOptions>().is_err());
        assert!("%18446744073709551616f".parse::<FormatOptions>().is_err());
        assert!("%.18446744073709551616f".parse::<FormatOptions>().is_err());
    }

    #[test]
    fn test_parse_format_with_prefix_and_suffix() {
        let formats = vec![
            ("--%f", "--", ""),
            ("%f::", "", "::"),
            ("--%f::", "--", "::"),
            ("%f%%", "", "%%"),
            ("%%%f", "%", ""),
            ("%% %f", "%%", ""),
        ];

        for (format, expected_prefix, expected_suffix) in formats {
            let options: FormatOptions = format.parse().unwrap();
            assert_eq!(expected_prefix, options.prefix);
            assert_eq!(expected_suffix, options.suffix);
        }
    }

    #[test]
    fn test_parse_format_with_padding() {
        let mut expected_options = FormatOptions::default();
        let formats = vec![("%12f", Some(12)), ("%-12f", Some(-12))];

        for (format, expected_padding) in formats {
            expected_options.padding = expected_padding;
            assert_eq!(expected_options, format.parse().unwrap());
        }
    }

    #[test]
    fn test_parse_format_with_precision() {
        let mut expected_options = FormatOptions::default();
        let formats = vec![
            ("%6.2f", Some(6), Some(2)),
            ("%6.f", Some(6), Some(0)),
            ("%.2f", None, Some(2)),
            ("%.f", None, Some(0)),
        ];

        for (format, expected_padding, expected_precision) in formats {
            expected_options.padding = expected_padding;
            expected_options.precision = expected_precision;
            assert_eq!(expected_options, format.parse().unwrap());
        }
    }

    #[test]
    fn test_parse_format_with_grouping() {
        let expected_options = FormatOptions {
            grouping: true,
            ..Default::default()
        };
        assert_eq!(expected_options, "%'f".parse().unwrap());
        assert_eq!(expected_options, "% ' f".parse().unwrap());
        assert_eq!(expected_options, "%'''''''f".parse().unwrap());
    }

    #[test]
    fn test_parse_format_with_zero_padding() {
        let expected_options = FormatOptions {
            padding: Some(10),
            zero_padding: true,
            ..Default::default()
        };
        assert_eq!(expected_options, "%010f".parse().unwrap());
        assert_eq!(expected_options, "% 0 10f".parse().unwrap());
        assert_eq!(expected_options, "%0000000010f".parse().unwrap());
    }

    #[test]
    fn test_parse_format_with_grouping_and_zero_padding() {
        let expected_options = FormatOptions {
            grouping: true,
            zero_padding: true,
            ..Default::default()
        };
        assert_eq!(expected_options, "%0'f".parse().unwrap());
        assert_eq!(expected_options, "%'0f".parse().unwrap());
        assert_eq!(expected_options, "%0'0'0'f".parse().unwrap());
        assert_eq!(expected_options, "%'0'0'0f".parse().unwrap());
    }

    #[test]
    fn test_set_invalid_mode() {
        assert_eq!(Ok(InvalidModes::Abort), InvalidModes::from_str("abort"));
        assert_eq!(Ok(InvalidModes::Abort), InvalidModes::from_str("ABORT"));

        assert_eq!(Ok(InvalidModes::Fail), InvalidModes::from_str("fail"));
        assert_eq!(Ok(InvalidModes::Fail), InvalidModes::from_str("FAIL"));

        assert_eq!(Ok(InvalidModes::Ignore), InvalidModes::from_str("ignore"));
        assert_eq!(Ok(InvalidModes::Ignore), InvalidModes::from_str("IGNORE"));

        assert_eq!(Ok(InvalidModes::Warn), InvalidModes::from_str("warn"));
        assert_eq!(Ok(InvalidModes::Warn), InvalidModes::from_str("WARN"));

        assert!(InvalidModes::from_str("something unknown").is_err());
    }
}
