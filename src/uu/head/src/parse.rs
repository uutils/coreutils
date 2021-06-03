use std::convert::TryFrom;
use std::ffi::OsString;

#[derive(PartialEq, Debug)]
pub enum ParseError {
    Syntax,
    Overflow,
}
/// Parses obsolete syntax
/// head -NUM[kmzv] // spell-checker:disable-line
pub fn parse_obsolete(src: &str) -> Option<Result<impl Iterator<Item = OsString>, ParseError>> {
    let mut chars = src.char_indices();
    if let Some((_, '-')) = chars.next() {
        let mut num_end = 0usize;
        let mut has_num = false;
        let mut last_char = 0 as char;
        for (n, c) in &mut chars {
            if c.is_numeric() {
                has_num = true;
                num_end = n;
            } else {
                last_char = c;
                break;
            }
        }
        if has_num {
            match src[1..=num_end].parse::<usize>() {
                Ok(num) => {
                    let mut quiet = false;
                    let mut verbose = false;
                    let mut zero_terminated = false;
                    let mut multiplier = None;
                    let mut c = last_char;
                    loop {
                        // not that here, we only match lower case 'k', 'c', and 'm'
                        match c {
                            // we want to preserve order
                            // this also saves us 1 heap allocation
                            'q' => {
                                quiet = true;
                                verbose = false
                            }
                            'v' => {
                                verbose = true;
                                quiet = false
                            }
                            'z' => zero_terminated = true,
                            'c' => multiplier = Some(1),
                            'b' => multiplier = Some(512),
                            'k' => multiplier = Some(1024),
                            'm' => multiplier = Some(1024 * 1024),
                            '\0' => {}
                            _ => return Some(Err(ParseError::Syntax)),
                        }
                        if let Some((_, next)) = chars.next() {
                            c = next
                        } else {
                            break;
                        }
                    }
                    let mut options = Vec::new();
                    if quiet {
                        options.push(OsString::from("-q"))
                    }
                    if verbose {
                        options.push(OsString::from("-v"))
                    }
                    if zero_terminated {
                        options.push(OsString::from("-z"))
                    }
                    if let Some(n) = multiplier {
                        options.push(OsString::from("-c"));
                        let num = match num.checked_mul(n) {
                            Some(n) => n,
                            None => return Some(Err(ParseError::Overflow)),
                        };
                        options.push(OsString::from(format!("{}", num)));
                    } else {
                        options.push(OsString::from("-n"));
                        options.push(OsString::from(format!("{}", num)));
                    }
                    Some(Ok(options.into_iter()))
                }
                Err(_) => Some(Err(ParseError::Overflow)),
            }
        } else {
            None
        }
    } else {
        None
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
    for (n, c) in &mut chars {
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
                if chars.next().is_some() {
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
    fn obsolete(src: &str) -> Option<Result<Vec<String>, ParseError>> {
        let r = parse_obsolete(src);
        match r {
            Some(s) => match s {
                Ok(v) => Some(Ok(v.map(|s| s.to_str().unwrap().to_owned()).collect())),
                Err(e) => Some(Err(e)),
            },
            None => None,
        }
    }
    fn obsolete_result(src: &[&str]) -> Option<Result<Vec<String>, ParseError>> {
        Some(Ok(src.iter().map(|s| s.to_string()).collect()))
    }
    #[test]
    #[cfg(not(target_pointer_width = "128"))]
    fn test_parse_overflow_x64() {
        assert_eq!(parse_num("1Y"), Err(ParseError::Overflow));
        assert_eq!(parse_num("1Z"), Err(ParseError::Overflow));
        assert_eq!(parse_num("100E"), Err(ParseError::Overflow));
        assert_eq!(parse_num("100000P"), Err(ParseError::Overflow));
        assert_eq!(parse_num("1000000000T"), Err(ParseError::Overflow));
        assert_eq!(
            parse_num("10000000000000000000000"),
            Err(ParseError::Overflow)
        );
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
        assert_eq!(parse_num("-"), Err(ParseError::Syntax));
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
        assert_eq!(parse_num("5MB"), Ok((5 * 1000 * 1000, false)));
    }
    #[test]
    fn test_parse_numbers_obsolete() {
        assert_eq!(obsolete("-5"), obsolete_result(&["-n", "5"]));
        assert_eq!(obsolete("-100"), obsolete_result(&["-n", "100"]));
        assert_eq!(obsolete("-5m"), obsolete_result(&["-c", "5242880"]));
        assert_eq!(obsolete("-1k"), obsolete_result(&["-c", "1024"]));
        assert_eq!(obsolete("-2b"), obsolete_result(&["-c", "1024"]));
        assert_eq!(obsolete("-1mmk"), obsolete_result(&["-c", "1024"]));
        assert_eq!(obsolete("-1vz"), obsolete_result(&["-v", "-z", "-n", "1"]));
        assert_eq!(
            obsolete("-1vzqvq"), // spell-checker:disable-line
            obsolete_result(&["-q", "-z", "-n", "1"])
        );
        assert_eq!(obsolete("-1vzc"), obsolete_result(&["-v", "-z", "-c", "1"]));
        assert_eq!(
            obsolete("-105kzm"),
            obsolete_result(&["-z", "-c", "110100480"])
        );
    }
    #[test]
    fn test_parse_errors_obsolete() {
        assert_eq!(obsolete("-5n"), Some(Err(ParseError::Syntax)));
        assert_eq!(obsolete("-5c5"), Some(Err(ParseError::Syntax)));
    }
    #[test]
    fn test_parse_obsolete_no_match() {
        assert_eq!(obsolete("-k"), None);
        assert_eq!(obsolete("asd"), None);
    }
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_obsolete_overflow_x64() {
        assert_eq!(
            obsolete("-1000000000000000m"),
            Some(Err(ParseError::Overflow))
        );
        assert_eq!(
            obsolete("-10000000000000000000000"),
            Some(Err(ParseError::Overflow))
        );
    }
    #[test]
    #[cfg(target_pointer_width = "32")]
    fn test_parse_obsolete_overflow_x32() {
        assert_eq!(obsolete("-42949672960"), Some(Err(ParseError::Overflow)));
        assert_eq!(obsolete("-42949672k"), Some(Err(ParseError::Overflow)));
    }
}
