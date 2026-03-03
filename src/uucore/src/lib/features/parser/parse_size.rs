// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) hdsf ghead gtail ACDBK hexdigit MEMORYSTATUSEX

//! Parser for sizes in SI or IEC units (multiples of 1000 or 1024 bytes).

use std::error::Error;
use std::fmt;
use std::num::{IntErrorKind, ParseIntError};

use crate::display::Quotable;
#[cfg(any(target_os = "linux", target_os = "android"))]
use procfs::{Current, Meminfo};

/// Error arising from trying to compute system memory.
#[derive(Debug)]
enum SystemError {
    IOError,
    ParseError,
    #[cfg(any(
        target_os = "macos",
        all(
            not(target_os = "linux"),
            not(target_os = "macos"),
            not(target_os = "windows"),
            not(target_os = "android"),
            not(target_os = "freebsd"),
            not(target_os = "openbsd"),
            not(target_os = "netbsd"),
            not(target_os = "dragonfly")
        )
    ))]
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
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "windows"
))]
fn total_physical_memory() -> Result<u128, SystemError> {
    #[cfg(target_family = "unix")]
    {
        // Try sysconf first as it is more robust than parsing /proc/meminfo
        // which might be restricted or have an unexpected format in some environments (e.g. CI/containers).
        let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };

        if pages > 0 && page_size > 0 {
            return Ok((pages as u128).saturating_mul(page_size as u128));
        }

        // Fallback to /proc/meminfo on Linux/Android
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            if let Ok(info) = Meminfo::current() {
                let total = (info.mem_total as u128).saturating_mul(1024);
                if total > 0 {
                    return Ok(total);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut mem_info: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        mem_info.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        if unsafe { GlobalMemoryStatusEx(&raw mut mem_info) } != 0 {
            return Ok(mem_info.ullTotalPhys as u128);
        }
    }

    Err(SystemError::IOError)
}

/// Return the number of bytes of memory that appear to be currently available.
#[cfg(any(target_os = "linux", target_os = "android"))]
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
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn available_memory_bytes() -> Option<u128> {
    None
}

/// Get the total number of bytes of physical memory.
///
/// TODO Implement this for non-Linux systems.
#[cfg(target_os = "macos")]
fn total_physical_memory() -> Result<u128, SystemError> {
    use std::ptr;
    let mut mem: u64 = 0;
    let mut len = size_of::<u64>();
    let name = "hw.memsize\0";
    unsafe {
        if libc::sysctlbyname(
            name.as_ptr().cast::<libc::c_char>(),
            ptr::addr_of_mut!(mem).cast::<libc::c_void>(),
            ptr::addr_of_mut!(len),
            ptr::null_mut(),
            0,
        ) == 0
        {
            Ok(mem as u128)
        } else {
            Err(SystemError::NotFound)
        }
    }
}

#[cfg(all(
    not(target_os = "linux"),
    not(target_os = "macos"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd"),
    not(target_os = "netbsd"),
    not(target_os = "dragonfly")
))]
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

impl<'parser> Parser<'parser> {
    /// Change allow_list of the parser - whitelist for the suffix
    ///
    /// # Errors
    ///
    /// Returns `ParserBuilderError` if:
    /// - Any entry in the allow_list is not a valid unit
    /// - The allow_list contains duplicates
    /// - The current default_unit (if set) is not in the new allow_list
    pub fn with_allow_list(
        &mut self,
        allow_list: &'parser [&str],
    ) -> Result<&mut Self, ParserBuilderError> {
        // Validate the allow_list itself
        Self::validate_allow_list(allow_list)?;

        // Validate compatibility with current state
        self.validate_allow_list_compat(allow_list)?;

        self.allow_list = Some(allow_list);
        Ok(self)
    }

