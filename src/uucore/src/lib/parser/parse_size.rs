//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) hdsf ghead gtail

use std::convert::TryFrom;
use std::error::Error;
use std::fmt;

use crate::display::Quotable;

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
    if size.is_empty() {
        return Err(ParseSizeError::parse_failure(size));
    }
    // Get the numeric part of the size argument. For example, if the
    // argument is "123K", then the numeric part is "123".
    let numeric_string: String = size.chars().take_while(|c| c.is_digit(10)).collect();
    let number: u64 = if !numeric_string.is_empty() {
        match numeric_string.parse() {
            Ok(n) => n,
            Err(_) => return Err(ParseSizeError::parse_failure(size)),
        }
    } else {
        1
    };

    // Get the alphabetic units part of the size argument and compute
    // the factor it represents. For example, if the argument is "123K",
    // then the unit part is "K" and the factor is 1024. This may be the
    // empty string, in which case, the factor is 1.
    let unit = &size[numeric_string.len()..];
    let (base, exponent): (u128, u32) = match unit {
        "B" | "" => (1, 0),
        "b" => (512, 1), // (`od`, `head` and `tail` use "b")
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
        _ => return Err(ParseSizeError::parse_failure(size)),
    };
    let factor = match u64::try_from(base.pow(exponent)) {
        Ok(n) => n,
        Err(_) => return Err(ParseSizeError::size_too_big(size)),
    };
    number
        .checked_mul(factor)
        .ok_or_else(|| ParseSizeError::size_too_big(size))
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseSizeError {
    ParseFailure(String), // Syntax
    SizeTooBig(String),   // Overflow
}

impl Error for ParseSizeError {
    fn description(&self) -> &str {
        match *self {
            ParseSizeError::ParseFailure(ref s) => &*s,
            ParseSizeError::SizeTooBig(ref s) => &*s,
        }
    }
}

impl fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s = match self {
            ParseSizeError::ParseFailure(s) | ParseSizeError::SizeTooBig(s) => s,
        };
        write!(f, "{}", s)
    }
}

// FIXME: It's more idiomatic to move the formatting into the Display impl,
// but there's a lot of downstream code that constructs these errors manually
// that would be affected
impl ParseSizeError {
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
            let s = format!("2{}B", c); // KB
            assert_eq!(Ok((2 * (1000_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("2{}", c); // K
            assert_eq!(Ok((2 * (1024_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("2{}iB", c); // KiB
            assert_eq!(Ok((2 * (1024_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("2{}iB", c.to_lowercase()); // kiB
            assert_eq!(Ok((2 * (1024_u128).pow(exp)) as u64), parse_size(&s));

            // suffix only
            let s = format!("{}B", c); // KB
            assert_eq!(Ok(((1000_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("{}", c); // K
            assert_eq!(Ok(((1024_u128).pow(exp)) as u64), parse_size(&s));
            let s = format!("{}iB", c); // KiB
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
    fn invalid_syntax() {
        let test_strings = [
            "328hdsf3290",
            "5MiB nonsense",
            "5mib",
            "biB",
            "-",
            "+",
            "",
            "-1",
            "1e2",
            "∞",
        ];
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
    fn bytes_suffix() {
        assert_eq!(Ok(1234), parse_size("1234B"));
        assert_eq!(Ok(0), parse_size("0B"));
        assert_eq!(Ok(5), parse_size("5B"));
        assert_eq!(Ok(999), parse_size("999B"));
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
}
