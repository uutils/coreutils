// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore powf
use uucore::display::Quotable;
use uucore::i18n::decimal::{locale_decimal_separator, locale_grouping_separator};
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
/// let mut fields = uu_numfmt::format::WhitespaceSplitter { s: Some("    1234 5"), skip_whitespace: None };
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
/// let mut fields = uu_numfmt::format::WhitespaceSplitter { s: Some("first second"), skip_whitespace: None };
///
/// assert_eq!(Some(("", "first")), fields.next());
/// assert_eq!(Some((" ", "second")), fields.next());
///
/// let mut fields = uu_numfmt::format::WhitespaceSplitter { s: Some(""), skip_whitespace: None };
///
/// assert_eq!(Some(("", "")), fields.next());
/// ```
pub struct WhitespaceSplitter<'a> {
    pub s: Option<&'a str>,
    pub skip_whitespace: Option<char>,
}

fn is_field_whitespace(c: char) -> bool {
    // Treat NBSP-like characters as part of a field, not as separators.
    // This matches GNU numfmt's handling in locale-sensitive tests.
    if matches!(c, '\u{00A0}' | '\u{2007}' | '\u{202F}' | '\u{2060}') {
        return false;
    }
    c.is_whitespace()
}

impl<'a> Iterator for WhitespaceSplitter<'a> {
    type Item = (&'a str, &'a str);

    /// Yield the next field in the input string as a tuple `(prefix, field)`.
    fn next(&mut self) -> Option<Self::Item> {
        let haystack = self.s?;

        let is_ws = |c: char| {
            if let Some(skip) = self.skip_whitespace {
                if c == skip {
                    return false;
                }
            }
            is_field_whitespace(c)
        };

        let (prefix, field) =
            haystack.split_at(haystack.find(|c: char| !is_ws(c)).unwrap_or(haystack.len()));

        let (field, rest) = field.split_at(field.find(is_ws).unwrap_or(field.len()));

        self.s = if rest.is_empty() { None } else { Some(rest) };

        Some((prefix, field))
    }
}

fn is_blank_for_suffix(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\u{00A0}' | '\u{2007}' | '\u{202F}' | '\u{2060}' | '\u{2003}'
    )
}

fn trim_trailing_blanks(s: &str) -> &str {
    s.trim_end_matches(is_blank_for_suffix)
}

fn locale_separators() -> (char, Option<&'static str>) {
    let decimal_sep = locale_decimal_separator().chars().next().unwrap_or('.');
    let grouping_sep = match locale_grouping_separator() {
        "" => None,
        sep => Some(sep),
    };
    (decimal_sep, grouping_sep)
}

fn decimal_separator_count(s: &str, decimal_sep: char) -> usize {
    s.chars().filter(|&c| c == decimal_sep).count()
}

struct NumberScan {
    end: usize,
    normalized: String,
}

fn scan_number_prefix(
    s: &str,
    decimal_sep: char,
    grouping_sep: Option<char>,
) -> Option<NumberScan> {
    let mut chars = s.char_indices().peekable();
    let mut normalized = String::new();
    let mut digits_before = 0usize;
    let mut digits_after = 0usize;
    let mut seen_decimal = false;
    let mut end = 0usize;
    let mut prev_was_digit = false;

    if let Some((idx, ch)) = chars.peek() {
        if *ch == '-' || *ch == '+' {
            normalized.push(*ch);
            end = idx + ch.len_utf8();
            chars.next();
        }
    }

    while let Some((idx, ch)) = chars.next() {
        if ch.is_ascii_digit() {
            if seen_decimal {
                digits_after += 1;
            } else {
                digits_before += 1;
            }
            normalized.push(ch);
            end = idx + ch.len_utf8();
            prev_was_digit = true;
            continue;
        }

        if ch == decimal_sep {
            if seen_decimal {
                break;
            }
            seen_decimal = true;
            normalized.push('.');
            end = idx + ch.len_utf8();
            prev_was_digit = false;
            continue;
        }

        if grouping_sep.is_some_and(|sep| sep == ch) {
            let next_is_digit = chars.peek().is_some_and(|(_, next)| next.is_ascii_digit());
            if !prev_was_digit || !next_is_digit {
                break;
            }
            end = idx + ch.len_utf8();
            prev_was_digit = false;
            continue;
        }

        break;
    }

    let digits = digits_before + digits_after;
    if digits == 0 {
        return None;
    }
    if seen_decimal && digits_after == 0 {
        return None;
    }

    Some(NumberScan { end, normalized })
}

