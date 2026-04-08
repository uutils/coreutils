// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore powf seps

use uucore::display::Quotable;
use uucore::i18n::decimal::locale_grouping_separator;
use uucore::translate;

use crate::options::{NumfmtOptions, RoundMethod, TransformOptions};
use crate::units::{
    DisplayableSuffix, RawSuffix, Result, Suffix, Unit, iec_bases_f64, si_bases_f64,
};

fn find_numeric_beginning(s: &str) -> Option<&str> {
    let mut decimal_point_seen = false;
    if s.is_empty() {
        return None;
    }

    if s.starts_with('.') {
        return Some(".");
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

/// Given a string like "5 K Field2" with unit_separator=" ", returns the length
/// of the valid prefix including the separator and suffix (e.g. "5 K" → 4).
fn valid_end_with_unit_separator(
    s: &str,
    valid_part: &str,
    unit: Unit,
    unit_separator: &str,
) -> Option<usize> {
    let after_sep = s.get(valid_part.len()..)?.strip_prefix(unit_separator)?;

    let mut chars = after_sep.chars();
    let first_char = chars.next()?;

    RawSuffix::try_from(&first_char).ok()?;

    let is_iec = chars.next() == Some('i') && matches!(unit, Unit::Auto | Unit::Iec(true));
    let suffix_len = 1 + usize::from(is_iec);

    Some(valid_part.len() + unit_separator.len() + suffix_len)
}

fn detailed_error_message(s: &str, unit: Unit, unit_separator: &str) -> Option<String> {
    if s.is_empty() {
        return Some(translate!("numfmt-error-invalid-number-empty"));
    }

    let number_prefix = find_valid_number_with_suffix(s, unit)
        .ok_or(translate!("numfmt-error-invalid-number", "input" => s.quote()))
        .ok()?;

    if number_prefix == "." {
        return Some(translate!("numfmt-error-invalid-suffix", "input" => s.quote()));
    }

    if number_prefix.ends_with('.') {
        return Some(translate!("numfmt-error-invalid-number", "input" => s.quote()));
    }

    // When a unit separator is in use, the valid part may extend beyond the
    // contiguous number+suffix found by find_valid_number_with_suffix.
    // For example "5 K Field2" with unit_separator=" " has number_prefix="5" but
    // the real valid prefix is "5 K"; the trailing " Field2" is the garbage.
    let valid_end =
        if !unit_separator.is_empty() && number_prefix == find_numeric_beginning(s).unwrap_or("") {
            valid_end_with_unit_separator(s, number_prefix, unit, unit_separator)
                .unwrap_or(number_prefix.len())
        } else {
            number_prefix.len()
        };

    let valid_part = &s[..valid_end];

    if valid_part != s && valid_part.parse::<f64>().is_ok() {
        return match s.chars().nth(valid_part.len()) {
            Some('+' | '-') => {
                Some(translate!("numfmt-error-invalid-suffix", "input" => s.quote()))
            }
            Some(v) if RawSuffix::try_from(&v).is_ok() => Some(
                translate!("numfmt-error-rejecting-suffix", "number" => valid_part, "suffix" => s[valid_part.len()..]),
            ),

            _ => Some(translate!("numfmt-error-invalid-suffix", "input" => s.quote())),
        };
    }

    if valid_part != s {
        let trailing = s[valid_part.len()..].trim_start();
        return Some(
            translate!("numfmt-error-invalid-specific-suffix", "input" => s.quote(), "suffix" => trailing.quote()),
        );
    }
    None
}

fn parse_number_part(s: &str, input: &str) -> Result<f64> {
    if s.ends_with('.') {
        return Err(translate!("numfmt-error-invalid-number", "input" => input.quote()));
    }

    s.parse::<f64>()
        .map_err(|_| translate!("numfmt-error-invalid-number", "input" => input.quote()))
}

fn parse_suffix(
    s: &str,
    unit: Unit,
    unit_separator: &str,
    explicit_unit_separator: bool,
) -> Result<(f64, Option<Suffix>)> {
    let trimmed = s.trim_end();
    if trimmed.is_empty() {
        return Err(translate!("numfmt-error-invalid-number-empty"));
    }

    let with_i = trimmed.ends_with('i');
    if with_i && ![Unit::Auto, Unit::Iec(true)].contains(&unit) {
        return Err(translate!("numfmt-error-invalid-suffix", "input" => s.quote()));
    }
    let mut iter = trimmed.chars();
    if with_i {
        iter.next_back();
    }
    let last = iter.next_back();
    let suffix = last
        .and_then(|c| RawSuffix::try_from(&c).ok())
        .map(|raw| (raw, with_i));
    match (suffix, last) {
        (Some(_), _) => {}
        (None, Some(c)) if c.is_ascii_digit() && !with_i => {}
        _ => return Err(translate!("numfmt-error-invalid-number", "input" => s.quote())),
    }

    let suffix_len = suffix.map_or(0, |(_, with_i)| 1 + usize::from(with_i));

    let number_part = &trimmed[..trimmed.len() - suffix_len];

    if suffix.is_some() {
        let separator_len = if explicit_unit_separator {
            if number_part.ends_with(unit_separator) {
                unit_separator.len()
            } else if unit_separator.is_empty() {
                0
            } else {
                return Err(translate!("numfmt-error-invalid-suffix", "input" => s.quote()));
            }
        } else {
            let number_trimmed = number_part.trim_end();
            let whitespace = number_part.len() - number_trimmed.len();
            if whitespace > 1 {
                return Err(translate!("numfmt-error-invalid-suffix", "input" => s.quote()));
            }
            whitespace
        };

        let number = parse_number_part(&number_part[..number_part.len() - separator_len], s)?;

        return Ok((number, suffix));
    }

    let number = parse_number_part(number_part, s)?;

    Ok((number, suffix))
}

fn apply_grouping(s: &str) -> String {
    let grouping_separator = locale_grouping_separator();
    if grouping_separator.is_empty() {
        return s.to_string();
    }

    let (sign, rest) = if let Some(rest) = s.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", s)
    };
    let (integer, fraction) = rest.split_once('.').map_or((rest, ""), |(i, f)| (i, f));
    if integer.len() < 4 {
        return s.to_string();
    }

    let sep_len = grouping_separator.len();
    let num_seps = (integer.len() - 1) / 3;
    let mut grouped = String::with_capacity(
        sign.len()
            + integer.len()
            + num_seps * sep_len
            + if fraction.is_empty() {
                0
            } else {
                1 + fraction.len()
            },
    );
    grouped.push_str(sign);

    let first_group = integer.len() % 3;
    let first_group = if first_group == 0 { 3 } else { first_group };
    grouped.push_str(&integer[..first_group]);
    for chunk in integer.as_bytes()[first_group..].chunks(3) {
        grouped.push_str(grouping_separator);
        // SAFETY: integer is known to be valid UTF-8 ASCII digits
        grouped.push_str(std::str::from_utf8(chunk).unwrap());
    }

    if !fraction.is_empty() {
        grouped.push('.');
        grouped.push_str(fraction);
    }

    grouped
}

fn split_next_field(s: &str) -> (&str, &str, &str) {
    let prefix_len = s.find(|c: char| !c.is_whitespace()).unwrap_or(s.len());
    let field_end = s[prefix_len..]
        .find(char::is_whitespace)
        .map_or(s.len(), |i| prefix_len + i);
    (&s[..prefix_len], &s[prefix_len..field_end], &s[field_end..])
}

/// When an explicit whitespace unit separator is set (e.g. `--unit-separator=" "`),
/// a suffix like "K" may appear as a separate whitespace-delimited field.  Detect
/// this case so the caller can merge the suffix back into the preceding number field.
fn split_mergeable_suffix<'a>(s: &'a str, options: &NumfmtOptions) -> Option<(&'a str, &'a str)> {
    if !options.explicit_unit_separator
        || options.unit_separator.is_empty()
        || !options.unit_separator.chars().all(char::is_whitespace)
    {
        return None;
    }

    if !s.starts_with(&options.unit_separator) {
        return None;
    }

    let (prefix, field, _) = split_next_field(s);
    if prefix != options.unit_separator {
        return None;
    }

    let first_char = field.chars().next()?;
    RawSuffix::try_from(&first_char).ok()?;
    match field.len() {
        1 => {}
        2 if field.ends_with('i') => {}
        _ => return None,
    }

    Some((prefix, field))
}

