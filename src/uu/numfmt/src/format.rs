// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore powf
use uucore::display::Quotable;
use uucore::translate;

use crate::options::{NumfmtOptions, RoundMethod, TransformOptions};
use crate::units::{DisplayableSuffix, IEC_BASES, RawSuffix, Result, SI_BASES, Suffix, Unit};

/// Iterate over a line's fields, where each field is a contiguous sequence of
/// non-whitespace, optionally prefixed with one or more characters of leading
/// whitespace. Fields are returned as tuples of `(prefix, field)`.
///
/// # Examples:
///
/// ```
/// let mut fields = uu_numfmt::format::WhitespaceSplitter { s: Some("    1234 5") };
///
/// assert_eq!(Some(("    ", "1234")), fields.next());
/// assert_eq!(Some((" ", "5")), fields.next());
/// assert_eq!(None, fields.next());
/// ```
///
/// Delimiters are included in the results; `prefix` will be empty only for
/// the first field of the line (including the case where the input line is
/// empty):
///
/// ```
/// let mut fields = uu_numfmt::format::WhitespaceSplitter { s: Some("first second") };
///
/// assert_eq!(Some(("", "first")), fields.next());
/// assert_eq!(Some((" ", "second")), fields.next());
///
/// let mut fields = uu_numfmt::format::WhitespaceSplitter { s: Some("") };
///
/// assert_eq!(Some(("", "")), fields.next());
/// ```
pub struct WhitespaceSplitter<'a> {
    pub s: Option<&'a str>,
}

impl<'a> Iterator for WhitespaceSplitter<'a> {
    type Item = (&'a str, &'a str);

    /// Yield the next field in the input string as a tuple `(prefix, field)`.
    fn next(&mut self) -> Option<Self::Item> {
        let haystack = self.s?;

        let (prefix, field) = haystack.split_at(
            haystack
                .find(|c: char| !c.is_whitespace())
                .unwrap_or(haystack.len()),
        );

        let (field, rest) = field.split_at(field.find(char::is_whitespace).unwrap_or(field.len()));

        self.s = if rest.is_empty() { None } else { Some(rest) };

        Some((prefix, field))
    }
}

fn parse_suffix(s: &str) -> Result<(f64, Option<Suffix>)> {
    if s.is_empty() {
        return Err(translate!("numfmt-error-invalid-number-empty"));
    }

    let with_i = s.ends_with('i');
    let mut iter = s.chars();
    if with_i {
        iter.next_back();
    }
    let suffix = match iter.next_back() {
        Some('K') => Some((RawSuffix::K, with_i)),
        Some('k') => Some((RawSuffix::K, with_i)),
        Some('M') => Some((RawSuffix::M, with_i)),
        Some('G') => Some((RawSuffix::G, with_i)),
        Some('T') => Some((RawSuffix::T, with_i)),
        Some('P') => Some((RawSuffix::P, with_i)),
        Some('E') => Some((RawSuffix::E, with_i)),
        Some('Z') => Some((RawSuffix::Z, with_i)),
        Some('Y') => Some((RawSuffix::Y, with_i)),
        Some('R') => Some((RawSuffix::R, with_i)),
        Some('Q') => Some((RawSuffix::Q, with_i)),
        Some('0'..='9') if !with_i => None,
        _ => {
            // If with_i is true, the string ends with 'i' but there's no valid suffix letter
            // This is always an invalid suffix (e.g., "1i", "2Ai")
            if with_i {
                return Err(translate!("numfmt-error-invalid-suffix", "input" => s.quote()));
            }
            // For other cases, check if the number part (without the last character) is valid
            let number_part = &s[..s.len() - 1];
            if number_part.is_empty() || number_part.parse::<f64>().is_err() {
                return Err(translate!("numfmt-error-invalid-number", "input" => s.quote()));
            }
            return Err(translate!("numfmt-error-invalid-suffix", "input" => s.quote()));
        }
    };

    let suffix_len = match suffix {
        None => 0,
        Some((_, false)) => 1,
        Some((_, true)) => 2,
    };

    let number = s[..s.len() - suffix_len]
        .parse::<f64>()
        .map_err(|_| translate!("numfmt-error-invalid-number", "input" => s.quote()))?;

    Ok((number, suffix))
}

