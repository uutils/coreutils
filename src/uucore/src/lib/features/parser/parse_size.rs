// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) hdsf ghead gtail ACDBK hexdigit

//! Parser for sizes in SI or IEC units (multiples of 1000 or 1024 bytes).

/// Where the size itself starts inside `src`, past the leading whitespace and
/// one optional prefix character.
///
/// A [`ParseSizeError`] reports offsets into the size the parser saw, never
/// into the argument as typed. The parsers taking a SIZE trim it and then take
/// at most one character of their own before the size proper — a sign for
/// `head` and `tail`, a mode character for `truncate` — so a caret over the
/// argument has to count both back in. Only *one* such character is taken, so
/// a second one belongs to the size as far as the parser is concerned and is
/// left in.
///
/// # Arguments
///
/// * `src` - The argument as typed.
/// * `is_prefix` - Whether a character is the prefix the caller's parser
///   strips.
pub fn size_offset(src: &str, is_prefix: impl Fn(char) -> bool) -> usize {
    let trimmed = src.trim_start();
    (src.len() - trimmed.len())
        + trimmed
            .chars()
            .next()
            .filter(|&c| is_prefix(c))
            .map_or(0, char::len_utf8)
}

/// The first eleven powers of 1000: `1, 10^3, 10^6, ..., 10^30`.
///
/// Index `n` is the SI base for the `n`-th suffix (0 → no suffix, 1 → K/kB,
/// 2 → M/MB, ...).
pub const SI_BASES: [u128; 11] = [
    1,
    1_000,
    1_000_000,
    1_000_000_000,
    1_000_000_000_000,
    1_000_000_000_000_000,
    1_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000_000_000,
];

/// The first eleven powers of 1024: `1, 1024, 1024^2, ..., 1024^10`.
///
/// Index `n` is the IEC base for the `n`-th suffix (0 → no suffix, 1 → Ki,
/// 2 → Mi, ...).
pub const IEC_BASES: [u128; 11] = [
    1,
    1_024,
    1_048_576,
    1_073_741_824,
    1_099_511_627_776,
    1_125_899_906_842_624,
    1_152_921_504_606_846_976,
    1_180_591_620_717_411_303_424,
    1_208_925_819_614_629_174_706_176,
    1_237_940_039_285_380_274_899_124_224,
    1_267_650_600_228_229_401_496_703_205_376,
];

use std::error::Error;
use std::fmt;
use std::num::{IntErrorKind, ParseIntError};
use std::ops::Range;

use crate::display::Quotable;
#[cfg(target_os = "linux")]
use procfs::{Current, Meminfo};

/// Error arising from trying to compute system memory.
enum SystemError {
    IOError,
    ParseError,
    #[cfg(not(target_os = "linux"))]
    NotFound,
}

impl From<std::io::Error> for SystemError {
    fn from(_: std::io::Error) -> Self {
        Self::IOError
    }
}

impl From<ParseIntError> for SystemError {
    fn from(_: ParseIntError) -> Self {
        Self::ParseError
    }
}

/// Get the total number of bytes of physical memory.
///
/// The information is read from the `/proc/meminfo` file.
///
/// # Errors
///
/// If there is a problem reading the file or finding the appropriate
/// entry in the file.
#[cfg(target_os = "linux")]
fn total_physical_memory() -> Result<u128, SystemError> {
    let info = Meminfo::current().map_err(|_| SystemError::IOError)?;
    Ok((info.mem_total as u128).saturating_mul(1024))
}

/// Return the number of bytes of memory that appear to be currently available.
#[cfg(target_os = "linux")]
pub fn available_memory_bytes() -> Option<u128> {
    let info = Meminfo::current().ok()?;

    if let Some(available_kib) = info.mem_available {
        let available_bytes = (available_kib as u128).saturating_mul(1024);
        if available_bytes > 0 {
            return Some(available_bytes);
        }
    }

    let fallback_kib = (info.mem_free as u128)
        .saturating_add(info.buffers as u128)
        .saturating_add(info.cached as u128);

    if fallback_kib > 0 {
        Some(fallback_kib.saturating_mul(1024))
    } else {
        total_physical_memory().ok()
    }
}

/// Return `None` when the platform does not expose Linux-like `/proc/meminfo`.
#[cfg(not(target_os = "linux"))]
pub fn available_memory_bytes() -> Option<u128> {
    None
}

/// Get the total number of bytes of physical memory.
///
/// TODO Implement this for non-Linux systems.
#[cfg(not(target_os = "linux"))]
fn total_physical_memory() -> Result<u128, SystemError> {
    Err(SystemError::NotFound)
}

