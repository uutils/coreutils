use uucore::display::Quotable;

use crate::options::{NumfmtOptions, RoundMethod};
use crate::units::{DisplayableSuffix, RawSuffix, Result, Suffix, Unit, IEC_BASES, SI_BASES};

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

        let (field, rest) = field.split_at(
            field
                .find(|c: char| c.is_whitespace())
                .unwrap_or(field.len()),
        );

        self.s = if !rest.is_empty() { Some(rest) } else { None };

        Some((prefix, field))
    }
}

fn parse_suffix(s: &str) -> Result<(f64, Option<Suffix>)> {
    if s.is_empty() {
        return Err("invalid number: ''".to_string());
    }

    let with_i = s.ends_with('i');
    let mut iter = s.chars();
    if with_i {
        iter.next_back();
    }
    let suffix = match iter.next_back() {
        Some('K') => Some((RawSuffix::K, with_i)),
        Some('M') => Some((RawSuffix::M, with_i)),
        Some('G') => Some((RawSuffix::G, with_i)),
        Some('T') => Some((RawSuffix::T, with_i)),
        Some('P') => Some((RawSuffix::P, with_i)),
        Some('E') => Some((RawSuffix::E, with_i)),
        Some('Z') => Some((RawSuffix::Z, with_i)),
        Some('Y') => Some((RawSuffix::Y, with_i)),
        Some('0'..='9') => None,
        _ => return Err(format!("invalid suffix in input: {}", s.quote())),
    };

    let suffix_len = match suffix {
        None => 0,
        Some((_, false)) => 1,
        Some((_, true)) => 2,
    };

    let number = s[..s.len() - suffix_len]
        .parse::<f64>()
        .map_err(|_| format!("invalid number: {}", s.quote()))?;

    Ok((number, suffix))
}

fn remove_suffix(i: f64, s: Option<Suffix>, u: &Unit) -> Result<f64> {
    match (s, u) {
        (None, _) => Ok(i),
        (Some((raw_suffix, false)), &Unit::Auto) | (Some((raw_suffix, false)), &Unit::Si) => {
            match raw_suffix {
                RawSuffix::K => Ok(i * 1e3),
                RawSuffix::M => Ok(i * 1e6),
                RawSuffix::G => Ok(i * 1e9),
                RawSuffix::T => Ok(i * 1e12),
                RawSuffix::P => Ok(i * 1e15),
                RawSuffix::E => Ok(i * 1e18),
                RawSuffix::Z => Ok(i * 1e21),
                RawSuffix::Y => Ok(i * 1e24),
            }
        }
        (Some((raw_suffix, false)), &Unit::Iec(false))
        | (Some((raw_suffix, true)), &Unit::Auto)
        | (Some((raw_suffix, true)), &Unit::Iec(true)) => match raw_suffix {
            RawSuffix::K => Ok(i * IEC_BASES[1]),
            RawSuffix::M => Ok(i * IEC_BASES[2]),
            RawSuffix::G => Ok(i * IEC_BASES[3]),
            RawSuffix::T => Ok(i * IEC_BASES[4]),
            RawSuffix::P => Ok(i * IEC_BASES[5]),
            RawSuffix::E => Ok(i * IEC_BASES[6]),
            RawSuffix::Z => Ok(i * IEC_BASES[7]),
            RawSuffix::Y => Ok(i * IEC_BASES[8]),
        },
        (_, _) => Err("This suffix is unsupported for specified unit".to_owned()),
    }
}

fn transform_from(s: &str, opts: &Unit) -> Result<f64> {
    let (i, suffix) = parse_suffix(s)?;

    remove_suffix(i, suffix, opts).map(|n| if n < 0.0 { -n.abs().ceil() } else { n.ceil() })
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

fn consider_suffix(n: f64, u: &Unit, round_method: RoundMethod) -> Result<(f64, Option<Suffix>)> {
    use crate::units::RawSuffix::*;

    let abs_n = n.abs();
    let suffixes = [K, M, G, T, P, E, Z, Y];

    let (bases, with_i) = match *u {
        Unit::Si => (&SI_BASES, false),
        Unit::Iec(with_i) => (&IEC_BASES, with_i),
        Unit::Auto => return Err("Unit 'auto' isn't supported with --to options".to_owned()),
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
        _ => return Err("Number is too big and unsupported".to_string()),
    };

    let v = div_round(n, bases[i], round_method);

    // check if rounding pushed us into the next base
    if v.abs() >= bases[1] {
        Ok((v / bases[1], Some((suffixes[i], with_i))))
    } else {
        Ok((v, Some((suffixes[i - 1], with_i))))
    }
}

fn transform_to(s: f64, opts: &Unit, round_method: RoundMethod) -> Result<String> {
    let (i2, s) = consider_suffix(s, opts, round_method)?;
    Ok(match s {
        None => format!("{}", i2),
        Some(s) if i2.abs() < 10.0 => format!("{:.1}{}", i2, DisplayableSuffix(s)),
        Some(s) => format!("{:.0}{}", i2, DisplayableSuffix(s)),
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

    let number = transform_to(
        transform_from(source_without_suffix, &options.transform.from)?,
        &options.transform.to,
        options.round,
    )?;

    // bring back the suffix before applying padding
    let number_with_suffix = match &options.suffix {
        Some(suffix) => format!("{}{}", number, suffix),
        None => number,
    };

    Ok(match implicit_padding.unwrap_or(options.padding) {
        0 => number_with_suffix,
        p if p > 0 => format!("{:>padding$}", number_with_suffix, padding = p as usize),
        p => format!(
            "{:<padding$}",
            number_with_suffix,
            padding = p.abs() as usize
        ),
    })
}

fn format_and_print_delimited(s: &str, options: &NumfmtOptions) -> Result<()> {
    let delimiter = options.delimiter.as_ref().unwrap();

    for (n, field) in (1..).zip(s.split(delimiter)) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        // print delimiter before second and subsequent fields
        if n > 1 {
            print!("{}", delimiter);
        }

        if field_selected {
            print!("{}", format_string(field.trim_start(), options, None)?);
        } else {
            // print unselected field without conversion
            print!("{}", field);
        }
    }

    println!();

    Ok(())
}

fn format_and_print_whitespace(s: &str, options: &NumfmtOptions) -> Result<()> {
    for (n, (prefix, field)) in (1..).zip(WhitespaceSplitter { s: Some(s) }) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        if field_selected {
            let empty_prefix = prefix.is_empty();

            // print delimiter before second and subsequent fields
            let prefix = if n > 1 {
                print!(" ");
                &prefix[1..]
            } else {
                prefix
            };

            let implicit_padding = if !empty_prefix && options.padding == 0 {
                Some((prefix.len() + field.len()) as isize)
            } else {
                None
            };

            print!("{}", format_string(field, options, implicit_padding)?);
        } else {
            // print unselected field without conversion
            print!("{}{}", prefix, field);
        }
    }

    println!();

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