/// Returns the implicit precision of a number, which is the count of digits after the dot. For
/// example, 1.23 has an implicit precision of 2.
fn parse_implicit_precision(s: &str) -> usize {
    match s.split_once('.') {
        Some((_, decimal_part)) => decimal_part
            .chars()
            .take_while(char::is_ascii_digit)
            .count(),
        None => 0,
    }
}

fn remove_suffix(i: f64, s: Option<Suffix>, u: &Unit) -> Result<f64> {
    match (s, u) {
        (Some((raw_suffix, false)), &Unit::Auto | &Unit::Si) => match raw_suffix {
            RawSuffix::K => Ok(i * 1e3),
            RawSuffix::M => Ok(i * 1e6),
            RawSuffix::G => Ok(i * 1e9),
            RawSuffix::T => Ok(i * 1e12),
            RawSuffix::P => Ok(i * 1e15),
            RawSuffix::E => Ok(i * 1e18),
            RawSuffix::Z => Ok(i * 1e21),
            RawSuffix::Y => Ok(i * 1e24),
            RawSuffix::R => Ok(i * 1e27),
            RawSuffix::Q => Ok(i * 1e30),
        },
        (Some((raw_suffix, false)), &Unit::Iec(false))
        | (Some((raw_suffix, true)), &Unit::Auto | &Unit::Iec(true)) => match raw_suffix {
            RawSuffix::K => Ok(i * IEC_BASES[1]),
            RawSuffix::M => Ok(i * IEC_BASES[2]),
            RawSuffix::G => Ok(i * IEC_BASES[3]),
            RawSuffix::T => Ok(i * IEC_BASES[4]),
            RawSuffix::P => Ok(i * IEC_BASES[5]),
            RawSuffix::E => Ok(i * IEC_BASES[6]),
            RawSuffix::Z => Ok(i * IEC_BASES[7]),
            RawSuffix::Y => Ok(i * IEC_BASES[8]),
            RawSuffix::R => Ok(i * IEC_BASES[9]),
            RawSuffix::Q => Ok(i * IEC_BASES[10]),
        },
        (Some((raw_suffix, false)), &Unit::Iec(true)) => Err(
            translate!("numfmt-error-missing-i-suffix", "number" => i, "suffix" => format!("{raw_suffix:?}")),
        ),
        (Some((raw_suffix, with_i)), &Unit::None) => Err(
            translate!("numfmt-error-rejecting-suffix", "number" => i, "suffix" => format!("{raw_suffix:?}{}", if with_i { "i" } else { "" })),
        ),
        (None, _) => Ok(i),
        (_, _) => Err(translate!("numfmt-error-suffix-unsupported-for-unit")),
    }
}

fn transform_from(s: &str, opts: &TransformOptions) -> Result<f64> {
    let (i, suffix) = parse_suffix(s)?;
    let i = i * (opts.from_unit as f64);

    remove_suffix(i, suffix, &opts.from).map(|n| {
        // GNU numfmt doesn't round values if no --from argument is provided by the user
        if opts.from == Unit::None {
            if n == -0.0 { 0.0 } else { n }
        } else if n < 0.0 {
            -n.abs().ceil()
        } else {
            n.ceil()
        }
    })
}

