// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) hdsf ghead gtail ACDBK hexdigit

use std::error::Error;
use std::fmt;
use std::num::IntErrorKind;

use crate::display::Quotable;

/// Parser for sizes in SI or IEC units (multiples of 1000 or 1024 bytes).
///
/// The [`Parser::parse`] function performs the parse.
#[derive(Default)]
pub struct Parser<'parser> {
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
}

impl<'parser> Parser<'parser> {
    pub fn with_allow_list(&mut self, allow_list: &'parser [&str]) -> &mut Self {
        self.allow_list = Some(allow_list);
        self
    }

    pub fn with_default_unit(&mut self, default_unit: &'parser str) -> &mut Self {
        self.default_unit = Some(default_unit);
        self
    }

    pub fn with_b_byte_count(&mut self, value: bool) -> &mut Self {
        self.b_byte_count = value;
        self
    }

    /// Parse a size string into a number of bytes.
    ///
    /// A size string comprises an integer and an optional unit. The unit
    /// may be K, M, G, T, P, E, Z or Y (powers of 1024), or KB, MB,
    /// etc. (powers of 1000), or b which is 512.
    /// Binary prefixes can be used, too: KiB=K, MiB=M, and so on.
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
    /// use uucore::parse_size::parse_size;
    /// assert_eq!(Ok(123), parse_size("123"));
    /// assert_eq!(Ok(9 * 1000), parse_size("9kB")); // kB is 1000
    /// assert_eq!(Ok(2 * 1024), parse_size("2K")); // K is 1024
    /// assert_eq!(Ok(44251 * 1024), parse_size("0xACDBK"));
    /// ```
    pub fn parse(&self, size: &str) -> Result<u64, ParseSizeError> {
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
                .chain(size.chars().skip(2).take_while(|c| c.is_ascii_hexdigit()))
                .collect(),
            _ => size.chars().take_while(|c| c.is_ascii_digit()).collect(),
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
            "KB" | "kB" => (1000, 1),
            "MB" | "mB" => (1000, 2),
            "GB" | "gB" => (1000, 3),
            "TB" | "tB" => (1000, 4),
            "PB" | "pB" => (1000, 5),
            "EB" | "eB" => (1000, 6),
            "ZB" | "zB" => (1000, 7),
            "YB" | "yB" => (1000, 8),
            _ if numeric_string.is_empty() => return Err(ParseSizeError::parse_failure(size)),
            _ => return Err(ParseSizeError::invalid_suffix(size)),
        };
        let factor = match u64::try_from(base.pow(exponent)) {
            Ok(n) => n,
            Err(_) => return Err(ParseSizeError::size_too_big(size)),
        };