    /// Change default_unit of the parser - when no suffix is provided
    ///
    /// # Errors
    ///
    /// Returns `ParserBuilderError` if:
    /// - The unit string is empty or contains whitespace
    /// - The unit is not a valid unit string
    /// - The unit is not in the allow_list (if allow_list is set)
    /// - The unit is "b" and b_byte_count is true (ambiguous)
    pub fn with_default_unit(
        &mut self,
        default_unit: &'parser str,
    ) -> Result<&mut Self, ParserBuilderError> {
        // Validate unit string
        Self::is_valid_unit_string(default_unit).map_err(|reason| {
            ParserBuilderError::InvalidUnit {
                unit: default_unit.to_string(),
                reason,
            }
        })?;

        // Validate compatibility with current state
        self.validate_default_unit_compat(default_unit)?;

        self.default_unit = Some(default_unit);
        Ok(self)
    }

    /// Change b_byte_count of the parser - to treat "b" as a "byte count" instead of "block"
    ///
    /// # Errors
    ///
    /// Returns `ParserBuilderError` if:
    /// - The default_unit is "b" (would create ambiguity)
    pub fn with_b_byte_count(&mut self, value: bool) -> Result<&mut Self, ParserBuilderError> {
        // Check for conflict with default_unit="b"
        if value && self.default_unit == Some("b") {
            return Err(ParserBuilderError::BByteCountConflict {
                default_unit: "b".to_string(),
                b_byte_count: value,
            });
        }

        self.b_byte_count = value;
        Ok(self)
    }

    /// Change no_empty_numeric of the parser - to allow empty numeric strings
    ///
    /// This method always succeeds as there are no validation constraints.
    #[allow(clippy::unnecessary_wraps)]
    pub fn with_allow_empty_numeric(
        &mut self,
        value: bool,
    ) -> Result<&mut Self, ParserBuilderError> {
        self.no_empty_numeric = value;
        Ok(self)
    }

    /// Validate that a unit string is valid
    fn is_valid_unit_string(unit: &str) -> Result<(), String> {
        // Check empty
        if unit.is_empty() {
            return Err("empty string".to_string());
        }

        // Check whitespace
        if unit.trim() != unit || unit.chars().any(char::is_whitespace) {
            return Err("contains whitespace".to_string());
        }

        // Valid units based on test analysis (lines 540-580)
        const VALID_UNITS: &[&str] = &[
            // Single char uppercase (1024 powers)
            "K", "M", "G", "T", "P", "E", "Z", "Y", "R", "Q",
            // Single char lowercase (1024 powers) - GNU sort compatibility
            "k", "m", "g", "t", "p", "e", "z", "y", "r", "q",
            // Two char decimal (1000 powers)
            "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB", "RB", "QB",
            // Binary IEC (1024 powers)
            "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB", "RiB", "QiB",
            // Lowercase binary
            "kiB", "miB", "giB", "tiB", "piB", "eiB", "ziB", "yiB", "riB", "qiB",
            // Special
            "b", "B", "%",
        ];

        if !VALID_UNITS.contains(&unit) {
            return Err(format!("'{unit}' is not a valid unit"));
        }

        Ok(())
    }

    /// Validate an allow_list
    fn validate_allow_list(allow_list: &[&str]) -> Result<(), ParserBuilderError> {
        use std::collections::HashMap;

        let mut invalid = Vec::new();
        let mut seen = HashMap::new();

        for (idx, &unit) in allow_list.iter().enumerate() {
            // Check validity
            if let Err(_reason) = Self::is_valid_unit_string(unit) {
                invalid.push(unit.to_string());
                continue;
            }

            // Check duplicates
            if let Some(&prev_idx) = seen.get(unit) {
                return Err(ParserBuilderError::DuplicateInAllowList {
                    unit: unit.to_string(),
                    indices: vec![prev_idx, idx],
                });
            }
            seen.insert(unit, idx);
        }

        if !invalid.is_empty() {
            return Err(ParserBuilderError::InvalidAllowList {
                reason: "contains invalid units".to_string(),
                invalid_entries: invalid,
            });
        }

        Ok(())
    }

    /// Validate that default_unit is compatible with current state
    fn validate_default_unit_compat(&self, unit: &str) -> Result<(), ParserBuilderError> {
        // Check against allow_list if set
        if let Some(allow_list) = self.allow_list {
            if !allow_list.contains(&unit) {
                return Err(ParserBuilderError::UnitNotAllowed {
                    unit: unit.to_string(),
                    allowed: allow_list.iter().map(|s| (*s).to_string()).collect(),
                });
            }
        }

        // Check b_byte_count conflict
        if unit == "b" && self.b_byte_count {
            return Err(ParserBuilderError::BByteCountConflict {
                default_unit: unit.to_string(),
                b_byte_count: self.b_byte_count,
            });
        }

        Ok(())
    }