fn apply_decimal_separator(num: &str, decimal_sep: char) -> String {
    if decimal_sep == '.' {
        return num.to_string();
    }
    if let Some(pos) = num.find('.') {
        let mut out = String::with_capacity(num.len());
        out.push_str(&num[..pos]);
        out.push(decimal_sep);
        out.push_str(&num[pos + 1..]);
        out
    } else {
        num.to_string()
    }
}

fn apply_grouping(num: &str, grouping_sep: &str, decimal_sep: char) -> String {
    let mut parts = num.splitn(2, '.');
    let int_part = parts.next().unwrap_or("");
    let frac_part = parts.next();

    let (sign, digits) = match int_part.chars().next() {
        Some('-') | Some('+') => {
            let sign = int_part.chars().next().unwrap();
            (Some(sign), &int_part[1..])
        }
        _ => (None, int_part),
    };

    let digits_chars: Vec<char> = digits.chars().collect();
    let len = digits_chars.len();
    let mut out = String::new();
    if let Some(sign) = sign {
        out.push(sign);
    }
    for (i, ch) in digits_chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push_str(grouping_sep);
        }
        out.push(*ch);
    }

    if let Some(frac) = frac_part {
        out.push(decimal_sep);
        out.push_str(frac);
    }

    out
}

#[cfg(test)]
fn find_numeric_beginning(s: &str) -> Option<&str> {
    let mut decimal_point_seen = false;
    if s.is_empty() {
        return None;
    }

    for (idx, c) in s.char_indices() {
        if c == '-' && idx == 0 {
            continue;
        }
        if c.is_ascii_digit() {
            continue;
        }
        if c == '.' && !decimal_point_seen {
            decimal_point_seen = true;
            continue;
        }
        if s[..idx].parse::<f64>().is_err() {
            return None;
        }
        return Some(&s[..idx]);
    }

    Some(s)
}

// finds the valid beginning part of an input string, or None.
#[cfg(test)]
fn find_valid_number_with_suffix(s: &str, unit: Unit) -> Option<&str> {
    let numeric_part = find_numeric_beginning(s)?;

    let accepts_suffix = unit != Unit::None;
    let accepts_i = [Unit::Auto, Unit::Iec(true)].contains(&unit);

    let mut characters = s.chars().skip(numeric_part.len());
    let potential_suffix = characters.next();
    let potential_i = characters.next();

    if !accepts_suffix {
        return Some(numeric_part);
    }

    match (potential_suffix, potential_i) {
        (Some(suffix), None) if RawSuffix::try_from(&suffix).is_ok() => {
            Some(&s[..=numeric_part.len()])
        }
        (Some(suffix), Some('i')) if accepts_i && RawSuffix::try_from(&suffix).is_ok() => {
            Some(&s[..numeric_part.len() + 2])
        }
        (Some(suffix), Some(_)) if RawSuffix::try_from(&suffix).is_ok() => {
            Some(&s[..=numeric_part.len()])
        }
        _ => Some(numeric_part),
    }
}

#[cfg(test)]
fn detailed_error_message(s: &str, unit: Unit) -> Option<String> {
    parse_suffix(s, unit, 1).err()
}

