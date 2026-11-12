// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uucore::parser::parse_size::{ParseSizeError, parse_size_u64};

pub fn parse_number_of_bytes(s: &str) -> Result<u64, ParseSizeError> {
    // GNU's -j/-N number parser skips leading blanks and accepts a single
    // optional '+' before the number. The digits have to follow the sign
    // immediately, so a '-', a doubled sign, a sign on its own, or a blank
    // between the sign and the number is still rejected.
    const BLANKS: [char; 6] = [' ', '\t', '\n', '\x0b', '\x0c', '\r'];
    let number = {
        let no_blanks = s.trim_start_matches(BLANKS);
        no_blanks.strip_prefix('+').unwrap_or(no_blanks)
    };
    // The number must follow the sign immediately: a blank right after a '+' is
    // rejected (`parse_size_u64` would otherwise trim it). A '-', a doubled
    // sign, or a sign on its own leaves a value `parse_size_u64` already
    // rejects below.
    if number.starts_with(BLANKS) {
        return Err(ParseSizeError::ParseFailure(s.to_string()));
    }

    let mut start = 0;
    let mut len = number.len();
    let mut radix = 16;
    let mut multiply = 1;

    if number.starts_with("0x") || number.starts_with("0X") {
        start = 2;
    } else if number.starts_with('0') {
        radix = 8;
    } else {
        return parse_size_u64(number);
    }

    let mut ends_with = number.chars().rev();
    match ends_with.next() {
        Some('b') if radix != 16 => {
            multiply = 512;
            len -= 1;
        }
        Some('k' | 'K') => {
            multiply = 1024;
            len -= 1;
        }
        Some('m' | 'M') => {
            multiply = 1024 * 1024;
            len -= 1;
        }
        Some('G') => {
            multiply = 1024 * 1024 * 1024;
            len -= 1;
        }
        Some('T') => {
            multiply = 1024 * 1024 * 1024 * 1024;
            len -= 1;
        }
        Some('P') => {
            multiply = 1024 * 1024 * 1024 * 1024 * 1024;
            len -= 1;
        }
        Some('E') if radix != 16 => {
            multiply = 1024 * 1024 * 1024 * 1024 * 1024 * 1024;
            len -= 1;
        }
        Some('B') if radix != 16 => {
            len -= 2;
            multiply = match ends_with.next() {
                Some('k' | 'K') => 1000,
                Some('m' | 'M') => 1000 * 1000,
                Some('G') => 1000 * 1000 * 1000,
                Some('T') => 1000 * 1000 * 1000 * 1000,
                Some('P') => 1000 * 1000 * 1000 * 1000 * 1000,
                Some('E') => 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                _ => return Err(ParseSizeError::ParseFailure(s.to_string())),
            }
        }
        _ => {}
    }

    let factor = u64::from_str_radix(&number[start..len], radix)
        .map_err(|e| ParseSizeError::ParseFailure(e.to_string()))?;
    factor
        .checked_mul(multiply)
        .ok_or_else(|| ParseSizeError::SizeTooBig(s.to_string()))
}

/// Parses a plain decimal number, as `-w` takes: no radix prefix (a leading
/// zero is not octal) and no multiplier suffix.
pub fn parse_width(s: &str) -> Result<u64, ParseSizeError> {
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Err(ParseSizeError::ParseFailure(s.to_string()));
    }
    if digits < s.len() {
        return Err(ParseSizeError::InvalidSuffix(s.to_string()));
    }
    s.parse()
        .map_err(|_| ParseSizeError::SizeTooBig(s.to_string()))
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_parse_number_of_bytes() {
    // octal input
    assert_eq!(8, parse_number_of_bytes("010").unwrap());
    assert_eq!(8 * 512, parse_number_of_bytes("010b").unwrap());
    assert_eq!(8 * 1024, parse_number_of_bytes("010k").unwrap());
    assert_eq!(8 * 1_048_576, parse_number_of_bytes("010m").unwrap());

    // hex input
    assert_eq!(15, parse_number_of_bytes("0xf").unwrap());
    assert_eq!(14, parse_number_of_bytes("0XE").unwrap());
    assert_eq!(15, parse_number_of_bytes("0XF").unwrap());
    assert_eq!(27, parse_number_of_bytes("0x1b").unwrap());
    assert_eq!(16 * 1024, parse_number_of_bytes("0x10k").unwrap());
    assert_eq!(16 * 1_048_576, parse_number_of_bytes("0x10m").unwrap());

    // suffixes above 4 GiB, on 32-bit targets too
    assert_eq!(1 << 40, parse_number_of_bytes("01T").unwrap());
    assert_eq!(1 << 50, parse_number_of_bytes("01P").unwrap());
    assert_eq!(1 << 60, parse_number_of_bytes("01E").unwrap());
    assert_eq!(1_000_000_000_000, parse_number_of_bytes("01TB").unwrap());
    assert_eq!(
        1_000_000_000_000_000,
        parse_number_of_bytes("01PB").unwrap()
    );
    assert_eq!(
        1_000_000_000_000_000_000,
        parse_number_of_bytes("01EB").unwrap()
    );
    assert_eq!(1 << 40, parse_number_of_bytes("0x1T").unwrap());
    assert_eq!(1 << 50, parse_number_of_bytes("0x1P").unwrap());
}

#[test]
fn test_parse_number_of_bytes_leading_plus_and_blanks() {
    // A single leading '+' is accepted, before any radix prefix or suffix.
    assert_eq!(5, parse_number_of_bytes("+5").unwrap());
    assert_eq!(8, parse_number_of_bytes("+010").unwrap());
    assert_eq!(5, parse_number_of_bytes("+0x5").unwrap());
    assert_eq!(5 * 1024, parse_number_of_bytes("+5k").unwrap());

    // Leading blanks are skipped, with or without the sign.
    assert_eq!(5, parse_number_of_bytes("  5").unwrap());
    assert_eq!(5, parse_number_of_bytes(" +5").unwrap());

    // The number has to follow the sign immediately: a '-', a doubled sign, a
    // sign on its own, or a blank after the sign is a parse failure.
    for s in ["-5", "++5", "+-5", "+", "+ 5"] {
        assert!(matches!(
            parse_number_of_bytes(s),
            Err(ParseSizeError::ParseFailure(_))
        ));
    }
}

#[test]
fn test_parse_width() {
    assert_eq!(Ok(4), parse_width("4"));
    // Not octal.
    assert_eq!(Ok(10), parse_width("010"));
    assert_eq!(Ok(8), parse_width("08"));
    assert_eq!(Ok(0), parse_width("00"));
    assert_eq!(Ok(u64::MAX), parse_width("18446744073709551615"));

    for s in [
        "1K",
        "1k",
        "2b",
        "1KB",
        "0x8",
        "4B",
        "99999999999999999999K",
    ] {
        assert!(matches!(
            parse_width(s),
            Err(ParseSizeError::InvalidSuffix(_))
        ));
    }
    for s in ["", "abc", "-4", "+4", "K"] {
        assert!(matches!(
            parse_width(s),
            Err(ParseSizeError::ParseFailure(_))
        ));
    }
    assert!(matches!(
        parse_width("18446744073709551616"),
        Err(ParseSizeError::SizeTooBig(_))
    ));
}