    /// Validate that allow_list is compatible with current state
    fn validate_allow_list_compat(&self, allow_list: &[&str]) -> Result<(), ParserBuilderError> {
        // Check that default_unit (if set) is in new allow_list
        if let Some(default_unit) = self.default_unit {
            if !allow_list.contains(&default_unit) {
                return Err(ParserBuilderError::UnitNotAllowed {
                    unit: default_unit.to_string(),
                    allowed: allow_list.iter().map(|s| (*s).to_string()).collect(),
                });
            }
        }

        Ok(())
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

        // Special case: for percentage, just compute the given fraction
        // of the total physical memory on the machine, if possible.
        if unit == "%" {
            let number: u128 = Self::parse_number(&numeric_string, 10, size)?;
            if number == 0 {
                return Ok(0);
            }
            return match total_physical_memory() {
                Ok(total) => Ok(number.saturating_mul(total) / 100),
                Err(_) => Err(ParseSizeError::PhysicalMem(size.to_string())),
            };
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
            "KB" | "kB" => (1000, 1),
            "MB" | "mB" => (1000, 2),
            "GB" | "gB" => (1000, 3),
            "TB" | "tB" => (1000, 4),
            "PB" | "pB" => (1000, 5),
            "EB" | "eB" => (1000, 6),
            "ZB" | "zB" => (1000, 7),
            "YB" | "yB" => (1000, 8),
            "RB" | "rB" => (1000, 9),
            "QB" | "qB" => (1000, 10),
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
        if let Some(prefix) = size.strip_prefix("0b") {
            if !prefix.is_empty() {
                return NumberSystem::Binary;
            }
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

/// Extracts the thousands separator flag from a block size string.
///
/// GNU coreutils uses a leading single quote (`'`) to indicate that output
/// should be formatted with locale-aware thousands separators.
///
/// # Arguments
/// * `size` - The block size string (e.g., `"'1"`, `"'1K"`, `"1024"`)
///
/// # Returns
/// A tuple of `(cleaned_string, use_thousands_separator)`
///
/// # Examples
/// ```
/// use uucore::features::parser::parse_size::extract_thousands_separator_flag;
/// assert_eq!(extract_thousands_separator_flag("'1"), ("1", true));
/// assert_eq!(extract_thousands_separator_flag("'1K"), ("1K", true));
/// assert_eq!(extract_thousands_separator_flag("1024"), ("1024", false));
/// assert_eq!(extract_thousands_separator_flag(""), ("", false));
/// ```
pub fn extract_thousands_separator_flag(size: &str) -> (&str, bool) {
    if let Some(stripped) = size.strip_prefix('\'') {
        (stripped, true)
    } else {
        (size, false)
    }
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
/// Error type for Parser builder configuration validation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParserBuilderError {
    /// Invalid unit string provided
    InvalidUnit { unit: String, reason: String },

    /// Unit not in allow_list
    UnitNotAllowed { unit: String, allowed: Vec<String> },

    /// Conflicting configuration detected
    ConflictingConfig {
        setting: String,
        previous: String,
        current: String,
    },

    /// Empty or invalid allow_list
    InvalidAllowList {
        reason: String,
        invalid_entries: Vec<String>,
    },

    /// Duplicate unit in allow_list
    DuplicateInAllowList { unit: String, indices: Vec<usize> },

    /// Case-sensitive conflict in allow_list
    CaseSensitiveConflict {
        units: Vec<String>,
        explanation: String,
    },

    /// Invalid combination of settings
    InvalidCombination {
        settings: Vec<String>,
        reason: String,
    },

    /// Default unit conflicts with b_byte_count setting
    BByteCountConflict {
        default_unit: String,
        b_byte_count: bool,
    },
}

impl fmt::Display for ParserBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidUnit { unit, reason } => {
                write!(f, "invalid unit {}: {reason}", unit.quote())
            }
            Self::UnitNotAllowed { unit, allowed } => {
                write!(
                    f,
                    "unit {} is not in allow list: allowed units are [{}]",
                    unit.quote(),
                    allowed.join(", ")
                )
            }
            Self::ConflictingConfig {
                setting,
                previous,
                current,
            } => {
                write!(
                    f,
                    "conflicting configuration for {setting}: previously set to {}, now {}",
                    previous.quote(),
                    current.quote()
                )
            }
            Self::InvalidAllowList {
                reason,
                invalid_entries,
            } => {
                write!(
                    f,
                    "invalid allow_list ({reason}): invalid entries: [{}]",
                    invalid_entries
                        .iter()
                        .map(|s| s.quote().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::DuplicateInAllowList { unit, indices } => {
                write!(
                    f,
                    "duplicate unit {} in allow_list at positions: {indices:?}",
                    unit.quote()
                )
            }
            Self::CaseSensitiveConflict { units, explanation } => {
                write!(
                    f,
                    "case-sensitive conflict in units [{}]: {explanation}",
                    units.join(", ")
                )
            }
            Self::InvalidCombination { settings, reason } => {
                write!(
                    f,
                    "invalid combination of settings [{}]: {reason}",
                    settings.join(", "),
                )
            }
            Self::BByteCountConflict {
                default_unit,
                b_byte_count,
            } => {
                write!(
                    f,
                    "default_unit {} conflicts with b_byte_count={b_byte_count}",
                    default_unit.quote(),
                )
            }
        }
    }
}

impl Error for ParserBuilderError {}

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

    /// Builder configuration error
    BuilderConfig(ParserBuilderError),
}

impl Error for ParseSizeError {
    fn description(&self) -> &str {
        match *self {
            Self::InvalidSuffix(ref s) => s,
            Self::ParseFailure(ref s) => s,
            Self::SizeTooBig(ref s) => s,
            Self::PhysicalMem(ref s) => s,
            Self::BuilderConfig(_) => "builder configuration error",
        }
    }
}

impl fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::InvalidSuffix(s)
            | Self::ParseFailure(s)
            | Self::SizeTooBig(s)
            | Self::PhysicalMem(s) => write!(f, "{s}"),
            Self::BuilderConfig(e) => write!(f, "{e}"),
        }
    }
}