/// Parser for sizes in SI or IEC units (multiples of 1000 or 1024 bytes).
///
/// The [`Parser::parse`] function performs the parse.
#[derive(Default)]
pub struct Parser<'parser> {
    /// Whether to allow empty numeric strings.
    pub no_empty_numeric: bool,
    /// Whether to treat the suffix "B" as meaning "bytes".
    pub capital_b_bytes: bool,
    /// Whether to treat "b" as a "byte count" instead of "block"
    pub b_byte_count: bool,
    /// Whitelist for the suffix
    pub allow_list: Option<&'parser [&'parser str]>,
    /// Default unit when no suffix is provided
    pub default_unit: Option<&'parser str>,
}

enum NumberSystem {
    Decimal,
    Octal,
    Hexadecimal,
    Binary,
}

/// The leading part of `size` that spells out its number.
///
/// The rest is the unit, so this is also where a caret goes when the unit is
/// the part at fault.
///
/// # Arguments
///
/// * `size` - The SIZE operand as typed.
/// * `number_system` - How its digits are to be read.
fn numeric_prefix(size: &str, number_system: NumberSystem) -> &str {
    let len = match number_system {
        NumberSystem::Hexadecimal | NumberSystem::Binary => {
            let is_digit = |c: char| match number_system {
                NumberSystem::Hexadecimal => c.is_ascii_hexdigit(),
                _ => c.is_digit(2),
            };
            // The `0x` or `0b` that named the system, and the digits after it.
            2 + size
                .chars()
                .skip(2)
                .take_while(|&c| is_digit(c))
                .map(char::len_utf8)
                .sum::<usize>()
        }
        _ => size
            .chars()
            .take_while(char::is_ascii_digit)
            .map(char::len_utf8)
            .sum(),
    };
    &size[..len.min(size.len())]
}

impl<'parser> Parser<'parser> {
    /// Change allow_list of the parser - whitelist for the suffix
    pub fn with_allow_list(&mut self, allow_list: &'parser [&str]) -> &mut Self {
        self.allow_list = Some(allow_list);
        self
    }

    /// Change default_unit of the parser - when no suffix is provided
    pub fn with_default_unit(&mut self, default_unit: &'parser str) -> &mut Self {
        self.default_unit = Some(default_unit);
        self
    }

    /// Change b_byte_count of the parser - to treat "b" as a "byte count" instead of "block"
    pub fn with_b_byte_count(&mut self, value: bool) -> &mut Self {
        self.b_byte_count = value;
        self
    }