struct WhitespaceSplitter<'a, 'b> {
    s: Option<&'a str>,
    options: &'b NumfmtOptions,
}

impl<'a> Iterator for WhitespaceSplitter<'a, '_> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let haystack = self.s?;
        let (prefix, field, rest) = split_next_field(haystack);

        if field.is_empty() {
            self.s = None;
            return Some((prefix, field));
        }

        if let Some((suffix_prefix, suffix_field)) = split_mergeable_suffix(rest, self.options) {
            let merged_len = prefix.len() + field.len() + suffix_prefix.len() + suffix_field.len();
            let merged_field = &haystack[prefix.len()..merged_len];
            self.s = Some(&haystack[merged_len..]).filter(|rest| !rest.is_empty());
            return Some((prefix, merged_field));
        }

        self.s = Some(rest).filter(|rest| !rest.is_empty());
        Some((prefix, field))
    }
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

fn remove_suffix(i: f64, s: Option<Suffix>, u: Unit) -> Result<f64> {
    let Some((raw_suffix, with_i)) = s else {
        return Ok(i);
    };
    let idx = raw_suffix.index() + 1;
    match (with_i, u) {
        (false, Unit::Auto | Unit::Si) => Ok(i * si_bases_f64()[idx]),
        (false, Unit::Iec(false)) | (true, Unit::Auto | Unit::Iec(true)) => {
            Ok(i * iec_bases_f64()[idx])
        }
        (false, Unit::Iec(true)) => Err(
            translate!("numfmt-error-missing-i-suffix", "number" => i, "suffix" => format!("{raw_suffix:?}")),
        ),
        (_, Unit::None) => Err(
            translate!("numfmt-error-rejecting-suffix", "number" => i, "suffix" => format!("{raw_suffix:?}{}", if with_i { "i" } else { "" })),
        ),
        _ => Err(translate!("numfmt-error-suffix-unsupported-for-unit")),
    }
}

