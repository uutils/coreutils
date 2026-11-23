// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore powf seps replacen

use std::fmt::Write as _;
use std::io::Write as _;

use uucore::display::Quotable;
use uucore::i18n::decimal::{locale_decimal_separator, locale_grouping_separator};
use uucore::{show, translate};

use crate::errors::NumfmtError;
use crate::numeric::ParsedNumber;
use crate::options::{InvalidModes, NumfmtOptions, RoundMethod, TransformOptions};
use crate::units::{
    DisplayableSuffix, RawSuffix, Result, Suffix, Unit, iec_bases_f64, si_bases_f64,
};

/// What can go wrong while writing a formatted line: either the line itself is
/// not convertible (which `--invalid` decides what to do with), or the output
/// could not be written at all (which is always fatal).
pub enum WriteError {
    Io(std::io::Error),
    Invalid(String),
}

impl From<std::io::Error> for WriteError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<String> for WriteError {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}

fn find_numeric_beginning(s: &str) -> Option<&str> {
    let dec_sep = locale_decimal_separator();
    let mut seen_dec = false;
    if s.is_empty() {
        return None;
    }

    if s.starts_with(dec_sep) {
        return Some(&s[..dec_sep.len()]);
    }

    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '-' && i == 0 {
            continue;
        }
        if c.is_ascii_digit() {
            continue;
        }
        if !seen_dec && s[i..].starts_with(dec_sep) {
            seen_dec = true;
            // skip past any remaining bytes of a multi-byte sep
            for _ in 1..dec_sep.chars().count() {
                chars.next();
            }
            continue;
        }
        let num_str = s[..i].replace(dec_sep, ".");
        num_str.parse::<f64>().ok()?;
        return Some(&s[..i]);
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

/// Given a string like "5 K Field2" with `unit_separator=" "`, returns the length
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

/// Length of the leading part of `s` that reads as a number, suffix included.
fn valid_prefix_len(s: &str, unit: Unit, unit_separator: &str) -> usize {
    let Some(number_prefix) = find_valid_number_with_suffix(s, unit) else {
        return 0;
    };

    // When a unit separator is in use, the valid part may extend beyond the
    // contiguous number+suffix found by find_valid_number_with_suffix.
    // For example "5 K Field2" with unit_separator=" " has number_prefix="5" but
    // the real valid prefix is "5 K"; the trailing " Field2" is the garbage.
    if !unit_separator.is_empty() && number_prefix == find_numeric_beginning(s).unwrap_or("") {
        valid_end_with_unit_separator(s, number_prefix, unit, unit_separator)
            .unwrap_or(number_prefix.len())
    } else {
        number_prefix.len()
    }
}

/// Byte range of the part of a refused `input` a caret should point at: what
/// follows the number that could be read, or the whole of it when there is
/// none. Surrounding whitespace, reproduced rather than converted, is left out.
pub fn invalid_span(input: &str, options: &NumfmtOptions) -> std::ops::Range<usize> {
    let start = input.len() - input.trim_start().len();
    let mut end = input.trim_end().len();

    // Whitespace only: there is no number to point past, and the two offsets
    // above have crossed — `start` is the whole length, `end` is zero.
    if start >= end {
        return 0..input.len();
    }

    // A declared --suffix is stripped before parsing, so it is not at fault.
    if let Some(suffix) = &options.suffix
        && !suffix.is_empty()
        && let Some(stripped) = input[start..end].strip_suffix(suffix.as_str())
    {
        end = start + stripped.len();
    }

    let valid = valid_prefix_len(
        &input[start..end],
        options.transform.from,
        &options.unit_separator,
    );
    start + valid..end.max(start + valid)
}

/// Whether `input` begins with something that reads as a number, so that advice
/// about what may follow one is only given where there is one. A lone sign or
/// decimal separator is the leading part of a number, but not yet a number.
pub fn holds_number(input: &str) -> bool {
    find_numeric_beginning(input.trim_start()).is_some_and(|number| {
        number
            .replace(locale_decimal_separator(), ".")
            .parse::<f64>()
            .is_ok()
    })
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

    let valid_part = &s[..valid_prefix_len(s, unit, unit_separator)];

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

fn parse_number_part(s: &str, input: &str) -> Result<ParsedNumber> {
    let dec_sep = locale_decimal_separator();
    if s.ends_with(dec_sep) {
        return Err(translate!("numfmt-error-invalid-number", "input" => input.quote()));
    }

    // GNU rejects a leading '+' and scientific notation, which Rust's parsers accept.
    if s.starts_with('+') {
        return Err(translate!("numfmt-error-invalid-number", "input" => input.quote()));
    }
    if s.bytes().any(|b| b == b'e' || b == b'E') {
        return Err(translate!("numfmt-error-invalid-suffix", "input" => input.quote()));
    }

    if let Ok(n) = s.parse::<i128>() {
        return Ok(ParsedNumber::ExactInt(n));
    }

    if dec_sep != "." && s.contains('.') {
        return Err(translate!("numfmt-error-invalid-number", "input" => input.quote()));
    }

    let normalized = if dec_sep == "." {
        std::borrow::Cow::Borrowed(s)
    } else {
        std::borrow::Cow::Owned(s.replace(dec_sep, "."))
    };

    normalized
        .parse::<f64>()
        .map(ParsedNumber::Float)
        .map_err(|_| translate!("numfmt-error-invalid-number", "input" => input.quote()))
}

fn parse_suffix(
    s: &str,
    unit: Unit,
    unit_separator: &str,
    explicit_unit_separator: bool,
) -> Result<(ParsedNumber, Option<Suffix>)> {
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

fn apply_grouping_in_place(buf: &mut String, sep: &str, dec_sep: &str) {
    if sep.is_empty() {
        return;
    }
    let start = usize::from(buf.starts_with('-'));
    let mut pos = buf[start..].find(dec_sep).map_or(buf.len(), |i| start + i);
    while pos >= start + 4 {
        pos -= 3;
        buf.insert_str(pos, sep);
    }
}

/// Returns the first character of `s` and its length in bytes, or `None` if
/// `s` is empty or doesn't start with a valid UTF-8 character.
fn first_char(s: &[u8]) -> Option<(char, usize)> {
    let head = &s[..s.len().min(4)];
    let valid = match std::str::from_utf8(head) {
        Ok(valid) => valid,
        Err(e) => std::str::from_utf8(&head[..e.valid_up_to()]).unwrap_or_default(),
    };
    valid.chars().next().map(|c| (c, c.len_utf8()))
}

/// Returns the length of the blank at the start of `s`, or `None` if `s`
/// doesn't start with one. Bytes that aren't valid UTF-8 are never blanks.
fn blank_len(s: &[u8]) -> Option<usize> {
    first_char(s)
        .filter(|(c, _)| c.is_whitespace())
        .map(|(_, len)| len)
}

/// Splits `s` into the blanks before its first field, the field, and the rest.
///
/// This works on bytes, so a field that isn't valid UTF-8 (such as a file
/// name) doesn't keep the other fields of the line from being converted.
fn split_next_field(s: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let mut prefix_len = 0;
    while let Some(len) = blank_len(&s[prefix_len..]) {
        prefix_len += len;
    }
    let mut field_end = prefix_len;
    while field_end < s.len() && blank_len(&s[field_end..]).is_none() {
        field_end += first_char(&s[field_end..]).map_or(1, |(_, len)| len);
    }
    (&s[..prefix_len], &s[prefix_len..field_end], &s[field_end..])
}

/// When an explicit whitespace unit separator is set (e.g. `--unit-separator=" "`),
/// a suffix like "K" may appear as a separate whitespace-delimited field.  Detect
/// this case so the caller can merge the suffix back into the preceding number field.
fn split_mergeable_suffix<'a>(
    s: &'a [u8],
    options: &NumfmtOptions,
) -> Option<(&'a [u8], &'a [u8])> {
    if !options.explicit_unit_separator
        || options.unit_separator.is_empty()
        || !options.unit_separator.chars().all(char::is_whitespace)
    {
        return None;
    }

    let unit_separator = options.unit_separator.as_bytes();
    if !s.starts_with(unit_separator) {
        return None;
    }

    let (prefix, field, _) = split_next_field(s);
    if prefix != unit_separator {
        return None;
    }

    let (suffix, _) = first_char(field)?;
    RawSuffix::try_from(&suffix).ok()?;
    match field.len() {
        1 => {}
        2 if field.ends_with(b"i") => {}
        _ => return None,
    }

    Some((prefix, field))
}

struct WhitespaceSplitter<'a, 'b> {
    s: Option<&'a [u8]>,
    options: &'b NumfmtOptions,
}

impl<'a> Iterator for WhitespaceSplitter<'a, '_> {
    type Item = (&'a [u8], &'a [u8]);

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
    let dec_sep = locale_decimal_separator();
    match s.split_once(dec_sep) {
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

fn try_scale_exact_int_with_from_unit(
    value: ParsedNumber,
    from_unit: usize,
) -> Option<ParsedNumber> {
    let integer = value.exact_int()?;
    let from_unit = i128::try_from(from_unit).ok()?;
    let scaled = integer.checked_mul(from_unit)?;

    Some(ParsedNumber::ExactInt(scaled))
}

fn transform_from(
    s: &str,
    opts: &TransformOptions,
    options: &NumfmtOptions,
) -> Result<ParsedNumber> {
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

    if had_no_suffix && let Some(scaled) = try_scale_exact_int_with_from_unit(i, opts.from_unit) {
        return Ok(scaled);
    }

    let i = i.to_f64() * (opts.from_unit as f64);

    remove_suffix(i, suffix, opts.from).map(|n| {
        let n = if opts.from == Unit::None || had_no_suffix {
            if n == -0.0 { 0.0 } else { n }
        } else if n < 0.0 {
            -n.abs().ceil()
        } else {
            n.ceil()
        };
        ParsedNumber::Float(n)
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

    // rounding is a no-op once the scale factor overflows f64;
    // dividing by it would turn the value into NaN
    if !p.is_finite() {
        return n;
    }

    method.round(p * n) / p
}

fn consider_suffix(
    n: f64,
    u: Unit,
    round_method: RoundMethod,
    precision: usize,
    is_precision_specified: bool,
) -> Result<(f64, Option<Suffix>)> {
    use crate::units::RawSuffix::{E, G, K, M, P, Q, R, T, Y, Z};

    let abs_n = n.abs();
    let suffixes = [K, M, G, T, P, E, Z, Y, R, Q];

    let (bases, with_i) = match u {
        Unit::Si => (si_bases_f64(), false),
        Unit::Iec(with_i) => (iec_bases_f64(), with_i),
        Unit::Auto => {
            return Err(
                translate!("numfmt-error-invalid-unit-argument", "arg" => "auto", "opt" => "--to"),
            );
        }
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

    // iec caps at 3 decimals to match gnu, si stays as is
    let effective_precision = if matches!(u, Unit::Iec(_)) {
        precision.min(3)
    } else {
        precision
    };
    let v = if is_precision_specified {
        // An explicit `--format` precision, `%.0f` included, rounds the scaled
        // value to exactly that many decimals. `div_round` is for the default
        // presentation instead, which keeps one decimal below the next base.
        round_with_precision(n / bases[i], round_method, effective_precision)
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

fn is_too_large_to_format(scaled: i128, precision: usize) -> bool {
    const MAX_FORMATTED: u128 = 10_000_000_000_000_000_000;
    let precision_factor = 10_u128.pow(precision.min(19) as u32);
    // `.max(1)`: a zero value must still be bounded by precision, else any
    // `--format` precision is accepted (0 * factor == 0), printing/allocating
    // unboundedly. Treating 0 as magnitude 1 matches GNU's threshold.
    scaled
        .unsigned_abs()
        .max(1)
        .checked_mul(precision_factor)
        .is_none_or(|v| v >= MAX_FORMATTED)
}

/// Reusable buffers for formatting one field at a time.
#[derive(Default)]
pub struct FormatScratch {
    buf: String,
    pad_buf: String,
}

fn try_format_exact_int_without_suffix_scaling(
    value: ParsedNumber,
    opts: &TransformOptions,
    precision: usize,
    out: &mut String,
) -> Option<Result<()>> {
    if opts.to != Unit::None {
        return None;
    }

    let integer = value.exact_int()?;
    let to_unit = i128::try_from(opts.to_unit).ok()?;

    if integer % to_unit != 0 {
        return None;
    }

    let scaled = integer / to_unit;

    if is_too_large_to_format(scaled, precision) {
        return Some(Err(too_large_to_print(scaled as f64, precision)));
    }

    let _ = write!(out, "{scaled}");
    if precision > 0 {
        out.push_str(locale_decimal_separator());
        out.extend(std::iter::repeat_n('0', precision));
    }
    Some(Ok(()))
}

/// GNU names the precision only when there is one.
fn too_large_to_print(value: f64, precision: usize) -> String {
    let value_sci = format_gnu_general(value);
    if precision == 0 {
        format!("value too large to be printed: '{value_sci}' (consider using --to)")
    } else {
        format!(
            "value/precision too large to be printed: '{value_sci}/{precision}' (consider using --to)"
        )
    }
}

fn format_gnu_general(v: f64) -> String {
    // like C's %g: 6 significant figures, trailing zeros trimmed, and an
    // exponent of at least two digits only outside 1e-4..1e6
    let s = format!("{v:.5e}");
    let Some(e_pos) = s.find('e') else {
        return s;
    };
    let (mantissa, rest) = s.split_at(e_pos);
    let exp: i32 = rest[1..].parse().unwrap_or(0);
    if (-4..6).contains(&exp) {
        let fixed = format!("{v:.*}", (5 - exp) as usize);
        if !fixed.contains('.') {
            return fixed;
        }
        return fixed
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
    }
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mantissa}e{sign}{:02}", exp.abs())
}

/// Append `value` with `precision` decimals to `out`, as `{:.precision$}` would,
/// replacing `.` with `dec_sep`.
///
/// Most of what numfmt prints is a whole number — `348M`, `1024` — and the
/// general float formatter reaches for a big-integer expansion of the value to
/// get its last digit right. When the value is an integer the answer needs no
/// expansion, so take the short way and let the rest fall through.
fn write_float_localized(out: &mut String, value: f64, precision: usize, dec_sep: &str) {
    // Past 2^53 an f64 no longer holds every integer, and the two routes would
    // not agree on what to print.
    const EXACT_INTEGERS_UP_TO: f64 = 9_007_199_254_740_992.0;

    if precision == 0 && value.is_finite() && value.abs() < EXACT_INTEGERS_UP_TO {
        // Both routes round halves to even; they part only on the sign of a
        // negative value that rounds to zero, which `{:.0}` keeps.
        let rounded = value.round_ties_even();
        if rounded == 0.0 && value.is_sign_negative() {
            out.push_str("-0");
        } else {
            let _ = write!(out, "{}", rounded as i64);
        }
        return;
    }

    let start = out.len();
    let _ = write!(out, "{value:.precision$}");
    if dec_sep != "."
        && let Some(dot) = out[start..].find('.')
    {
        let idx = start + dot;
        out.replace_range(idx..=idx, dec_sep);
    }
}

fn transform_to(
    s: ParsedNumber,
    opts: &TransformOptions,
    round_method: RoundMethod,
    precision: usize,
    unit_separator: &str,
    is_precision_specified: bool,
    out: &mut String,
) -> Result<()> {
    out.clear();
    if let Some(result) = try_format_exact_int_without_suffix_scaling(s, opts, precision, out) {
        return result;
    }

    let s = s.to_f64();
    let i2 = s / (opts.to_unit as f64);
    // a value read with a suffix, like 1Z, is a float and skipped the check above
    if opts.to == Unit::None
        && i2.is_finite()
        && i2.abs().max(1.0) * 10_f64.powi(precision.min(19) as i32) >= 1e19
    {
        return Err(too_large_to_print(i2, precision));
    }
    let (i2, s) = consider_suffix(i2, opts.to, round_method, precision, is_precision_specified)?;
    let (value, digits) = match s {
        None if opts.to == Unit::None && precision <= u16::MAX.into() => {
            let rounded = round_with_precision(i2, round_method, precision);
            // rounding a small negative number leaves -0.0, which would print as "-0"
            (if rounded == 0.0 { 0.0 } else { rounded }, precision)
        }
        None if is_precision_specified && precision <= u16::MAX.into() => {
            (round_with_precision(i2, round_method, 0), precision)
        }
        Some(_) if precision > 0 && precision <= u16::MAX.into() => (i2, precision),
        // single digit before the decimal, like 1.5K
        Some(_) if !is_precision_specified && i2.abs() < 10.0 => (i2, 1),
        _ => (i2, 0),
    };
    write_float_localized(out, value, digits, locale_decimal_separator());
    if let Some(s) = s {
        out.push_str(unit_separator);
        let _ = write!(out, "{}", DisplayableSuffix(s, opts.to));
    }
    Ok(())
}

/// Pad `s` to at least `width` characters using `fill`.
/// Right-aligns when `right_align` is true, left-aligns otherwise.
/// Unlike `format!("{:>width$}")`, this handles widths larger than 65535.
fn pad_string(s: &str, width: usize, fill: char, right_align: bool, out: &mut String) {
    let len = if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    };
    let pad = width.saturating_sub(len);
    out.reserve(s.len() + pad * fill.len_utf8());
    if right_align {
        out.extend(std::iter::repeat_n(fill, pad));
    }
    out.push_str(s);
    if !right_align {
        out.extend(std::iter::repeat_n(fill, pad));
    }
}

fn pad_in_place(buf: &mut String, pad_buf: &mut String, padding: isize) {
    if padding != 0 {
        pad_buf.clear();
        pad_string(buf, padding.unsigned_abs(), ' ', padding > 0, pad_buf);
        std::mem::swap(buf, pad_buf);
    }
}

/// Split a scaled value into its numeric part and the unit suffix that trails
/// it (`k`, `Mi`, ...).
///
/// A value with no digits at all, such as `inf`, has no numeric part to pad, so
/// it is returned whole and is padded as before.
fn split_number_and_units(s: &str) -> (&str, &str) {
    match s.rfind(|c: char| c.is_ascii_digit()) {
        Some(last_digit) => s.split_at(last_digit + 1),
        None => (s, ""),
    }
}

fn format_string(
    source: &str,
    options: &NumfmtOptions,
    implicit_padding: Option<isize>,
    scratch: &mut FormatScratch,
) -> Result<()> {
    let FormatScratch { buf, pad_buf } = scratch;
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

    transform_to(
        transform_from(source_without_suffix, &options.transform, options)?,
        &options.transform,
        options.round,
        precision,
        &options.unit_separator,
        is_precision_specified,
        buf,
    )?;

    // bring back the suffix before applying padding
    if options.grouping {
        apply_grouping_in_place(buf, locale_grouping_separator(), locale_decimal_separator());
    }

    if let Some(suffix) = &options.suffix {
        buf.push_str(suffix);
    }

    let padding = options
        .format
        .padding
        .unwrap_or_else(|| implicit_padding.unwrap_or(options.padding));

    match padding {
        p if p > 0 && options.format.zero_padding => {
            // GNU zero-pads the number itself: "Optional zero (%010f) width
            // will zero pad the number". A unit suffix from --to and the
            // --suffix text sit outside the width, unlike space padding,
            // which applies to the whole output.
            // The --suffix text is user-supplied and may itself contain
            // digits, so peel it off before locating the number.
            let suffix_len = options.suffix.as_deref().unwrap_or("").len();
            let (number, _) = split_number_and_units(&buf[..buf.len() - suffix_len]);
            let (sign, digits) = number.split_at(usize::from(number.starts_with(['-', '+'])));
            pad_buf.clear();
            pad_buf.push_str(sign);
            pad_string(
                digits,
                (p as usize).saturating_sub(sign.len()),
                '0',
                true,
                pad_buf,
            );
            pad_buf.push_str(&buf[number.len()..]);
            std::mem::swap(buf, pad_buf);

            pad_in_place(buf, pad_buf, implicit_padding.unwrap_or(options.padding));
        }
        p => pad_in_place(buf, pad_buf, p),
    }

    if !options.format.prefix.is_empty() {
        buf.insert_str(0, &options.format.prefix);
    }
    buf.push_str(&options.format.suffix);

    Ok(())
}

/// Encodes a byte slice as a string, representing non-UTF-8 bytes and non-printable ASCII
/// bytes as octal escapes. Valid UTF-8 multi-byte characters pass through unchanged.
/// Used to safely format invalid input in error messages.
pub(crate) fn escape_line(line: &[u8]) -> String {
    let mut result = String::new();
    for chunk in line.utf8_chunks() {
        for c in chunk.valid().chars() {
            if c.is_ascii() && !c.is_ascii_graphic() && !c.is_ascii_whitespace() {
                let _ = write!(result, "\\{:03o}", c as u8);
            } else {
                result.push(c);
            }
        }
        for &b in chunk.invalid() {
            let _ = write!(result, "\\{b:03o}");
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

/// Handles a field that could not be converted the way `--invalid` asks:
/// `abort` fails the line, the other modes report the field (or not, for
/// `ignore`) so that the caller can write it unchanged and go on.
fn report_invalid_field(
    options: &NumfmtOptions,
    message: String,
) -> std::result::Result<(), WriteError> {
    match options.invalid {
        InvalidModes::Abort => return Err(WriteError::Invalid(message)),
        InvalidModes::Fail => show!(NumfmtError::FormattingError(message)),
        InvalidModes::Warn => {
            let _ = writeln!(std::io::stderr(), "numfmt: {message}");
        }
        InvalidModes::Ignore => {}
    }
    Ok(())
}

/// Writes `input` with each selected field converted, and returns whether a
/// field could not be converted.
pub fn write_formatted_with_delimiter<W: std::io::Write + ?Sized>(
    writer: &mut W,
    input: &[u8],
    options: &NumfmtOptions,
    eol: Option<u8>,
    scratch: &mut FormatScratch,
) -> std::result::Result<bool, WriteError> {
    let delimiter = options.delimiter.as_deref().unwrap();
    let mut saw_invalid = false;

    for (n, field) in (1..).zip(split_bytes(input, delimiter)) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        // add delimiter before second and subsequent fields
        if n > 1 {
            writer.write_all(delimiter)?;
        }

        if field_selected {
            // Field must be valid UTF-8 for numeric conversion
            let formatted = std::str::from_utf8(field)
                .map_err(|_| translate!("numfmt-error-invalid-number", "input" => escape_line(field).quote()))
                .and_then(|field_str| {
                    format_string(field_str.trim_start(), options, None, scratch)
                });
            match formatted {
                Ok(()) => writer.write_all(scratch.buf.as_bytes())?,
                Err(message) => {
                    report_invalid_field(options, message)?;
                    saw_invalid = true;
                    writer.write_all(field)?;
                }
            }
        } else {
            // add unselected field without conversion
            writer.write_all(field)?;
        }
    }

    if let Some(eol) = eol {
        writer.write_all(&[eol])?;
    }

    Ok(saw_invalid)
}

/// Writes the space that stands for the first of the `blanks` between two
/// fields, and returns the blanks that follow it.
fn write_separator<'a, W: std::io::Write + ?Sized>(
    writer: &mut W,
    blanks: &'a [u8],
) -> std::io::Result<&'a [u8]> {
    writer.write_all(b" ")?;
    Ok(&blanks[blank_len(blanks).unwrap_or(0)..])
}

/// Writes `input` with each selected field converted, and returns whether a
/// field could not be converted.
pub fn write_formatted_with_whitespace<W: std::io::Write + ?Sized>(
    writer: &mut W,
    input: &[u8],
    options: &NumfmtOptions,
    eol: Option<u8>,
    scratch: &mut FormatScratch,
) -> std::result::Result<bool, WriteError> {
    let mut saw_invalid = false;
    for (n, (prefix, field)) in (1..).zip(WhitespaceSplitter {
        s: Some(input),
        options,
    }) {
        let field_selected = uucore::ranges::contain(&options.fields, n);

        if field_selected {
            let empty_prefix = prefix.is_empty();

            // add delimiter before second and subsequent fields
            let prefix = if n > 1 {
                write_separator(writer, prefix)?
            } else {
                prefix
            };

            let implicit_padding = if !empty_prefix && options.padding == 0 {
                Some((prefix.len() + field.len()) as isize)
            } else {
                None
            };

            // Field must be valid UTF-8 for numeric conversion
            let formatted = std::str::from_utf8(field)
                .map_err(|_| translate!("numfmt-error-invalid-number", "input" => escape_line(field).quote()))
                .and_then(|field_str| {
                    format_string(field_str, options, implicit_padding, scratch)
                });
            match formatted {
                Ok(()) => writer.write_all(scratch.buf.as_bytes())?,
                Err(message) => {
                    // An invalid field is written as it is, like an unselected one.
                    report_invalid_field(options, message)?;
                    saw_invalid = true;
                    writer.write_all(prefix)?;
                    writer.write_all(field)?;
                }
            }
        } else {
            // the first blank between fields is written as a space
            let prefix = if n > 1 {
                write_separator(writer, prefix)?
            } else {
                prefix
            };
            // add unselected field without conversion
            writer.write_all(prefix)?;
            writer.write_all(field)?;
        }
    }

    if let Some(eol) = eol {
        writer.write_all(&[eol])?;
    }

    Ok(saw_invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_float_matches_the_general_formatter() {
        // The short route must be indistinguishable from `{:.precision$}`,
        // including the halves, the signed zero and the values that leave the
        // range where an f64 holds every integer.
        for value in [
            0.0,
            -0.0,
            -0.4,
            0.5,
            1.5,
            2.5,
            -1.5,
            -2.5,
            -0.5,
            348.123_456,
            1023.999,
            9_007_199_254_740_992.0,
            9_007_199_254_740_994.0,
            -9_007_199_254_740_994.0,
            1e300,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            let mut out = String::new();
            for precision in [0, 1, 2, 6] {
                for dec_sep in [".", ","] {
                    out.clear();
                    write_float_localized(&mut out, value, precision, dec_sep);
                    assert_eq!(
                        out,
                        format!("{value:.precision$}").replacen('.', dec_sep, 1),
                        "value {value}, precision {precision}, dec_sep {dec_sep}"
                    );
                }
            }
        }
    }

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
        assert_eq!(number.to_f64(), 1.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(!with_i);

        let result = parse_suffix("2R", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number.to_f64(), 2.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert!(!with_i);

        let result = parse_suffix("3k", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number.to_f64(), 3.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::K as i32);
        assert!(!with_i);

        let result = parse_suffix("4Qi", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number.to_f64(), 4.0);
        assert!(suffix.is_some());
        let (raw_suffix, with_i) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert!(with_i);

        let result = parse_suffix("5Ri", Unit::Auto, "", false);
        assert!(result.is_ok());
        let (number, suffix) = result.unwrap();
        assert_eq!(number.to_f64(), 5.0);
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

        let result = consider_suffix(1e27, Unit::Si, RoundMethod::FromZero, 0, false);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::R as i32);
        assert_eq!(value, 1.0);

        let result = consider_suffix(1e30, Unit::Si, RoundMethod::FromZero, 0, false);
        assert!(result.is_ok());
        let (value, suffix) = result.unwrap();
        assert!(suffix.is_some());
        let (raw_suffix, _) = suffix.unwrap();
        assert_eq!(raw_suffix as i32, RawSuffix::Q as i32);
        assert_eq!(value, 1.0);

        let result = consider_suffix(5e30, Unit::Si, RoundMethod::FromZero, 0, false);
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
        assert_eq!(parse_number_part("42", "42").unwrap().to_f64(), 42.0);
        assert_eq!(parse_number_part("-3.5", "-3.5").unwrap().to_f64(), -3.5);
        assert_eq!(parse_number_part("0", "0").unwrap().to_f64(), 0.0);
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
        for s in ["0", "999", "-99"] {
            let mut buf = s.to_string();
            apply_grouping_in_place(
                &mut buf,
                locale_grouping_separator(),
                locale_decimal_separator(),
            );
            assert_eq!(buf, s);
        }
    }

    #[test]
    fn test_apply_grouping_with_fraction() {
        // Fraction part should not be grouped
        let mut result = "1234.567".to_string();
        apply_grouping_in_place(
            &mut result,
            locale_grouping_separator(),
            locale_decimal_separator(),
        );
        // Depending on locale, separator may or may not be present
        assert!(result.contains("567"));
        assert!(result.contains('.'));
    }

    #[test]
    fn test_apply_grouping_negative() {
        let mut result = "-1234".to_string();
        apply_grouping_in_place(
            &mut result,
            locale_grouping_separator(),
            locale_decimal_separator(),
        );
        assert!(result.starts_with('-'));
    }

    #[test]
    fn test_apply_grouping_large_numbers() {
        // These tests verify grouping structure; actual separator depends on locale
        for (input, min_len) in [("1000000", 7), ("1234567890", 10), ("-9999999999999", 13)] {
            let mut result = input.to_string();
            apply_grouping_in_place(
                &mut result,
                locale_grouping_separator(),
                locale_decimal_separator(),
            );
            assert!(result.len() >= min_len);
            if input.starts_with('-') {
                assert!(result.starts_with('-'));
            }
        }

        for (input, sep, dec_sep, expected) in [
            ("999", ",", ".", "999"),
            ("1000", ",", ".", "1,000"),
            ("-123", ",", ".", "-123"),
            ("-1234", ",", ".", "-1,234"),
            ("1234567.891", ",", ".", "1,234,567.891"),
            ("1234567", "\u{202f}", ".", "1\u{202f}234\u{202f}567"),
            ("1234", "", ".", "1234"),
            ("1234567,891", ".", ",", "1.234.567,891"),
            ("-1234,5", "\u{202f}", ",", "-1\u{202f}234,5"),
            ("999,1234", ".", ",", "999,1234"),
        ] {
            let mut buf = input.to_string();
            apply_grouping_in_place(&mut buf, sep, dec_sep);
            assert_eq!(buf, expected);
        }
    }

    #[test]
    fn test_apply_grouping_tiny_fraction() {
        // Small decimal: integer part < 4 digits, so no grouping
        for s in ["0.000001", "1.23456789"] {
            let mut buf = s.to_string();
            apply_grouping_in_place(
                &mut buf,
                locale_grouping_separator(),
                locale_decimal_separator(),
            );
            assert_eq!(buf, s);
        }
    }

    #[test]
    fn test_apply_grouping_exactly_four_digits() {
        let mut result = "1000".to_string();
        apply_grouping_in_place(
            &mut result,
            locale_grouping_separator(),
            locale_decimal_separator(),
        );
        // Should be grouped (4 digits)
        assert!(result.len() >= 4);
    }

    #[test]
    fn test_parse_number_part_large_and_tiny() {
        assert_eq!(
            parse_number_part("999999999999", "999999999999")
                .unwrap()
                .to_f64(),
            999_999_999_999.0
        );
        assert_eq!(
            parse_number_part("0.000000001", "0.000000001")
                .unwrap()
                .to_f64(),
            0.000_000_001
        );
        assert_eq!(
            parse_number_part("-99999999", "-99999999")
                .unwrap()
                .to_f64(),
            -99_999_999.0
        );
    }
}