    /// Change no_empty_numeric of the parser - to allow empty numeric strings
    pub fn with_allow_empty_numeric(&mut self, value: bool) -> &mut Self {
        self.no_empty_numeric = value;
        self
    }
    /// Parse a size string into a number of bytes.
    ///
    /// A size string comprises an integer and an optional unit. The integer
    /// may be in decimal, octal (0 prefix), hexadecimal (0x prefix), or
    /// binary (0b prefix) notation. The unit may be K, M, G, T, P, E, Z, Y,
    /// R or Q (powers of 1024), or KB, MB, etc. (powers of 1000), or b which
    /// is 512. Binary prefixes can be used, too: KiB=K, MiB=M, and so on.
    ///
    /// # Errors
    ///
    /// Will return `ParseSizeError` if it's not possible to parse this
    /// string into a number, e.g. if the string does not begin with a
    /// numeral, or if the unit is not one of the supported units described
    /// in the preceding section.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uucore::parser::parse_size::Parser;
    /// let parser = Parser {
    ///     default_unit: Some("M"),
    ///     ..Default::default()
    /// };
    /// assert_eq!(Ok(123 * 1024 * 1024), parser.parse("123M")); // M is 1024^2
    /// assert_eq!(Ok(123 * 1024 * 1024), parser.parse("123")); // default unit set to "M" on parser instance
    /// assert_eq!(Ok(9 * 1000), parser.parse("9kB")); // kB is 1000
    /// assert_eq!(Ok(2 * 1024), parser.parse("2K")); // K is 1024
    /// assert_eq!(Ok(44251 * 1024), parser.parse("0xACDBK")); // 0xACDB is 44251 in decimal
    /// assert_eq!(Ok(44251 * 1024 * 1024), parser.parse("0b1010110011011011")); // 0b1010110011011011 is 44251 in decimal, default M
    /// ```
    pub fn parse(&self, size: &str) -> Result<u128, ParseSizeError> {
        if size.is_empty() {
            return Err(ParseSizeError::parse_failure(size));
        }

        let number_system = Self::determine_number_system(size);

        // Split the size argument into numeric and unit parts
        // For example, if the argument is "123K", the numeric part is "123", and
        // the unit is "K"
        let numeric_string: String = match number_system {
            NumberSystem::Hexadecimal => size
                .chars()
                .take(2)
                .chain(size.chars().skip(2).take_while(char::is_ascii_hexdigit))
                .collect(),
            NumberSystem::Binary => size
                .chars()
                .take(2)
                .chain(size.chars().skip(2).take_while(|c| c.is_digit(2)))
                .collect(),
            _ => size.chars().take_while(char::is_ascii_digit).collect(),
        };
        let mut unit: &str = &size[numeric_string.len()..];

        if let Some(default_unit) = self.default_unit {
            // Check if `unit` is empty then assigns `default_unit` to `unit`
            if unit.is_empty() {
                unit = default_unit;
            }
        }

        // Check if `b` is a byte count and remove `b`
        if self.b_byte_count && unit.ends_with('b') {
            // If `unit` = 'b' then return error
            if numeric_string.is_empty() {
                return Err(ParseSizeError::parse_failure(size));
            }
            unit = &unit[0..unit.len() - 1];
        }

        if let Some(allow_list) = self.allow_list {
            // Check if `unit` appears in `allow_list`, if not return error
            if !allow_list.contains(&unit) && !unit.is_empty() {
                if numeric_string.is_empty() {
                    return Err(ParseSizeError::parse_failure(size));
                }
                return Err(ParseSizeError::invalid_suffix(size));
            }
        }

        // Special case: for percentage, just compute the given fraction
        // of the total physical memory on the machine, if possible.
        if unit == "%" {
            let number: u128 = Self::parse_number(&numeric_string, 10, size)?;
            return match total_physical_memory() {
                Ok(total) => (number / 100)
                    .checked_mul(total)
                    .ok_or_else(|| ParseSizeError::size_too_big(size)),
                Err(_) => Err(ParseSizeError::PhysicalMem(size.to_string())),
            };
        }

        // Compute the factor the unit represents.
        // empty string means the factor is 1.
        //
        // The lowercase "b" (used by `od`, `head`, `tail`, etc.) means
        // "block" and the Posix block size is 512. The uppercase "B"
        // means "byte".
        let (base, exponent): (u128, u32) = match unit {
            "" => (1, 0),
            "B" if self.capital_b_bytes => (1, 0),
            "b" => (512, 1),
            "KiB" | "kiB" | "K" | "k" => (1024, 1),
            "MiB" | "miB" | "M" | "m" => (1024, 2),
            "GiB" | "giB" | "G" | "g" => (1024, 3),
            "TiB" | "tiB" | "T" | "t" => (1024, 4),
            "PiB" | "piB" | "P" | "p" => (1024, 5),
            "EiB" | "eiB" | "E" | "e" => (1024, 6),
            "ZiB" | "ziB" | "Z" | "z" => (1024, 7),
            "YiB" | "yiB" | "Y" | "y" => (1024, 8),
            "RiB" | "riB" | "R" | "r" => (1024, 9),
            "QiB" | "qiB" | "Q" | "q" => (1024, 10),
            "KB" | "kB" | "KD" | "kD" => (1000, 1),
            "MB" | "mB" | "MD" | "mD" => (1000, 2),
            "GB" | "gB" | "GD" | "gD" => (1000, 3),
            "TB" | "tB" | "TD" | "tD" => (1000, 4),
            "PB" | "pB" | "PD" | "pD" => (1000, 5),
            "EB" | "eB" | "ED" | "eD" => (1000, 6),
            "ZB" | "zB" | "ZD" | "zD" => (1000, 7),
            "YB" | "yB" | "YD" | "yD" => (1000, 8),
            "RB" | "rB" | "RD" | "rD" => (1000, 9),
            "QB" | "qB" | "QD" | "qD" => (1000, 10),
            _ if numeric_string.is_empty() => return Err(ParseSizeError::parse_failure(size)),
            _ => return Err(ParseSizeError::invalid_suffix(size)),
        };
        let factor = base.pow(exponent);

        // parse string into u128
        let number: u128 = match number_system {
            NumberSystem::Decimal => {
                if numeric_string.is_empty() && !self.no_empty_numeric {
                    1
                } else {
                    Self::parse_number(&numeric_string, 10, size)?
                }
            }
            NumberSystem::Octal => {
                let trimmed_string = numeric_string.trim_start_matches('0');
                Self::parse_number(trimmed_string, 8, size)?
            }
            NumberSystem::Hexadecimal => {
                let trimmed_string = numeric_string.trim_start_matches("0x");
                Self::parse_number(trimmed_string, 16, size)?
            }
            NumberSystem::Binary => {
                let trimmed_string = numeric_string.trim_start_matches("0b");
                Self::parse_number(trimmed_string, 2, size)?
            }
        };

        number
            .checked_mul(factor)
            .ok_or_else(|| ParseSizeError::size_too_big(size))
    }

    /// Explicit u128 alias for `parse()`
    pub fn parse_u128(&self, size: &str) -> Result<u128, ParseSizeError> {
        self.parse(size)
    }

    /// Same as `parse()` but tries to return u64
    pub fn parse_u64(&self, size: &str) -> Result<u64, ParseSizeError> {
        self.parse(size).and_then(|num_u128| {
            u64::try_from(num_u128).map_err(|_| ParseSizeError::size_too_big(size))
        })
    }