/// Divide numerator by denominator, with rounding.
///
/// If the result of the division is less than 10.0, round to one decimal point.
///
/// Otherwise, round to an integer.
///
/// # Examples:
///
/// ```
/// use uu_numfmt::format::div_round;
/// use uu_numfmt::options::RoundMethod;
///
/// // Rounding methods:
/// assert_eq!(div_round(1.01, 1.0, RoundMethod::FromZero), 1.1);
/// assert_eq!(div_round(1.01, 1.0, RoundMethod::TowardsZero), 1.0);
/// assert_eq!(div_round(1.01, 1.0, RoundMethod::Up), 1.1);
/// assert_eq!(div_round(1.01, 1.0, RoundMethod::Down), 1.0);
/// assert_eq!(div_round(1.01, 1.0, RoundMethod::Nearest), 1.0);
///
/// // Division:
/// assert_eq!(div_round(999.1, 1000.0, RoundMethod::FromZero), 1.0);
/// assert_eq!(div_round(1001., 10., RoundMethod::FromZero), 101.);
/// assert_eq!(div_round(9991., 10., RoundMethod::FromZero), 1000.);
/// assert_eq!(div_round(-12.34, 1.0, RoundMethod::FromZero), -13.0);
/// assert_eq!(div_round(1000.0, -3.14, RoundMethod::FromZero), -319.0);
/// assert_eq!(div_round(-271828.0, -271.0, RoundMethod::FromZero), 1004.0);
/// ```
pub fn div_round(n: f64, d: f64, method: RoundMethod) -> f64 {
    let v = n / d;

    if v.abs() < 10.0 {
        method.round(10.0 * v) / 10.0
    } else {
        method.round(v)
    }
}

/// Rounds to the specified number of decimal points.
fn round_with_precision(n: f64, method: RoundMethod, precision: usize) -> f64 {
    let p = 10.0_f64.powf(precision as f64);

    method.round(p * n) / p
}

fn consider_suffix(
    n: f64,
    u: &Unit,
    round_method: RoundMethod,
    precision: usize,
) -> Result<(f64, Option<Suffix>)> {
    use crate::units::RawSuffix::{E, G, K, M, P, Q, R, T, Y, Z};

    let abs_n = n.abs();
    let suffixes = [K, M, G, T, P, E, Z, Y, R, Q];

    let (bases, with_i) = match *u {
        Unit::Si => (&SI_BASES, false),
        Unit::Iec(with_i) => (&IEC_BASES, with_i),
        Unit::Auto => return Err(translate!("numfmt-error-unit-auto-not-supported-with-to")),
        Unit::None => return Ok((n, None)),
    };

    let i = match abs_n {
        _ if abs_n <= bases[1] - 1.0 => return Ok((n, None)),
        _ if abs_n < bases[2] => 1,
        _ if abs_n < bases[3] => 2,
        _ if abs_n < bases[4] => 3,
        _ if abs_n < bases[5] => 4,
        _ if abs_n < bases[6] => 5,
        _ if abs_n < bases[7] => 6,
        _ if abs_n < bases[8] => 7,
        _ if abs_n < bases[9] => 8,
        _ if abs_n < bases[10] => 9,
        _ if abs_n < bases[10] * 1000.0 => 10,
        _ => return Err(translate!("numfmt-error-number-too-big")),
    };

    let v = if precision > 0 {
        round_with_precision(n / bases[i], round_method, precision)
    } else {
        div_round(n, bases[i], round_method)
    };

    // check if rounding pushed us into the next base
    if v.abs() >= bases[1] {
        Ok((v / bases[1], Some((suffixes[i], with_i))))
    } else {
        Ok((v, Some((suffixes[i - 1], with_i))))
    }
}

fn transform_to(
    s: f64,
    opts: &TransformOptions,
    round_method: RoundMethod,
    precision: usize,
) -> Result<String> {
    let (i2, s) = consider_suffix(s, &opts.to, round_method, precision)?;
    let i2 = i2 / (opts.to_unit as f64);
    Ok(match s {
        None => {
            format!(
                "{:.precision$}",
                round_with_precision(i2, round_method, precision),
            )
        }
        Some(s) if precision > 0 => {
            format!("{i2:.precision$}{}", DisplayableSuffix(s, opts.to),)
        }
        Some(s) if i2.abs() < 10.0 => format!("{i2:.1}{}", DisplayableSuffix(s, opts.to)),
        Some(s) => format!("{i2:.0}{}", DisplayableSuffix(s, opts.to)),
    })
}