impl From<ParserBuilderError> for ParseSizeError {
    fn from(e: ParserBuilderError) -> Self {
        Self::BuilderConfig(e)
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
    fn parse_size_options() -> Result<(), ParserBuilderError> {
        let mut parser = Parser::default();

        parser
            .with_allow_list(&["k", "K", "G", "MB", "M"])?
            .with_default_unit("K")?;

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
            ])?
            .with_default_unit("K")?
            .with_b_byte_count(true)?;

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

        Ok(())
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
    fn test_extract_thousands_separator_flag() {
        // Valid inputs with quote
        assert_eq!(extract_thousands_separator_flag("'1"), ("1", true));
        assert_eq!(extract_thousands_separator_flag("'1K"), ("1K", true));
        assert_eq!(extract_thousands_separator_flag("'1024"), ("1024", true));
        assert_eq!(extract_thousands_separator_flag("'1kB"), ("1kB", true));
        assert_eq!(extract_thousands_separator_flag("'0"), ("0", true));
        assert_eq!(
            extract_thousands_separator_flag("'1234567890"),
            ("1234567890", true)
        );

        // Valid inputs without quote
        assert_eq!(extract_thousands_separator_flag("1"), ("1", false));
        assert_eq!(extract_thousands_separator_flag("1K"), ("1K", false));
        assert_eq!(extract_thousands_separator_flag("1024"), ("1024", false));
        assert_eq!(extract_thousands_separator_flag("1kB"), ("1kB", false));

        // Edge cases
        assert_eq!(extract_thousands_separator_flag(""), ("", false));
        assert_eq!(extract_thousands_separator_flag("'"), ("", true));
        assert_eq!(extract_thousands_separator_flag("''1"), ("'1", true));
        assert_eq!(extract_thousands_separator_flag("'''"), ("''", true));
    }
}
