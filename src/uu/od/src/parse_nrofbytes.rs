use uucore::parse_size::{parse_size, ParseSizeError};

pub fn parse_number_of_bytes(s: &str) -> Result<u64, ParseSizeError> {
    let mut start = 0;
    let mut len = s.len();
    let mut radix = 16;
    let mut multiply = 1;

    if s.starts_with("0x") || s.starts_with("0X") {
        start = 2;
    } else if s.starts_with('0') {
        radix = 8;
    } else {
        return parse_size(&s[start..]);
    }

    let mut ends_with = s.chars().rev();
    match ends_with.next() {
        Some('b') if radix != 16 => {
            multiply = 512;
            len -= 1;
        }
        Some('k') | Some('K') => {
            multiply = 1024;
            len -= 1;
        }
        Some('m') | Some('M') => {
            multiply = 1024 * 1024;
            len -= 1;
        }
        Some('G') => {
            multiply = 1024 * 1024 * 1024;
            len -= 1;
        }
        #[cfg(target_pointer_width = "64")]
        Some('T') => {
            multiply = 1024 * 1024 * 1024 * 1024;
            len -= 1;
        }
        #[cfg(target_pointer_width = "64")]
        Some('P') => {
            multiply = 1024 * 1024 * 1024 * 1024 * 1024;
            len -= 1;
        }
        #[cfg(target_pointer_width = "64")]
        Some('E') => {
            multiply = 1024 * 1024 * 1024 * 1024 * 1024 * 1024;
            len -= 1;
        }
        Some('B') if radix != 16 => {
            len -= 2;
            multiply = match ends_with.next() {
                Some('k') | Some('K') => 1000,
                Some('m') | Some('M') => 1000 * 1000,
                Some('G') => 1000 * 1000 * 1000,
                #[cfg(target_pointer_width = "64")]
                Some('T') => 1000 * 1000 * 1000 * 1000,
                #[cfg(target_pointer_width = "64")]
                Some('P') => 1000 * 1000 * 1000 * 1000 * 1000,
                #[cfg(target_pointer_width = "64")]
                Some('E') => 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                _ => return Err(ParseSizeError::ParseFailure(s.to_string())),
            }
        }
        _ => {}
    }

    let factor = match u64::from_str_radix(&s[start..len], radix) {
        Ok(f) => f,
        Err(e) => return Err(ParseSizeError::ParseFailure(e.to_string())),
    };
    factor
        .checked_mul(multiply)
        .ok_or_else(|| ParseSizeError::SizeTooBig(s.to_string()))
}

#[test]
fn test_parse_number_of_bytes() {
    // octal input
    assert_eq!(8, parse_number_of_bytes("010").unwrap());
    assert_eq!(8 * 512, parse_number_of_bytes("010b").unwrap());
    assert_eq!(8 * 1024, parse_number_of_bytes("010k").unwrap());
    assert_eq!(8 * 1_048_576, parse_number_of_bytes("010m").unwrap());

    // hex input
    assert_eq!(15, parse_number_of_bytes("0xf").unwrap());
    assert_eq!(15, parse_number_of_bytes("0XF").unwrap());
    assert_eq!(27, parse_number_of_bytes("0x1b").unwrap());
    assert_eq!(16 * 1024, parse_number_of_bytes("0x10k").unwrap());
    assert_eq!(16 * 1_048_576, parse_number_of_bytes("0x10m").unwrap());
}