        // parse string into u64
        let number: u64 = match number_system {
            NumberSystem::Decimal => {
                if numeric_string.is_empty() {
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
        };

        number
            .checked_mul(factor)
            .ok_or_else(|| ParseSizeError::size_too_big(size))
    }

    fn determine_number_system(size: &str) -> NumberSystem {
        if size.len() <= 1 {
            return NumberSystem::Decimal;
        }

        if size.starts_with("0x") {
            return NumberSystem::Hexadecimal;
        }

        let num_digits: usize = size
            .chars()
            .take_while(|c| c.is_ascii_digit())
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
    ) -> Result<u64, ParseSizeError> {
        u64::from_str_radix(numeric_string, radix).map_err(|e| match e.kind() {
            IntErrorKind::PosOverflow => ParseSizeError::size_too_big(original_size),
            _ => ParseSizeError::ParseFailure(original_size.to_string()),
        })
    }
}

/// Parse a size string into a number of bytes.
///
/// A size string comprises an integer and an optional unit. The unit
/// may be K, M, G, T, P, E, Z or Y (powers of 1024), or KB, MB,
/// etc. (powers of 1000), or b which is 512.
/// Binary prefixes can be used, too: KiB=K, MiB=M, and so on.
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
/// use uucore::parse_size::parse_size;
/// assert_eq!(Ok(123), parse_size("123"));
/// assert_eq!(Ok(9 * 1000), parse_size("9kB")); // kB is 1000
/// assert_eq!(Ok(2 * 1024), parse_size("2K")); // K is 1024
/// ```
pub fn parse_size(size: &str) -> Result<u64, ParseSizeError> {
    Parser::default().parse(size)
}

/// Same as `parse_size()`, except returns `u64::MAX` on overflow
/// GNU lib/coreutils include similar functionality
/// and GNU test suite checks this behavior for some utils
pub fn parse_size_max(size: &str) -> Result<u64, ParseSizeError> {
    let result = Parser::default().parse(size);
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

#[derive(Debug, PartialEq, Eq)]
pub enum ParseSizeError {
    InvalidSuffix(String), // Suffix
    ParseFailure(String),  // Syntax
    SizeTooBig(String),    // Overflow
}

impl Error for ParseSizeError {
    fn description(&self) -> &str {
        match *self {
            Self::InvalidSuffix(ref s) => s,
            Self::ParseFailure(ref s) => s,
            Self::SizeTooBig(ref s) => s,
        }
    }
}

impl fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s = match self {
            Self::InvalidSuffix(s) | Self::ParseFailure(s) | Self::SizeTooBig(s) => s,
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
        // Units  are  K,M,G,T,P,E,Z,Y (powers of 1024) or KB,MB,... (powers of 1000).
        // Binary prefixes can be used, too: KiB=K, MiB=M, and so on.
        let suffixes = [
            ('K', 1u32),
            ('M', 2u32),
            ('G', 3u32),
            ('T', 4u32),
            ('P', 5u32),
            ('E', 6u32),
            // The following will always result ParseSizeError::SizeTooBig as they cannot fit in u64
            // ('Z', 7u32),
            // ('Y', 8u32),
        ];

        for &(c, exp) in &suffixes {
            let s = format!("2{c}B"); // KB
            assert_eq!(Ok((2 * (1000_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("2{c}"); // K
            assert_eq!(Ok((2 * (1024_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("2{c}iB"); // KiB
            assert_eq!(Ok((2 * (1024_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("2{}iB", c.to_lowercase()); // kiB
            assert_eq!(Ok((2 * (1024_u128).pow(exp)) as u64), parse_size(&s));

            // suffix only
            let s = format!("{c}B"); // KB
            assert_eq!(Ok(((1000_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("{c}"); // K
            assert_eq!(Ok(((1024_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("{c}iB"); // KiB
            assert_eq!(Ok(((1024_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("{}iB", c.to_lowercase()); // kiB
            assert_eq!(Ok(((1024_u128).pow(exp)) as u64), parse_size(&s));
        }
    }

    #[test]
    #[cfg(not(target_pointer_width = "128"))]
    fn overflow_x64() {
        assert!(parse_size("10000000000000000000000").is_err());
        assert!(parse_size("1000000000T").is_err());
        assert!(parse_size("100000P").is_err());
        assert!(parse_size("100E").is_err());
        assert!(parse_size("1Z").is_err());
        assert!(parse_size("1Y").is_err());

        assert!(variant_eq(
            &parse_size("1Z").unwrap_err(),
            &ParseSizeError::SizeTooBig(String::new())
        ));

        assert_eq!(
            ParseSizeError::SizeTooBig("'1Y': Value too large for defined data type".to_string()),
            parse_size("1Y").unwrap_err()
        );
    }

    #[test]
    #[cfg(not(target_pointer_width = "128"))]
    fn overflow_to_max_x64() {
        assert_eq!(Ok(u64::MAX), parse_size_max("18446744073709551616"));
        assert_eq!(Ok(u64::MAX), parse_size_max("10000000000000000000000"));
        assert_eq!(Ok(u64::MAX), parse_size_max("1Y"));
    }

    #[test]
    fn invalid_suffix() {
        let test_strings = ["5mib", "1eb", "1H"];
        for &test_string in &test_strings {
            assert_eq!(
                parse_size(test_string).unwrap_err(),
                ParseSizeError::InvalidSuffix(format!("{}", test_string.quote()))
            );
        }
    }

    #[test]
    fn invalid_syntax() {
        let test_strings = ["biB", "-", "+", "", "-1", "∞"];
        for &test_string in &test_strings {
            assert_eq!(
                parse_size(test_string).unwrap_err(),
                ParseSizeError::ParseFailure(format!("{}", test_string.quote()))
            );
        }
    }

    #[test]
    fn b_suffix() {
        assert_eq!(Ok(3 * 512), parse_size("3b")); // b is 512
    }

    #[test]
    fn no_suffix() {
        assert_eq!(Ok(1234), parse_size("1234"));
        assert_eq!(Ok(0), parse_size("0"));
        assert_eq!(Ok(5), parse_size("5"));
        assert_eq!(Ok(999), parse_size("999"));
    }

    #[test]
    fn kilobytes_suffix() {
        assert_eq!(Ok(123 * 1000), parse_size("123KB")); // KB is 1000
        assert_eq!(Ok(9 * 1000), parse_size("9kB")); // kB is 1000
        assert_eq!(Ok(2 * 1024), parse_size("2K")); // K is 1024
        assert_eq!(Ok(0), parse_size("0K"));
        assert_eq!(Ok(0), parse_size("0KB"));
        assert_eq!(Ok(1000), parse_size("KB"));
        assert_eq!(Ok(1024), parse_size("K"));
        assert_eq!(Ok(2000), parse_size("2kB"));
        assert_eq!(Ok(4000), parse_size("4KB"));
    }

    #[test]
    fn megabytes_suffix() {
        assert_eq!(Ok(123 * 1024 * 1024), parse_size("123M"));
        assert_eq!(Ok(123 * 1000 * 1000), parse_size("123MB"));
        assert_eq!(Ok(1024 * 1024), parse_size("M"));
        assert_eq!(Ok(1000 * 1000), parse_size("MB"));
        assert_eq!(Ok(2 * 1_048_576), parse_size("2m"));
        assert_eq!(Ok(4 * 1_048_576), parse_size("4M"));
        assert_eq!(Ok(2_000_000), parse_size("2mB"));
        assert_eq!(Ok(4_000_000), parse_size("4MB"));
    }

    #[test]
    fn gigabytes_suffix() {
        assert_eq!(Ok(1_073_741_824), parse_size("1G"));
        assert_eq!(Ok(2_000_000_000), parse_size("2GB"));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn x64() {
        assert_eq!(Ok(1_099_511_627_776), parse_size("1T"));
        assert_eq!(Ok(1_125_899_906_842_624), parse_size("1P"));
        assert_eq!(Ok(1_152_921_504_606_846_976), parse_size("1E"));
        assert_eq!(Ok(2_000_000_000_000), parse_size("2TB"));
        assert_eq!(Ok(2_000_000_000_000_000), parse_size("2PB"));
        assert_eq!(Ok(2_000_000_000_000_000_000), parse_size("2EB"));
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
                "b", "k", "K", "m", "M", "MB", "g", "G", "t", "T", "P", "E", "Z", "Y",
            ])
            .with_default_unit("K")
            .with_b_byte_count(true);

        assert_eq!(Ok(1024), parser.parse("1"));
        assert_eq!(Ok(2 * 1024), parser.parse("2"));
        assert_eq!(Ok(1000 * 1000), parser.parse("1MB"));
        assert_eq!(Ok(1024 * 1024), parser.parse("1M"));
        assert_eq!(Ok(1024 * 1024 * 1024), parser.parse("1G"));

        assert_eq!(Ok(1), parser.parse("1b"));
        assert_eq!(Ok(1024), parser.parse("1024b"));
        assert_eq!(Ok(1024 * 1024 * 1024), parser.parse("1024Mb"));

        assert!(parser.parse("b").is_err());
        assert!(parser.parse("1B").is_err());
        assert!(parser.parse("B").is_err());
    }

    #[test]
    fn parse_octal_size() {
        assert_eq!(Ok(63), parse_size("077"));
        assert_eq!(Ok(528), parse_size("01020"));
        assert_eq!(Ok(668 * 1024), parse_size("01234K"));
    }

    #[test]
    fn parse_hex_size() {
        assert_eq!(Ok(10), parse_size("0xA"));
        assert_eq!(Ok(94722), parse_size("0x17202"));
        assert_eq!(Ok(44251 * 1024), parse_size("0xACDBK"));
    }
}