fn parse_suffix(s: &str, unit: Unit, max_whitespace: usize) -> Result<(f64, Option<Suffix>)> {
    let trimmed = trim_trailing_blanks(s);
    if trimmed.is_empty() {
        return Err(translate!("numfmt-error-invalid-number-empty"));
    }

    let (decimal_sep, grouping_sep) = locale_separators();
    let grouping_sep = grouping_sep
        .and_then(|sep| sep.chars().next())
        .filter(|&sep| sep != decimal_sep);

    let Some(scan) = scan_number_prefix(trimmed, decimal_sep, grouping_sep) else {
        if decimal_separator_count(trimmed, decimal_sep) >= 2 {
            return Err(translate!(
                "numfmt-error-invalid-suffix",
                "input" => trimmed.quote()
            ));
        }
        return Err(translate!(
            "numfmt-error-invalid-number",
            "input" => trimmed.quote()
        ));
    };

    let number = scan
        .normalized
        .parse::<f64>()
        .map_err(|_| translate!("numfmt-error-invalid-number", "input" => trimmed.quote()))?;
    let number_str = &trimmed[..scan.end];

    let mut rest = &trimmed[scan.end..];

    if rest.is_empty() {
        return Ok((number, None));
    }

    let whitespace_len = rest.len() - rest.trim_start_matches(is_blank_for_suffix).len();
    if whitespace_len > max_whitespace {
        return Err(translate!(
            "numfmt-error-invalid-suffix",
            "input" => trimmed.quote()
        ));
    }
    rest = &rest[whitespace_len..];

    if rest.is_empty() {
        return Ok((number, None));
    }

    if matches!(unit, Unit::None) {
        let mut chars = rest.chars();
        let suffix_char = chars.next().unwrap();
        if RawSuffix::try_from(&suffix_char).is_ok() {
            return Err(translate!(
                "numfmt-error-rejecting-suffix",
                "number" => number_str,
                "suffix" => rest
            ));
        }
        return Err(translate!(
            "numfmt-error-invalid-suffix",
            "input" => trimmed.quote()
        ));
    }

    let mut chars = rest.chars();
    let suffix_char = chars.next().unwrap();
    let Ok(raw_suffix) = RawSuffix::try_from(&suffix_char) else {
        return Err(translate!(
            "numfmt-error-invalid-suffix",
            "input" => trimmed.quote()
        ));
    };

    let mut with_i = false;
    let mut remainder = chars.as_str();
    if remainder.starts_with('i') {
        if [Unit::Auto, Unit::Iec(true)].contains(&unit) {
            with_i = true;
            remainder = &remainder[1..];
        } else {
            let suffix_detail = remainder.trim_start_matches(is_blank_for_suffix);
            return Err(translate!(
                "numfmt-error-invalid-specific-suffix",
                "input" => trimmed.quote(),
                "suffix" => suffix_detail.quote()
            ));
        }
    }

    if matches!(unit, Unit::Iec(true)) && !with_i {
        return Err(translate!(
            "numfmt-error-missing-i-suffix",
            "number" => number,
            "suffix" => format!("{raw_suffix:?}")
        ));
    }

    if !remainder.is_empty() {
        let suffix_detail = remainder.trim_start_matches(is_blank_for_suffix);
        if suffix_detail.is_empty() {
            return Err(translate!(
                "numfmt-error-invalid-suffix",
                "input" => trimmed.quote()
            ));
        }
        return Err(translate!(
            "numfmt-error-invalid-specific-suffix",
            "input" => trimmed.quote(),
            "suffix" => suffix_detail.quote()
        ));
    }

    Ok((number, Some((raw_suffix, with_i))))
}

/// Returns the implicit precision of a number, which is the count of digits after the dot. For
/// example, 1.23 has an implicit precision of 2.
fn parse_implicit_precision(s: &str, decimal_sep: char) -> usize {
    match s.split_once(decimal_sep) {
        Some((_, decimal_part)) => decimal_part
            .chars()
            .take_while(char::is_ascii_digit)
            .count(),
        None => 0,
    }
}

