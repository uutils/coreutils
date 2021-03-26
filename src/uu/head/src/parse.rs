use std::convert::TryFrom;

#[derive(PartialEq, Debug)]
pub enum ParseError {
    Syntax,
    Overflow,
}
/// Parses obsolete syntax
/// head -NUM
/// (note:)
pub fn parse_obsolete(src: &str) -> Option<Result<(usize, bool), ParseError>> {
    let re = regex::Regex::new(r"^-(\d+)(?:([cbkm])+)?$").unwrap();
    match re.captures(src) {
        Some(cap) => {
            let num = cap.get(1).unwrap();
            let mut num = match num.as_str().parse::<usize>() {
                Ok(n) => n,
                Err(_) => {
                    return Some(Err(ParseError::Overflow));
                }
            };
            let mut bytes = false;
            if let Some(m) = cap.get(2) {
                let newn = match m.as_str().chars().next().unwrap_or(0 as char) {
                    'c' => num.checked_mul(1),
                    'b' => num.checked_mul(512),
                    'k' => num.checked_mul(1024),
                    'm' => num.checked_mul(1024 * 1024),
                    _ => unreachable!(),
                };
                match newn {
                    Some(n) => {
                        num = n;
                        bytes = true;
                    }
                    None => return Some(Err(ParseError::Overflow)),
                }
            }
            Some(Ok((num, bytes)))
        }
        None => None,
    }
}
/// Parses an -c or -n argument,
/// the bool specifies whether to read from the end
pub fn parse_num(src: &str) -> Result<(usize, bool), ParseError> {
    let mut num_start = 0;
    let mut chars = src.char_indices();
    let (mut chars, all_but_last) = match chars.next() {
        Some((_, c)) => {
            if c == '-' {
                num_start += 1;
                (chars, true)
            } else {
                (src.char_indices(), false)
            }
        }
        None => return Err(ParseError::Syntax),
    };
    let mut num_end = 0usize;
    let mut last_char = 0 as char;
    let mut num_count = 0usize;
    while let Some((n, c)) = chars.next() {
        if c.is_numeric() {
            num_end = n;
            num_count += 1;
        } else {
            last_char = c;
            break;
        }
    }

    let num = if num_count > 0 {
        match src[num_start..=num_end].parse::<usize>() {
            Ok(n) => Some(n),
            Err(_) => return Err(ParseError::Overflow),
        }
    } else {
        None
    };

    if last_char == 0 as char {
        if let Some(n) = num {
            Ok((n, all_but_last))
        } else {
            Err(ParseError::Syntax)
        }
    } else {
        let base: u128 = match chars.next() {
            Some((_, c)) => {
                let b = match c {
                    'B' if last_char != 'b' => 1000,
                    'i' if last_char != 'b' => {
                        if let Some((_, 'B')) = chars.next() {
                            1024
                        } else {
                            return Err(ParseError::Syntax);
                        }
                    }
                    _ => return Err(ParseError::Syntax),
                };
                if let Some(_) = chars.next() {
                    return Err(ParseError::Syntax);
                } else {
                    b
                }
            }
            None => 1024,
        };
        let mul = match last_char.to_lowercase().next().unwrap() {
            'b' => 512,
            'k' => base.pow(1),
            'm' => base.pow(2),
            'g' => base.pow(3),
            't' => base.pow(4),
            'p' => base.pow(5),
            'e' => base.pow(6),
            'z' => base.pow(7),
            'y' => base.pow(8),
            _ => return Err(ParseError::Syntax),
        };
        let mul = match usize::try_from(mul) {
            Ok(n) => n,
            Err(_) => return Err(ParseError::Overflow),
        };
        match num.unwrap_or(1).checked_mul(mul) {
            Some(n) => Ok((n, all_but_last)),
            None => Err(ParseError::Overflow),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_overflow_x64() {
        assert_eq!(parse_num("1Y"), Err(ParseError::Overflow));
        assert_eq!(parse_num("1000000000T"), Err(ParseError::Overflow));
    }
    #[test]
    #[cfg(target_pointer_width = "32")]
    fn test_parse_overflow_x32() {
        assert_eq!(parse_num("1T"), Err(ParseError::Overflow));
        assert_eq!(parse_num("1000G"), Err(ParseError::Overflow));
    }
    #[test]
    fn test_parse_bad_syntax() {
        assert_eq!(parse_num("5MiB nonsense"), Err(ParseError::Syntax));
        assert_eq!(parse_num("Nonsense string"), Err(ParseError::Syntax));
        assert_eq!(parse_num("5mib"), Err(ParseError::Syntax));
        assert_eq!(parse_num("biB"), Err(ParseError::Syntax));
        assert_eq!(parse_num(""), Err(ParseError::Syntax));
    }
    #[test]
    fn test_parse_numbers() {
        assert_eq!(parse_num("k"), Ok((1024, false)));
        assert_eq!(parse_num("MiB"), Ok((1024 * 1024, false)));
        assert_eq!(parse_num("-5"), Ok((5, true)));
        assert_eq!(parse_num("b"), Ok((512, false)));
        assert_eq!(parse_num("-2GiB"), Ok((2 * 1024 * 1024 * 1024, true)));
        assert_eq!(parse_num("5M"), Ok((5 * 1024 * 1024, false)));
    }
    #[test]
    fn test_parse_numbers_obsolete() {
        assert_eq!(parse_obsolete("-5"), Some(Ok((5, false))));
        assert_eq!(parse_obsolete("-100"), Some(Ok((100, false))));
        assert_eq!(parse_obsolete("-5m"), Some(Ok((5 * 1024 * 1024, true))));
        assert_eq!(parse_obsolete("-1k"), Some(Ok((1024, true))));
        assert_eq!(parse_obsolete("-2b"), Some(Ok((1024, true))));
        assert_eq!(parse_obsolete("-1mmk"), Some(Ok((1024, true))));
    }
    #[test]
    fn test_parse_errors_obsolete() {
        assert_eq!(parse_obsolete("-5n"), None);
        assert_eq!(parse_obsolete("-5c5"), None);
    }
    #[test]
    fn test_parse_obsolete_nomatch() {
        assert_eq!(parse_obsolete("-k"), None);
        assert_eq!(parse_obsolete("asd"), None);
    }
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_obsolete_overflow_x64() {
        assert_eq!(
            parse_obsolete("-1000000000000000m"),
            Some(Err(ParseError::Overflow))
        );
        assert_eq!(
            parse_obsolete("-10000000000000000000000"),
            Some(Err(ParseError::Overflow))
        );
    }
    #[test]
    #[cfg(target_pointer_width = "32")]
    fn test_parse_obsolete_overflow_x32() {
        assert_eq!(
            parse_obsolete("-42949672960"),
            Some(Err(ParseError::Overflow))
        );
        assert_eq!(
            parse_obsolete("-42949672k"),
            Some(Err(ParseError::Overflow))
        );
    }
}