fn format_string(
    source: &str,
    options: &NumfmtOptions,
    implicit_padding: Option<isize>,
) -> Result<String> {
    // strip the (optional) suffix before applying any transformation
    let source_without_suffix = match &options.suffix {
        Some(suffix) => source.strip_suffix(suffix).unwrap_or(source),
        None => source,
    };

    let precision = if let Some(p) = options.format.precision {
        p
    } else if options.transform.from == Unit::None && options.transform.to == Unit::None {
        parse_implicit_precision(source_without_suffix)
    } else {
        0
    };

    let number = transform_to(
        transform_from(source_without_suffix, &options.transform)?,
        &options.transform,
        options.round,
        precision,
    )?;

    // bring back the suffix before applying padding
    let number_with_suffix = match &options.suffix {
        Some(suffix) => format!("{number}{suffix}"),
        None => number,
    };

    let padding = options
        .format
        .padding
        .unwrap_or_else(|| implicit_padding.unwrap_or(options.padding));

    let padded_number = match padding {
        0 => number_with_suffix,
        p if p > 0 && options.format.zero_padding => {
            let zero_padded = format!("{number_with_suffix:0>padding$}", padding = p as usize);

            match implicit_padding.unwrap_or(options.padding) {
                0 => zero_padded,
                p if p > 0 => format!("{zero_padded:>padding$}", padding = p as usize),
                p => format!("{zero_padded:<padding$}", padding = p.unsigned_abs()),
            }
        }
        p if p > 0 => format!("{number_with_suffix:>padding$}", padding = p as usize),
        p => format!("{number_with_suffix:<padding$}", padding = p.unsigned_abs()),
    };

    Ok(format!(
        "{}{padded_number}{}",
        options.format.prefix, options.format.suffix
    ))
}

fn format_and_print_delimited(s: &str, options: &NumfmtOptions) -> Result<()> {
    let delimiter = options.delimiter.as_ref().unwrap();
    let mut output = String::new();

    for (n, field) in (1..).zip(s.split(delimiter)) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        // add delimiter before second and subsequent fields
        if n > 1 {
            output.push_str(delimiter);
        }

        if field_selected {
            output.push_str(&format_string(field.trim_start(), options, None)?);
        } else {
            // add unselected field without conversion
            output.push_str(field);
        }
    }

    println!("{output}");

    Ok(())
}

fn format_and_print_whitespace(s: &str, options: &NumfmtOptions) -> Result<()> {
    let mut output = String::new();

    for (n, (prefix, field)) in (1..).zip(WhitespaceSplitter { s: Some(s) }) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        if field_selected {
            let empty_prefix = prefix.is_empty();

            // add delimiter before second and subsequent fields
            let prefix = if n > 1 {
                output.push(' ');
                &prefix[1..]
            } else {
                prefix
            };

            let implicit_padding = if !empty_prefix && options.padding == 0 {
                Some((prefix.len() + field.len()) as isize)
            } else {
                None
            };

            output.push_str(&format_string(field, options, implicit_padding)?);
        } else {
            // the -z option converts an initial \n into a space
            let prefix = if options.zero_terminated && prefix.starts_with('\n') {
                output.push(' ');
                &prefix[1..]
            } else {
                prefix
            };
            // add unselected field without conversion
            output.push_str(prefix);
            output.push_str(field);
        }
    }

    let eol = if options.zero_terminated { '\0' } else { '\n' };
    output.push(eol);
    print!("{output}");

    Ok(())
}