    /// Same as `parse_u64()`, except returns `u64::MAX` on overflow
    /// GNU lib/coreutils include similar functionality
    /// and GNU test suite checks this behavior for some utils (`split` for example)
    pub fn parse_u64_max(&self, size: &str) -> Result<u64, ParseSizeError> {
        let result = self.parse_u64(size);
        match result {
            Ok(_) => result,
            Err(error) => {
                if let ParseSizeError::SizeTooBig(_) = error {
                    Ok(u64::MAX)
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Same as `parse_u64_max()`, except for u128, i.e. returns `u128::MAX` on overflow
    pub fn parse_u128_max(&self, size: &str) -> Result<u128, ParseSizeError> {
        let result = self.parse_u128(size);
        match result {
            Ok(_) => result,
            Err(error) => {
                if let ParseSizeError::SizeTooBig(_) = error {
                    Ok(u128::MAX)
                } else {
                    Err(error)
                }
            }
        }
    }

    fn determine_number_system(size: &str) -> NumberSystem {
        if size.len() <= 1 {
            return NumberSystem::Decimal;
        }

        if size.starts_with("0x") {
            return NumberSystem::Hexadecimal;
        }

        // Binary prefix: "0b" followed by at least one binary digit (0 or 1)
        // Note: "0b" alone is treated as decimal 0 with suffix "b"
        if size
            .strip_prefix("0b")
            .is_some_and(|prefix| !prefix.is_empty())
        {
            return NumberSystem::Binary;
        }

        let num_digits: usize = size
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .len();
        let all_zeros = size.chars().all(|c| c == '0');
        if size.starts_with('0') && num_digits > 1 && !all_zeros {
            return NumberSystem::Octal;
        }

        NumberSystem::Decimal
    }

    fn parse_number(
        numeric_string: &str,
        radix: u32,
        original_size: &str,
    ) -> Result<u128, ParseSizeError> {
        u128::from_str_radix(numeric_string, radix).map_err(|e| match e.kind() {
            IntErrorKind::PosOverflow => ParseSizeError::size_too_big(original_size),
            _ => ParseSizeError::ParseFailure(original_size.to_string()),
        })
    }
}

pub fn allow_list_with_all_suffixes(units: &str) -> Vec<String> {
    let mut allow_list = Vec::with_capacity(4 * units.len());
    for unit in units.chars() {
        for suffix in &["", "iB", "B", "D"] {
            allow_list.push(format!("{unit}{suffix}"));
        }
    }
    allow_list
}

/// Parse a size string into a number of bytes
/// using Default Parser (no custom settings)
///
/// # Examples
///
/// ```rust
/// use uucore::parser::parse_size::parse_size_u128;
/// assert_eq!(Ok(123), parse_size_u128("123"));
/// assert_eq!(Ok(9 * 1000), parse_size_u128("9kB")); // kB is 1000
/// assert_eq!(Ok(2 * 1024), parse_size_u128("2K")); // K is 1024
/// assert_eq!(Ok(44251 * 1024), parse_size_u128("0xACDBK")); // hexadecimal
/// assert_eq!(Ok(10), parse_size_u128("0b1010")); // binary
/// assert_eq!(Ok(10 * 1024), parse_size_u128("0b1010K")); // binary with suffix
/// ```
pub fn parse_size_u128(size: &str) -> Result<u128, ParseSizeError> {
    Parser::default().parse(size)
}

/// Same as `parse_size_u128()`, but for u64
pub fn parse_size_u64(size: &str) -> Result<u64, ParseSizeError> {
    Parser::default().parse_u64(size)
}

/// Same as `parse_size_u64()`, except 0 fails to parse
pub fn parse_size_non_zero_u64(size: &str) -> Result<u64, ParseSizeError> {
    let v = Parser::default().parse_u64(size)?;
    if v == 0 {
        return Err(ParseSizeError::ParseFailure("0".to_string()));
    }
    Ok(v)
}

/// Same as `parse_size_u64()` - deprecated
#[deprecated = "Please use parse_size_u64(size: &str) -> Result<u64, ParseSizeError> OR parse_size_u128(size: &str) -> Result<u128, ParseSizeError> instead."]
pub fn parse_size(size: &str) -> Result<u64, ParseSizeError> {
    parse_size_u64(size)
}

/// Same as `parse_size_u64()`, except returns `u64::MAX` on overflow
/// GNU lib/coreutils include similar functionality
/// and GNU test suite checks this behavior for some utils
pub fn parse_size_u64_max(size: &str) -> Result<u64, ParseSizeError> {
    Parser::default().parse_u64_max(size)
}

/// Same as `parse_size_u128()`, except returns `u128::MAX` on overflow
pub fn parse_size_u128_max(size: &str) -> Result<u128, ParseSizeError> {
    Parser::default().parse_u128_max(size)
}

/// Error type for parse_size
#[derive(Debug, PartialEq, Eq)]
pub enum ParseSizeError {
    /// Suffix
    InvalidSuffix(String),

    /// Syntax
    ParseFailure(String),

    /// Overflow
    SizeTooBig(String),

    /// Could not determine total physical memory size.
    PhysicalMem(String),
}

impl Error for ParseSizeError {
    fn description(&self) -> &str {
        match *self {
            Self::InvalidSuffix(ref s)
            | Self::ParseFailure(ref s)
            | Self::SizeTooBig(ref s)
            | Self::PhysicalMem(ref s) => s,
        }
    }
}

impl fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s = match self {
            Self::InvalidSuffix(s)
            | Self::ParseFailure(s)
            | Self::SizeTooBig(s)
            | Self::PhysicalMem(s) => s,
        };
        write!(f, "{s}")
    }
}

// FIXME: It's more idiomatic to move the formatting into the Display impl,
// but there's a lot of downstream code that constructs these errors manually
// that would be affected
impl ParseSizeError {
    fn invalid_suffix(s: &str) -> Self {
        Self::InvalidSuffix(format!("{}", s.quote()))
    }

    fn parse_failure(s: &str) -> Self {
        // stderr on linux (GNU coreutils 8.32) (LC_ALL=C)
        // has to be handled in the respective uutils because strings differ, e.g.:
        //
        // `NUM`
        // head:     invalid number of bytes: '1fb'
        // tail:     invalid number of bytes: '1fb'
        //
        // `SIZE`
        // split:    invalid number of bytes: '1fb'
        // truncate: Invalid number: '1fb'
        //
        // `MODE`
        // stdbuf:   invalid mode '1fb'
        //
        // `SIZE`
        // sort:     invalid suffix in --buffer-size argument '1fb'
        // sort:     invalid --buffer-size argument 'fb'
        //
        // `SIZE`
        // du:       invalid suffix in --buffer-size argument '1fb'
        // du:       invalid suffix in --threshold argument '1fb'
        // du:       invalid --buffer-size argument 'fb'
        // du:       invalid --threshold argument 'fb'
        //
        // `BYTES`
        // od:       invalid suffix in --read-bytes argument '1fb'
        // od:       invalid --read-bytes argument  argument 'fb'
        //                   --skip-bytes
        //                   --width
        //                   --strings
        // etc.
        Self::ParseFailure(format!("{}", s.quote()))
    }

    /// Where in `size` this error belongs.
    ///
    /// The parser splits a SIZE into the number it starts with and the unit
    /// that follows, and every failure is about one or the other. The split
    /// is worked out the same way here, so the caret covers exactly the part
    /// the parser rejected.
    ///
    /// # Arguments
    ///
    /// * `size` - The SIZE operand as typed, the one this error is about.
    pub fn span(&self, size: &str) -> Range<usize> {
        let number = numeric_prefix(size, Parser::determine_number_system(size)).len();
        match self {
            // The number is fine; it is the unit that is not.
            Self::InvalidSuffix(_) => number..size.len(),
            // The number is what does not fit.
            Self::SizeTooBig(_) => 0..number,
            // Nothing usable was read: the operand as a whole is at fault.
            Self::ParseFailure(_) | Self::PhysicalMem(_) => 0..size.len(),
        }
    }

    /// Render this error against `snapshot`, with a caret under the part of the
    /// SIZE that is at fault.
    ///
    /// Every utility taking a SIZE takes the same syntax, so the label and the
    /// advice are written once here rather than in each of them.
    ///
    /// # Arguments
    ///
    /// * `args` - The whole argument list, program name included — as
    ///   [`crate::diagnostics::capture`] returns it.
    /// * `option` - The option's value as typed, and the option it was given
    ///   to. The value may carry something in front of the size — `truncate`
    ///   takes a mode character, as in `+2K`, `head` and `tail` a sign — which
    ///   the caret has to count but the parser never saw.
    /// * `size_at` - Where the size itself starts inside the value, zero when
    ///   the whole of it is the size.
    /// * `message` - The headline, already localized. It differs between
    ///   utilities, so it is passed in rather than built here.
    ///
    /// # Returns
    ///
    /// `false` when no argument carries the value as that option's value, in
    /// which case the caller should fall back to the plain one-line message.
    pub fn render_size_value(
        &self,
        args: &[std::ffi::OsString],
        option: &crate::diagnostics::OptionValue,
        size_at: usize,
        message: &str,
    ) -> bool {
        let Some(size) = option.value.get(size_at..) else {
            return false;
        };
        // Labelled only where a label would add to the message, per the
        // convention in `crate::diagnostics`.
        let label = match self {
            Self::InvalidSuffix(_) => Some(crate::translate!("size-diag-label-invalid-suffix")),
            Self::SizeTooBig(_) => Some(crate::translate!("size-diag-label-too-big")),
            Self::ParseFailure(_) | Self::PhysicalMem(_) => None,
        };
        let span = self.span(size);
        crate::diagnostics::Snapshot::with_program(args).render_option(
            option,
            size_at + span.start..size_at + span.end,
            message,
            label.as_deref(),
            Some(&crate::translate!("size-diag-help-syntax")),
        )
    }

    /// The error to raise for a SIZE that does not parse.
    ///
    /// Draws the caret when the arguments as typed were kept, and quiets
    /// `error` when it did: the report has already said everything the
    /// one-line message would, and the exit code is all that is left to
    /// carry.
    ///
    /// # Arguments
    ///
    /// * `diag_args` - The arguments as typed, or `None` when they were not
    ///   kept — as [`crate::diagnostics::capture`] returns them.
    /// * `option`, `size_at`, `message` - As for [`Self::render_size_value`].
    /// * `error` - The error to raise if nothing was drawn.
    pub fn size_value_error(
        &self,
        diag_args: Option<&[std::ffi::OsString]>,
        option: &crate::diagnostics::OptionValue,
        size_at: usize,
        message: &str,
        error: impl Into<Box<dyn crate::error::UError>>,
    ) -> Box<dyn crate::error::UError> {
        crate::diagnostics::error_after_report(diag_args, error, |args, _| {
            self.render_size_value(args, option, size_at, message)
        })
    }

    fn size_too_big(s: &str) -> Self {
        // stderr on linux (GNU coreutils 8.32) (LC_ALL=C)
        // has to be handled in the respective uutils because strings differ, e.g.:
        //
        // head:     invalid number of bytes: '1Y': Value too large for defined data type
        // tail:     invalid number of bytes: '1Y': Value too large for defined data type
        // split:    invalid number of bytes: '1Y': Value too large for defined data type
        // truncate:          Invalid number: '1Y': Value too large for defined data type
        // stdbuf:               invalid mode '1Y': Value too large for defined data type
        // sort:     -S argument '1Y' too large
        // du:       -B argument '1Y' too large
        // od:       -N argument '1Y' too large
        // etc.
        //
        // stderr on macos (brew - GNU coreutils 8.32) also differs for the same version, e.g.:
        // ghead:   invalid number of bytes: '1Y': Value too large to be stored in data type
        // gtail:   invalid number of bytes: '1Y': Value too large to be stored in data type
        Self::SizeTooBig(format!(
            "{}: Value too large for defined data type",
            s.quote()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant_eq(a: &ParseSizeError, b: &ParseSizeError) -> bool {
        std::mem::discriminant(a) == std::mem::discriminant(b)
    }

    #[test]
    fn all_suffixes() {
        // Units  are  K,M,G,T,P,E,Z,Y,R,Q (powers of 1024) or KB,MB,... (powers of 1000).
        // Binary prefixes can be used, too: KiB=K, MiB=M, and so on.
        let suffixes = [
            ('K', 1u32),
            ('M', 2u32),
            ('G', 3u32),
            ('T', 4u32),
            ('P', 5u32),
            ('E', 6u32),
            ('Z', 7u32),
            ('Y', 8u32),
            ('R', 9u32),
            ('Q', 10u32),
        ];

        for &(c, exp) in &suffixes {
            let s = format!("2{c}B"); // KB
            assert_eq!(Ok(2 * 1000_u128.pow(exp)), parse_size_u128(&s));
            let s = format!("2{c}"); // K
            assert_eq!(Ok(2 * 1024_u128.pow(exp)), parse_size_u128(&s));
            let s = format!("2{c}iB"); // KiB
            assert_eq!(Ok(2 * 1024_u128.pow(exp)), parse_size_u128(&s));
            let s = format!("2{}iB", c.to_lowercase()); // kiB
            assert_eq!(Ok(2 * 1024_u128.pow(exp)), parse_size_u128(&s));

            // suffix only
            let s = format!("{c}B"); // KB
            assert_eq!(Ok(1000_u128.pow(exp)), parse_size_u128(&s));
            let s = format!("{c}"); // K
            assert_eq!(Ok(1024_u128.pow(exp)), parse_size_u128(&s));
            let s = format!("{c}iB"); // KiB
            assert_eq!(Ok(1024_u128.pow(exp)), parse_size_u128(&s));
            let s = format!("{}iB", c.to_lowercase()); // kiB
            assert_eq!(Ok(1024_u128.pow(exp)), parse_size_u128(&s));
        }
    }

    #[test]
    fn overflow_x64() {
        assert!(parse_size_u64("10000000000000000000000").is_err());
        assert!(parse_size_u64("1000000000T").is_err());
        assert!(parse_size_u64("100000P").is_err());
        assert!(parse_size_u64("100E").is_err());
        assert!(parse_size_u64("1Z").is_err());
        assert!(parse_size_u64("1Y").is_err());
        assert!(parse_size_u64("1R").is_err());
        assert!(parse_size_u64("1Q").is_err());
        assert!(parse_size_u64("0b1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111").is_err());

        assert!(variant_eq(
            &parse_size_u64("1Z").unwrap_err(),
            &ParseSizeError::SizeTooBig(String::new())
        ));

        assert_eq!(
            ParseSizeError::SizeTooBig("'1Y': Value too large for defined data type".to_string()),
            parse_size_u64("1Y").unwrap_err()
        );
        assert_eq!(
            ParseSizeError::SizeTooBig("'1R': Value too large for defined data type".to_string()),
            parse_size_u64("1R").unwrap_err()
        );
        assert_eq!(
            ParseSizeError::SizeTooBig("'1Q': Value too large for defined data type".to_string()),
            parse_size_u64("1Q").unwrap_err()
        );
    }

    #[test]
    fn overflow_to_max_u64() {
        assert_eq!(Ok(1_099_511_627_776), parse_size_u64_max("1T"));
        assert_eq!(Ok(1_125_899_906_842_624), parse_size_u64_max("1P"));
        assert_eq!(Ok(u64::MAX), parse_size_u64_max("18446744073709551616"));
        assert_eq!(Ok(u64::MAX), parse_size_u64_max("10000000000000000000000"));
        assert_eq!(Ok(u64::MAX), parse_size_u64_max("1Y"));
        assert_eq!(Ok(u64::MAX), parse_size_u64_max("1R"));
        assert_eq!(Ok(u64::MAX), parse_size_u64_max("1Q"));
    }

    #[test]
    fn overflow_to_max_u128() {
        assert_eq!(
            Ok(12_379_400_392_853_802_748_991_242_240),
            parse_size_u128_max("10R")
        );
        assert_eq!(
            Ok(12_676_506_002_282_294_014_967_032_053_760),
            parse_size_u128_max("10Q")
        );
        assert_eq!(Ok(u128::MAX), parse_size_u128_max("1000000000000R"));
        assert_eq!(Ok(u128::MAX), parse_size_u128_max("1000000000Q"));
    }

    #[test]
    fn invalid_suffix() {
        let test_strings = ["5mib", "1eb", "1H"];
        for &test_string in &test_strings {
            assert_eq!(
                parse_size_u64(test_string).unwrap_err(),
                ParseSizeError::InvalidSuffix(format!("{}", test_string.quote()))
            );
        }
    }

    #[test]
    fn invalid_syntax() {
        let test_strings = ["biB", "-", "+", "", "-1", "∞"];
        for &test_string in &test_strings {
            assert_eq!(
                parse_size_u64(test_string).unwrap_err(),
                ParseSizeError::ParseFailure(format!("{}", test_string.quote()))
            );
        }
    }

    #[test]
    fn b_suffix() {
        assert_eq!(Ok(3 * 512), parse_size_u64("3b")); // b is 512
        assert_eq!(Ok(0), parse_size_u64("0b")); // b should be used as a suffix in this case instead of signifying binary
    }

    #[test]
    fn no_suffix() {
        assert_eq!(Ok(1234), parse_size_u64("1234"));
        assert_eq!(Ok(0), parse_size_u64("0"));
        assert_eq!(Ok(5), parse_size_u64("5"));
        assert_eq!(Ok(999), parse_size_u64("999"));
    }

    #[test]
    fn kilobytes_suffix() {
        assert_eq!(Ok(123 * 1000), parse_size_u64("123KB")); // KB is 1000
        assert_eq!(Ok(9 * 1000), parse_size_u64("9kB")); // kB is 1000
        assert_eq!(Ok(2 * 1024), parse_size_u64("2K")); // K is 1024
        assert_eq!(Ok(0), parse_size_u64("0K"));
        assert_eq!(Ok(0), parse_size_u64("0KB"));
        assert_eq!(Ok(1000), parse_size_u64("KB"));
        assert_eq!(Ok(1024), parse_size_u64("K"));
        assert_eq!(Ok(2000), parse_size_u64("2kB"));
        assert_eq!(Ok(4000), parse_size_u64("4KB"));
    }

    #[test]
    fn megabytes_suffix() {
        assert_eq!(Ok(123 * 1024 * 1024), parse_size_u64("123M"));
        assert_eq!(Ok(123 * 1000 * 1000), parse_size_u64("123MB"));
        assert_eq!(Ok(1024 * 1024), parse_size_u64("M"));
        assert_eq!(Ok(1000 * 1000), parse_size_u64("MB"));
        assert_eq!(Ok(2 * 1_048_576), parse_size_u64("2m"));
        assert_eq!(Ok(4 * 1_048_576), parse_size_u64("4M"));
        assert_eq!(Ok(2_000_000), parse_size_u64("2mB"));
        assert_eq!(Ok(4_000_000), parse_size_u64("4MB"));
    }

    #[test]
    fn gigabytes_suffix() {
        assert_eq!(Ok(1_073_741_824), parse_size_u64("1G"));
        assert_eq!(Ok(2_000_000_000), parse_size_u64("2GB"));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn x64() {
        assert_eq!(Ok(1_099_511_627_776), parse_size_u64("1T"));
        assert_eq!(Ok(1_125_899_906_842_624), parse_size_u64("1P"));
        assert_eq!(Ok(1_152_921_504_606_846_976), parse_size_u64("1E"));

        assert_eq!(Ok(1_180_591_620_717_411_303_424), parse_size_u128("1Z"));
        assert_eq!(Ok(1_208_925_819_614_629_174_706_176), parse_size_u128("1Y"));
        assert_eq!(
            Ok(1_237_940_039_285_380_274_899_124_224),
            parse_size_u128("1R")
        );
        assert_eq!(
            Ok(1_267_650_600_228_229_401_496_703_205_376),
            parse_size_u128("1Q")
        );

        assert_eq!(Ok(2_000_000_000_000), parse_size_u64("2TB"));
        assert_eq!(Ok(2_000_000_000_000_000), parse_size_u64("2PB"));
        assert_eq!(Ok(2_000_000_000_000_000_000), parse_size_u64("2EB"));

        assert_eq!(Ok(2_000_000_000_000_000_000_000), parse_size_u128("2ZB"));
        assert_eq!(
            Ok(2_000_000_000_000_000_000_000_000),
            parse_size_u128("2YB")
        );
        assert_eq!(
            Ok(2_000_000_000_000_000_000_000_000_000),
            parse_size_u128("2RB")
        );
        assert_eq!(
            Ok(2_000_000_000_000_000_000_000_000_000_000),
            parse_size_u128("2QB")
        );
    }

    #[test]
    fn parse_size_options() {
        let mut parser = Parser::default();

        parser
            .with_allow_list(&["k", "K", "G", "MB", "M"])
            .with_default_unit("K");

        assert_eq!(Ok(1024), parser.parse("1"));
        assert_eq!(Ok(2 * 1024), parser.parse("2"));
        assert_eq!(Ok(1000 * 1000), parser.parse("1MB"));
        assert_eq!(Ok(1024 * 1024), parser.parse("1M"));
        assert_eq!(Ok(1024 * 1024 * 1024), parser.parse("1G"));

        assert!(parser.parse("1T").is_err());
        assert!(parser.parse("1P").is_err());
        assert!(parser.parse("1E").is_err());

        parser
            .with_allow_list(&[
                "b", "k", "K", "m", "M", "MB", "g", "G", "t", "T", "P", "E", "Z", "Y", "R", "Q",
            ])
            .with_default_unit("K")
            .with_b_byte_count(true);

        assert_eq!(Ok(1024), parser.parse("1"));
        assert_eq!(Ok(2 * 1024), parser.parse("2"));
        assert_eq!(Ok(1000 * 1000), parser.parse("1MB"));
        assert_eq!(Ok(1024 * 1024), parser.parse("1M"));
        assert_eq!(Ok(1024 * 1024 * 1024), parser.parse("1G"));
        assert_eq!(
            Ok(1_237_940_039_285_380_274_899_124_224),
            parser.parse_u128("1R")
        );
        assert_eq!(
            Ok(1_267_650_600_228_229_401_496_703_205_376),
            parser.parse_u128("1Q")
        );

        assert_eq!(Ok(1), parser.parse("1b"));
        assert_eq!(Ok(1024), parser.parse("1024b"));
        assert_eq!(Ok(1024 * 1024 * 1024), parser.parse("1024Mb"));

        assert!(parser.parse("b").is_err());
        assert!(parser.parse("1B").is_err());
        assert!(parser.parse("B").is_err());
    }

    #[test]
    fn parse_octal_size() {
        assert_eq!(Ok(63), parse_size_u64("077"));
        assert_eq!(Ok(528), parse_size_u64("01020"));
        assert_eq!(Ok(668 * 1024), parse_size_u128("01234K"));
    }

    #[test]
    fn parse_hex_size() {
        assert_eq!(Ok(10), parse_size_u64("0xA"));
        assert_eq!(Ok(94722), parse_size_u64("0x17202"));
        assert_eq!(Ok(44251 * 1024), parse_size_u128("0xACDBK"));
    }

    #[test]
    fn parse_binary_size() {
        assert_eq!(Ok(44251), parse_size_u64("0b1010110011011011"));
        assert_eq!(Ok(44251 * 1024), parse_size_u64("0b1010110011011011K"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn parse_percent() {
        assert!(parse_size_u64("0%").is_ok());
        assert!(parse_size_u64("50%").is_ok());
        assert!(parse_size_u64("100%").is_ok());
        assert!(parse_size_u64("100000%").is_ok());
        assert!(parse_size_u64("-1%").is_err());
        assert!(parse_size_u64("1.0%").is_err());
        assert!(parse_size_u64("0x1%").is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn parse_percent_overflow() {
        // A percentage can fit in a u128 while its product with the total
        // physical memory does not. `u128::MAX` overflows for any machine
        // reporting more than 100 bytes of memory.
        let size = format!("{}%", u128::MAX);

        assert!(variant_eq(
            &parse_size_u128(&size).unwrap_err(),
            &ParseSizeError::SizeTooBig(String::new())
        ));
        assert_eq!(Ok(u128::MAX), parse_size_u128_max(&size));
        assert_eq!(Ok(u64::MAX), parse_size_u64_max(&size));
    }
}
