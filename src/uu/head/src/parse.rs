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
    // we're not using the bytefmt or byte_unit crates
    // because they aren't fully `head` format compliant

    if src.is_empty() {
        return Err(ParseError::Syntax);
    }

    // (1) is the number
    // (2) is b
    // (3) is any of the other units
    // (4) is `i`
    // (5) is `B`
    let re = regex::Regex::new(r"^-?(\d+)?(?:(b)|([kKmMgGtTpPeEzZyY])(?:(i)B|(B))?)?$").unwrap();

    match re.captures(src) {
        Some(cap) => {
            let number = match cap.get(1) {
                Some(n) => {
                    // we're matching digits so it's ok to unwrap
                    n.as_str().parse().unwrap()
                }
                None => 1usize,
            };
            //`{unit}iB` means base2 as well as just `{unit}`
            let is_base_2 = cap.get(4).is_some() || cap.get(5).is_none();

            let multiplier = if cap.get(2).is_some() {
                512
            } else if let Some(unit) = cap.get(3) {
                //this is safe as the first byte cannot be
                //part of a multi-byte character if it is
                //an ascii character
                match &unit.as_str().as_bytes()[0] {
                    b'k' | b'K' => {
                        if is_base_2 {
                            1024u128.pow(1)
                        } else {
                            1000u128.pow(1)
                        }
                    }
                    b'm' | b'M' => {
                        if is_base_2 {
                            1024u128.pow(2)
                        } else {
                            1000u128.pow(2)
                        }
                    }
                    b'g' | b'G' => {
                        if is_base_2 {
                            1024u128.pow(3)
                        } else {
                            1000u128.pow(3)
                        }
                    }
                    b't' | b'T' => {
                        if is_base_2 {
                            1024u128.pow(4)
                        } else {
                            1000u128.pow(4)
                        }
                    }
                    b'p' | b'P' => {
                        if is_base_2 {
                            1024u128.pow(5)
                        } else {
                            1000u128.pow(5)
                        }
                    }
                    b'e' | b'E' => {
                        if is_base_2 {
                            1024u128.pow(6)
                        } else {
                            1000u128.pow(6)
                        }
                    }
                    b'z' | b'Z' => {
                        if is_base_2 {
                            1024u128.pow(7)
                        } else {
                            1000u128.pow(7)
                        }
                    }
                    b'y' | b'Y' => {
                        if is_base_2 {
                            1024u128.pow(8)
                        } else {
                            1000u128.pow(8)
                        }
                    }
                    _ => {
                        // this branch should never run since the regex
                        // will only match an argument with valid branches
                        // in this case we just crash - something's gone very wrong
                        panic!("Fatal error parsing arguments")
                    }
                }
            } else {
                1
            };

            let number = match usize::try_from(multiplier) {
                Ok(n) => match number.checked_mul(n) {
                    Some(n) => n,
                    None => return Err(ParseError::Overflow),
                },
                Err(_) => return Err(ParseError::Overflow),
            };
            // again, first byte
            // the regex would've failed if src was empty,
            // i.e. we can index here safely
            Ok((number, src.as_bytes()[0] == b'-'))
        }
        None => Err(ParseError::Syntax),
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