/// Format a line of text according to the selected options.
///
/// Given a line of text `s`, split the line into fields, transform and format
/// any selected numeric fields, and print the result to stdout. Fields not
/// selected for conversion are passed through unmodified.
pub fn format_and_print(s: &str, options: &NumfmtOptions) -> Result<()> {
    match &options.delimiter {
        Some(_) => format_and_print_delimited(s, options),
        None => format_and_print_whitespace(s, options),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_with_precision() {
        let rm = RoundMethod::FromZero;
        assert_eq!(1.0, round_with_precision(0.12345, rm, 0));
        assert_eq!(0.2, round_with_precision(0.12345, rm, 1));
        assert_eq!(0.13, round_with_precision(0.12345, rm, 2));
        assert_eq!(0.124, round_with_precision(0.12345, rm, 3));
        assert_eq!(0.1235, round_with_precision(0.12345, rm, 4));
        assert_eq!(0.12345, round_with_precision(0.12345, rm, 5));

        let rm = RoundMethod::TowardsZero;
        assert_eq!(0.0, round_with_precision(0.12345, rm, 0));
        assert_eq!(0.1, round_with_precision(0.12345, rm, 1));
        assert_eq!(0.12, round_with_precision(0.12345, rm, 2));
        assert_eq!(0.123, round_with_precision(0.12345, rm, 3));
        assert_eq!(0.1234, round_with_precision(0.12345, rm, 4));
        assert_eq!(0.12345, round_with_precision(0.12345, rm, 5));
    }

    #[test]
    fn test_parse_implicit_precision() {
        assert_eq!(0, parse_implicit_precision(""));
        assert_eq!(0, parse_implicit_precision("1"));
        assert_eq!(1, parse_implicit_precision("1.2"));
        assert_eq!(2, parse_implicit_precision("1.23"));
        assert_eq!(3, parse_implicit_precision("1.234"));
        assert_eq!(0, parse_implicit_precision("1K"));
        assert_eq!(1, parse_implicit_precision("1.2K"));
        assert_eq!(2, parse_implicit_precision("1.23K"));
        assert_eq!(3, parse_implicit_precision("1.234K"));
    }

    #[test]
    fn test_parse_suffix_q_r_k() {
        let result = parse_suffix("1Q");
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 1.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(!with_i);

        let result = parse_suffix("2R");
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 2.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert!(!with_i);

        let result = parse_suffix("3k");
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 3.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::K as i32);
        assert!(!with_i);

        let result = parse_suffix("4Qi");
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 4.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(with_i);

        let result = parse_suffix("5Ri");
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 5.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert!(with_i);
    }

    #[test]
    fn test_parse_suffix_error_messages() {
        let result = parse_suffix("foo");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("numfmt-error-invalid-number") || error.contains("invalid number"));
        assert!(!error.contains("invalid suffix"));

        let result = parse_suffix("World");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("numfmt-error-invalid-number") || error.contains("invalid number"));
        assert!(!error.contains("invalid suffix"));

        let result = parse_suffix("123i");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("numfmt-error-invalid-suffix") || error.contains("invalid suffix"));
    }

    #[test]
    fn test_remove_suffix_q_r() {
        use crate::units::Unit;

        let result = remove_suffix(1.0, Some((RawSuffix::Q, false)), &Unit::Si);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1e30);

        let result = remove_suffix(1.0, Some((RawSuffix::R, false)), &Unit::Si);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1e27);

        let result = remove_suffix(1.0, Some((RawSuffix::Q, true)), &Unit::Iec(true));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), IEC_BASES[10]);

        let result = remove_suffix(1.0, Some((RawSuffix::R, true)), &Unit::Iec(true));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), IEC_BASES[9]);
    }

    #[test]
    fn test_consider_suffix_q_r() {
        use crate::options::RoundMethod;
        use crate::units::Unit;

        let result = consider_suffix(1e27, &Unit::Si, RoundMethod::FromZero, 0);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert_eq!(value, 1.0);

        let result = consider_suffix(1e30, &Unit::Si, RoundMethod::FromZero, 0);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert_eq!(value, 1.0);

        let result = consider_suffix(5e30, &Unit::Si, RoundMethod::FromZero, 0);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert_eq!(value, 5.0);
    }
}