fn remove_suffix(i: f64, s: Option<Suffix>, u: Unit) -> Result<f64> {
    match (s, u) {
        (Some((raw_suffix, false)), Unit::Auto | Unit::Si) => match raw_suffix {
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
        (Some((raw_suffix, false)), Unit::Iec(false))
        | (Some((raw_suffix, true)), Unit::Auto | Unit::Iec(true)) => match raw_suffix {
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
        (Some((raw_suffix, false)), Unit::Iec(true)) => Err(
            translate!("numfmt-error-missing-i-suffix", "number" => i, "suffix" => format!("{raw_suffix:?}")),
        ),
        (Some((raw_suffix, with_i)), Unit::None) => Err(
            translate!("numfmt-error-rejecting-suffix", "number" => i, "suffix" => format!("{raw_suffix:?}{}", if with_i { "i" } else { "" })),
        ),
        (None, _) => Ok(i),
        (_, _) => Err(translate!("numfmt-error-suffix-unsupported-for-unit")),
    }
}

fn transform_from(s: &str, opts: &TransformOptions, max_whitespace: usize) -> Result<f64> {
    let (i, suffix) = parse_suffix(s, opts.from, max_whitespace)?;
    let i = i * (opts.from_unit as f64);

    remove_suffix(i, suffix, opts.from).map(|n| {
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
    u: Unit,
    round_method: RoundMethod,
    precision: usize,
) -> Result<(f64, Option<Suffix>)> {
    use crate::units::RawSuffix::{E, G, K, M, P, Q, R, T, Y, Z};

    let abs_n = n.abs();
    let suffixes = [K, M, G, T, P, E, Z, Y, R, Q];

    let (bases, with_i) = match u {
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
    unit_separator: &str,
) -> Result<String> {
    let (i2, s) = consider_suffix(s, opts.to, round_method, precision)?;
    let i2 = i2 / (opts.to_unit as f64);
    Ok(match s {
        None => {
            format!(
                "{:.precision$}",
                round_with_precision(i2, round_method, precision),
            )
        }
        Some(s) if precision > 0 => {
            format!(
                "{i2:.precision$}{unit_separator}{}",
                DisplayableSuffix(s, opts.to),
            )
        }
        Some(s) if i2.abs() < 10.0 => {
            format!("{i2:.1}{unit_separator}{}", DisplayableSuffix(s, opts.to))
        }
        Some(s) => format!("{i2:.0}{unit_separator}{}", DisplayableSuffix(s, opts.to)),
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
    let (decimal_sep, grouping_sep) = locale_separators();

    let precision = if let Some(p) = options.format.precision {
        p
    } else if options.transform.from == Unit::None && options.transform.to == Unit::None {
        parse_implicit_precision(source_without_suffix, decimal_sep)
    } else {
        0
    };

    let mut number = transform_to(
        transform_from(
            source_without_suffix,
            &options.transform,
            options.max_whitespace,
        )?,
        &options.transform,
        options.round,
        precision,
        &options.unit_separator,
    )?;

    let grouping_requested = options.format.grouping && options.transform.to == Unit::None;
    number = if grouping_requested {
        grouping_sep.map_or_else(
            || apply_decimal_separator(&number, decimal_sep),
            |sep| apply_grouping(&number, sep, decimal_sep),
        )
    } else {
        apply_decimal_separator(&number, decimal_sep)
    };

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

fn split_bytes<'a>(input: &'a [u8], delim: &'a [u8]) -> impl Iterator<Item = &'a [u8]> {
    let mut remainder = Some(input);
    std::iter::from_fn(move || {
        let input = remainder.take()?;
        if delim.is_empty() {
            return Some(input);
        }
        match input.windows(delim.len()).position(|w| w == delim) {
            Some(pos) => {
                remainder = Some(&input[pos + delim.len()..]);
                Some(&input[..pos])
            }
            None => Some(input),
        }
    })
}

pub fn write_formatted_with_delimiter<W: std::io::Write>(
    writer: &mut W,
    input: &[u8],
    options: &NumfmtOptions,
    eol: Option<u8>,
) -> Result<()> {
    let delimiter = options.delimiter.as_deref().unwrap();

    for (n, field) in (1..).zip(split_bytes(input, delimiter)) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        // add delimiter before second and subsequent fields
        if n > 1 {
            writer.write_all(delimiter).unwrap();
        }

        if field_selected {
            // Field must be valid UTF-8 for numeric conversion
            let field_str = std::str::from_utf8(field)
                .map_err(|_| translate!("numfmt-error-invalid-number", "input" => String::from_utf8_lossy(field).into_owned().quote()))?
                .trim_start();
            let formatted = format_string(field_str, options, None)?;
            writer.write_all(formatted.as_bytes()).unwrap();
        } else {
            // add unselected field without conversion
            writer.write_all(field).unwrap();
        }
    }

    if let Some(eol) = eol {
        writer.write_all(&[eol]).unwrap();
    }

    Ok(())
}

pub fn write_formatted_with_whitespace<W: std::io::Write>(
    writer: &mut W,
    s: &str,
    options: &NumfmtOptions,
    eol: Option<u8>,
) -> Result<()> {
    for (n, (prefix, field)) in (1..).zip(WhitespaceSplitter {
        s: Some(s),
        skip_whitespace: None,
    }) {
        let field_selected = uucore::ranges::contain(&options.fields, n);
        let prefix_len = prefix.chars().count();
        let field_len = field.chars().count();

        if field_selected {
            let empty_prefix = prefix_len == 0;

            let prefix_for_padding_len = if n > 1 {
                writer.write_all(b" ").unwrap();
                prefix_len.saturating_sub(1)
            } else {
                prefix_len
            };

            let implicit_padding = if !empty_prefix && options.padding == 0 {
                Some((prefix_for_padding_len + field_len) as isize)
            } else {
                None
            };

            let formatted = format_string(field, options, implicit_padding)?;
            writer.write_all(formatted.as_bytes()).unwrap();
        } else {
            // the -z option converts an initial \n into a space
            let prefix = if options.zero_terminated && prefix.starts_with('\n') {
                writer.write_all(b" ").unwrap();
                &prefix[1..]
            } else {
                prefix
            };
            // add unselected field without conversion
            writer.write_all(prefix.as_bytes()).unwrap();
            writer.write_all(field.as_bytes()).unwrap();
        }
    }

    if let Some(eol) = eol {
        writer.write_all(&[eol]).unwrap();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cognitive_complexity)]
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
        assert_eq!(0, parse_implicit_precision("", '.'));
        assert_eq!(0, parse_implicit_precision("1", '.'));
        assert_eq!(1, parse_implicit_precision("1.2", '.'));
        assert_eq!(2, parse_implicit_precision("1.23", '.'));
        assert_eq!(3, parse_implicit_precision("1.234", '.'));
        assert_eq!(0, parse_implicit_precision("1K", '.'));
        assert_eq!(1, parse_implicit_precision("1.2K", '.'));
        assert_eq!(2, parse_implicit_precision("1.23K", '.'));
        assert_eq!(3, parse_implicit_precision("1.234K", '.'));
        assert_eq!(1, parse_implicit_precision("1,2", ','));
        assert_eq!(2, parse_implicit_precision("1,23", ','));
        assert_eq!(0, parse_implicit_precision("1.23", ','));
    }

    #[test]
    fn test_scan_number_prefix_grouping_must_be_between_digits() {
        let decimal_sep = '.';
        let grouping_sep = Some(',');

        let valid = scan_number_prefix("1,2,3", decimal_sep, grouping_sep).unwrap();
        assert_eq!(&"1,2,3"[..valid.end], "1,2,3");

        let valid_fraction = scan_number_prefix("1.2,3", decimal_sep, grouping_sep).unwrap();
        assert_eq!(&"1.2,3"[..valid_fraction.end], "1.2,3");

        let leading = scan_number_prefix(",1", decimal_sep, grouping_sep);
        assert!(leading.is_none());

        let repeated = scan_number_prefix("1,,2", decimal_sep, grouping_sep).unwrap();
        assert_eq!(&"1,,2"[..repeated.end], "1");

        let before_decimal = scan_number_prefix("1,.2", decimal_sep, grouping_sep).unwrap();
        assert_eq!(&"1,.2"[..before_decimal.end], "1");
    }

    #[test]
    fn test_parse_suffix_q_r_k() {
        let result = parse_suffix("1Q", Unit::Auto, 1);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 1.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(!with_i);

        let result = parse_suffix("2R", Unit::Auto, 1);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 2.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert!(!with_i);

        let result = parse_suffix("3k", Unit::Auto, 1);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 3.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::K as i32);
        assert!(!with_i);

        let result = parse_suffix("4Qi", Unit::Auto, 1);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 4.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(with_i);

        let result = parse_suffix("5Ri", Unit::Auto, 1);
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
        let result = parse_suffix("foo", Unit::Auto, 1);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("numfmt-error-invalid-number") || error.contains("invalid number"));
        assert!(!error.contains("invalid suffix"));

        let result = parse_suffix("World", Unit::Auto, 1);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("numfmt-error-invalid-number") || error.contains("invalid number"));
        assert!(!error.contains("invalid suffix"));
    }

    #[test]
    fn test_detailed_error_message() {
        let result = detailed_error_message("123i", Unit::Auto);
        assert!(result.is_some());
        let error = result.unwrap();
        assert!(error.contains("numfmt-error-invalid-suffix") || error.contains("invalid suffix"));

        let result = detailed_error_message("5MF", Unit::Auto);
        assert!(result.is_some());
        let error = result.unwrap();
        assert!(
            error.contains("numfmt-error-invalid-specific-suffix")
                || error.contains("invalid suffix")
        );

        let result = detailed_error_message("5KM", Unit::Auto);
        assert!(result.is_some());
        let error = result.unwrap();
        assert!(
            error.contains("numfmt-error-invalid-specific-suffix")
                || error.contains("invalid suffix")
        );
    }

    #[test]
    fn test_remove_suffix_q_r() {
        use crate::units::Unit;

        let result = remove_suffix(1.0, Some((RawSuffix::Q, false)), Unit::Si);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1e30);

        let result = remove_suffix(1.0, Some((RawSuffix::R, false)), Unit::Si);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1e27);

        let result = remove_suffix(1.0, Some((RawSuffix::Q, true)), Unit::Iec(true));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), IEC_BASES[10]);

        let result = remove_suffix(1.0, Some((RawSuffix::R, true)), Unit::Iec(true));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), IEC_BASES[9]);
    }

    #[test]
    fn test_find_valid_part() {
        assert_eq!(
            find_valid_number_with_suffix("12345KL", Unit::Auto),
            Some("12345K")
        );
        assert_eq!(
            find_valid_number_with_suffix("12345K", Unit::Auto),
            Some("12345K")
        );
        assert_eq!(
            find_valid_number_with_suffix("12345", Unit::Auto),
            Some("12345")
        );
        assert_eq!(
            find_valid_number_with_suffix("asd12345KL", Unit::Auto),
            None
        );
        assert_eq!(
            find_valid_number_with_suffix("8asdf", Unit::Auto),
            Some("8")
        );
        assert_eq!(find_valid_number_with_suffix("5i", Unit::Si), Some("5"));
        assert_eq!(
            find_valid_number_with_suffix("5i", Unit::Iec(true)),
            Some("5")
        );
        assert_eq!(
            find_valid_number_with_suffix("0.1KL", Unit::Auto),
            Some("0.1K")
        );
        assert_eq!(
            find_valid_number_with_suffix("0.1", Unit::Auto),
            Some("0.1")
        );
        assert_eq!(
            find_valid_number_with_suffix("-0.1MT", Unit::Auto),
            Some("-0.1M")
        );
        assert_eq!(
            find_valid_number_with_suffix("-0.1PT", Unit::Auto),
            Some("-0.1P")
        );
        assert_eq!(
            find_valid_number_with_suffix("-0.1PT", Unit::Auto),
            Some("-0.1P")
        );
        assert_eq!(
            find_valid_number_with_suffix("123.4.5", Unit::Auto),
            Some("123.4")
        );
        assert_eq!(
            find_valid_number_with_suffix("0.55KiJ", Unit::Iec(true)),
            Some("0.55Ki")
        );
        assert_eq!(
            find_valid_number_with_suffix("0.55KiJ", Unit::Iec(false)),
            Some("0.55K")
        );
        assert_eq!(
            find_valid_number_with_suffix("123KICK", Unit::Auto),
            Some("123K")
        );
        assert_eq!(find_valid_number_with_suffix("", Unit::Auto), None);
    }

    #[test]
    fn test_consider_suffix_q_r() {
        use crate::options::RoundMethod;
        use crate::units::Unit;

        let result = consider_suffix(1e27, Unit::Si, RoundMethod::FromZero, 0);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert_eq!(value, 1.0);

        let result = consider_suffix(1e30, Unit::Si, RoundMethod::FromZero, 0);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert_eq!(value, 1.0);

        let result = consider_suffix(5e30, Unit::Si, RoundMethod::FromZero, 0);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert_eq!(value, 5.0);
    }

    #[test]
    fn test_whitespace_splitter_nbsp_not_separator() {
        let s = "1\u{00A0}K 2".to_string();
        let mut fields = WhitespaceSplitter {
            s: Some(&s),
            skip_whitespace: None,
        };

        assert_eq!(Some(("", "1\u{00A0}K")), fields.next());
        assert_eq!(Some((" ", "2")), fields.next());
        assert_eq!(None, fields.next());
    }

    #[test]
    fn test_whitespace_splitter_em_space_is_separator() {
        let s = "1\u{2003}2".to_string();
        let mut fields = WhitespaceSplitter {
            s: Some(&s),
            skip_whitespace: None,
        };

        assert_eq!(Some(("", "1")), fields.next());
        assert_eq!(Some(("\u{2003}", "2")), fields.next());
        assert_eq!(None, fields.next());
    }
}