fn transform_from(s: &str, opts: &TransformOptions, options: &NumfmtOptions) -> Result<f64> {
    let (i, suffix) = parse_suffix(
        s,
        opts.from,
        &options.unit_separator,
        options.explicit_unit_separator,
    )
    .map_err(|original| {
        detailed_error_message(s, opts.from, &options.unit_separator).unwrap_or(original)
    })?;
    let had_no_suffix = suffix.is_none();
    let i = i * (opts.from_unit as f64);

    remove_suffix(i, suffix, opts.from).map(|n| {
        // GNU numfmt doesn't round values if no --from argument is provided by the user
        if opts.from == Unit::None || had_no_suffix {
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
        Unit::Si => (si_bases_f64(), false),
        Unit::Iec(with_i) => (iec_bases_f64(), with_i),
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
    is_precision_specified: bool,
) -> Result<String> {
    let i2 = s / (opts.to_unit as f64);
    let (i2, s) = consider_suffix(i2, opts.to, round_method, precision)?;
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
        Some(s) if is_precision_specified => {
            format!("{i2:.0}{unit_separator}{}", DisplayableSuffix(s, opts.to))
        }
        Some(s) if i2.abs() < 10.0 => {
            // when there's a single digit before the dot.
            format!("{i2:.1}{unit_separator}{}", DisplayableSuffix(s, opts.to))
        }
        Some(s) => {
            format!("{i2:.0}{unit_separator}{}", DisplayableSuffix(s, opts.to))
        }
    })
}

/// Pad `s` to at least `width` characters using `fill`.
/// Right-aligns when `right_align` is true, left-aligns otherwise.
/// Unlike `format!("{:>width$}")`, this handles widths larger than 65535.
fn pad_string(s: &str, width: usize, fill: char, right_align: bool) -> String {
    let len = s.len();
    if len >= width {
        return s.to_string();
    }
    let pad = width - len;
    let mut result = String::with_capacity(width);
    if right_align {
        result.extend(std::iter::repeat_n(fill, pad));
        result.push_str(s);
    } else {
        result.push_str(s);
        result.extend(std::iter::repeat_n(fill, pad));
    }
    result
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
    let mut is_precision_specified = true;
    let precision = if let Some(p) = options.format.precision {
        p
    } else if options.transform.to == Unit::None
        && !source_without_suffix
            .chars()
            .last()
            .is_some_and(char::is_alphabetic)
    {
        parse_implicit_precision(source_without_suffix)
    } else {
        is_precision_specified = false;
        0
    };

    let number = transform_to(
        transform_from(source_without_suffix, &options.transform, options)?,
        &options.transform,
        options.round,
        precision,
        &options.unit_separator,
        is_precision_specified,
    )?;

    // bring back the suffix before applying padding
    let grouped_number = if options.grouping {
        apply_grouping(&number)
    } else {
        number
    };

    let number_with_suffix = match &options.suffix {
        Some(suffix) => format!("{grouped_number}{suffix}"),
        None => grouped_number,
    };

    let padding = options
        .format
        .padding
        .unwrap_or_else(|| implicit_padding.unwrap_or(options.padding));

    let padded_number = match padding {
        0 => number_with_suffix,
        p if p > 0 && options.format.zero_padding => {
            let zero_padded = if let Some(unsigned) = number_with_suffix.strip_prefix(['-', '+']) {
                let sign = &number_with_suffix[..1];
                format!("{sign}{}", pad_string(unsigned, p as usize - 1, '0', true))
            } else {
                pad_string(&number_with_suffix, p as usize, '0', true)
            };

            match implicit_padding.unwrap_or(options.padding) {
                0 => zero_padded,
                p if p > 0 => pad_string(&zero_padded, p as usize, ' ', true),
                p => pad_string(&zero_padded, p.unsigned_abs(), ' ', false),
            }
        }
        p if p > 0 => pad_string(&number_with_suffix, p as usize, ' ', true),
        p => pad_string(&number_with_suffix, p.unsigned_abs(), ' ', false),
    };

    Ok(format!(
        "{}{padded_number}{}",
        options.format.prefix, options.format.suffix
    ))
}
/// Encodes a byte slice as a string, representing non-UTF-8 bytes and non-printable ASCII
/// bytes as octal escapes. Valid UTF-8 multi-byte characters pass through unchanged.
/// Used to safely format invalid input in error messages.
pub(crate) fn escape_line(line: &[u8]) -> String {
    let mut result = String::new();
    for chunk in line.utf8_chunks() {
        for c in chunk.valid().chars() {
            if c.is_ascii() && !c.is_ascii_graphic() && !c.is_ascii_whitespace() {
                result.push_str(&format!("\\{:03o}", c as u8));
            } else {
                result.push(c);
            }
        }
        for &b in chunk.invalid() {
            result.push_str(&format!("\\{b:03o}"));
        }
    }
    result
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

pub fn write_formatted_with_delimiter<W: std::io::Write + ?Sized>(
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
                .map_err(|_| translate!("numfmt-error-invalid-number", "input" => escape_line(field).quote()))?
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

pub fn write_formatted_with_whitespace<W: std::io::Write + ?Sized>(
    writer: &mut W,
    s: &str,
    options: &NumfmtOptions,
    eol: Option<u8>,
) -> Result<()> {
    for (n, (prefix, field)) in (1..).zip(WhitespaceSplitter {
        s: Some(s),
        options,
    }) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        if field_selected {
            let empty_prefix = prefix.is_empty();

            // add delimiter before second and subsequent fields
            let prefix = if n > 1 {
                writer.write_all(b" ").unwrap();
                &prefix[1..]
            } else {
                prefix
            };

            let implicit_padding = if !empty_prefix && options.padding == 0 {
                Some((prefix.len() + field.len()) as isize)
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
        let result = parse_suffix("1Q", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 1.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(!with_i);

        let result = parse_suffix("2R", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 2.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert!(!with_i);

        let result = parse_suffix("3k", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 3.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::K as i32);
        assert!(!with_i);

        let result = parse_suffix("4Qi", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number, 4.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(with_i);

        let result = parse_suffix("5Ri", Unit::Auto, "", false);
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
        let result = parse_suffix("foo", Unit::Auto, "", false);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("numfmt-error-invalid-number") || error.contains("invalid number"));
        assert!(!error.contains("invalid suffix"));

        let result = parse_suffix("World", Unit::Auto, "", false);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("numfmt-error-invalid-number") || error.contains("invalid number"));
        assert!(!error.contains("invalid suffix"));
    }

    #[test]
    fn test_detailed_error_message() {
        let result = detailed_error_message("123i", Unit::Auto, "");
        assert!(result.is_some());
        let error = result.unwrap();
        assert!(error.contains("numfmt-error-invalid-suffix") || error.contains("invalid suffix"));

        let result = detailed_error_message("5MF", Unit::Auto, "");
        assert!(result.is_some());
        let error = result.unwrap();
        assert!(
            error.contains("numfmt-error-invalid-specific-suffix")
                || error.contains("invalid suffix")
        );

        let result = detailed_error_message("5KM", Unit::Auto, "");
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

        let iec = iec_bases_f64();
        let result = remove_suffix(1.0, Some((RawSuffix::Q, true)), Unit::Iec(true));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), iec[10]);

        let result = remove_suffix(1.0, Some((RawSuffix::R, true)), Unit::Iec(true));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), iec[9]);
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
    fn test_detailed_error_message_empty() {
        let result = detailed_error_message("", Unit::Auto, "");
        assert!(result.is_some());
    }

    #[test]
    fn test_detailed_error_message_valid_number() {
        // A plain valid number should return None (no error)
        assert!(detailed_error_message("123", Unit::Auto, "").is_none());
        assert!(detailed_error_message("5K", Unit::Auto, "").is_none());
        assert!(detailed_error_message("-3.5M", Unit::Auto, "").is_none());
    }

    #[test]
    fn test_detailed_error_message_trailing_garbage() {
        // Number with suffix followed by extra chars
        let result = detailed_error_message("5Kx", Unit::Auto, "").unwrap();
        assert!(
            result.contains("numfmt-error-invalid-specific-suffix")
                || result.contains("invalid suffix")
        );
    }

    #[test]
    fn test_detailed_error_message_dot_only() {
        let result = detailed_error_message(".", Unit::Auto, "").unwrap();
        assert!(
            result.contains("numfmt-error-invalid-suffix") || result.contains("invalid suffix")
        );
    }

    #[test]
    fn test_detailed_error_message_trailing_dot() {
        let result = detailed_error_message("5.", Unit::Auto, "").unwrap();
        assert!(
            result.contains("numfmt-error-invalid-number") || result.contains("invalid number")
        );
    }

    #[test]
    fn test_detailed_error_message_unit_separator() {
        // With unit separator, "5 K" is valid
        assert!(detailed_error_message("5 K", Unit::Auto, " ").is_none());

        // "5 Kx" should report trailing garbage after the suffix
        let result = detailed_error_message("5 Kx", Unit::Auto, " ");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_number_part_valid() {
        assert_eq!(parse_number_part("42", "42").unwrap(), 42.0);
        assert_eq!(parse_number_part("-3.5", "-3.5").unwrap(), -3.5);
        assert_eq!(parse_number_part("0", "0").unwrap(), 0.0);
    }

    #[test]
    fn test_parse_number_part_trailing_dot() {
        assert!(parse_number_part("5.", "5.").is_err());
    }

    #[test]
    fn test_parse_number_part_non_numeric() {
        assert!(parse_number_part("abc", "abc").is_err());
        assert!(parse_number_part("", "").is_err());
    }

    #[test]
    fn test_apply_grouping_short_numbers() {
        // Numbers with fewer than 4 digits should be unchanged
        assert_eq!(apply_grouping("0"), "0");
        assert_eq!(apply_grouping("999"), "999");
        assert_eq!(apply_grouping("-99"), "-99");
    }

    #[test]
    fn test_apply_grouping_with_fraction() {
        // Fraction part should not be grouped
        let result = apply_grouping("1234.567");
        // Depending on locale, separator may or may not be present
        assert!(result.contains("567"));
        assert!(result.contains('.'));
    }

    #[test]
    fn test_apply_grouping_negative() {
        let result = apply_grouping("-1234");
        assert!(result.starts_with('-'));
    }

    #[test]
    fn test_apply_grouping_large_numbers() {
        // These tests verify grouping structure; actual separator depends on locale
        let result = apply_grouping("1000000");
        // Should have separators inserted (length grows if separator is non-empty)
        assert!(result.len() >= 7);

        let result = apply_grouping("1234567890");
        assert!(result.len() >= 10);

        let result = apply_grouping("-9999999999999");
        assert!(result.starts_with('-'));
        assert!(result.len() >= 13);
    }

    #[test]
    fn test_apply_grouping_tiny_fraction() {
        // Small decimal: integer part < 4 digits, so no grouping
        assert_eq!(apply_grouping("0.000001"), "0.000001");
        assert_eq!(apply_grouping("1.23456789"), "1.23456789");
    }

    #[test]
    fn test_apply_grouping_exactly_four_digits() {
        let result = apply_grouping("1000");
        // Should be grouped (4 digits)
        assert!(result.len() >= 4);
    }

    #[test]
    fn test_parse_number_part_large_and_tiny() {
        assert_eq!(
            parse_number_part("999999999999", "999999999999").unwrap(),
            999_999_999_999.0
        );
        assert_eq!(
            parse_number_part("0.000000001", "0.000000001").unwrap(),
            0.000_000_001
        );
        assert_eq!(
            parse_number_part("-99999999", "-99999999").unwrap(),
            -99_999_999.0
        );
    }
}
