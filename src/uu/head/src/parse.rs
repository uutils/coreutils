//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::ffi::OsString;
use uucore::parse_size::{parse_size, ParseSizeError};

#[derive(PartialEq, Debug)]
pub enum ParseError {
    Syntax,
    Overflow,
}
/// Parses obsolete syntax
/// head -NUM\[kmzv\] // spell-checker:disable-line
pub fn parse_obsolete(src: &str) -> Option<Result<impl Iterator<Item = OsString>, ParseError>> {
    let mut chars = src.char_indices();
    if let Some((_, '-')) = chars.next() {
        let mut num_end = 0usize;
        let mut has_num = false;
        let mut last_char = 0 as char;
        for (n, c) in &mut chars {
            if c.is_digit(10) {
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
                                verbose = false;
                            }
                            'v' => {
                                verbose = true;
                                quiet = false;
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
                            c = next;
                        } else {
                            break;
                        }
                    }
                    let mut options = Vec::new();
                    if quiet {
                        options.push(OsString::from("-q"));
                    }
                    if verbose {
                        options.push(OsString::from("-v"));
                    }
                    if zero_terminated {
                        options.push(OsString::from("-z"));
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
pub fn parse_num(src: &str) -> Result<(u64, bool), ParseSizeError> {
    let mut size_string = src.trim();
    let mut all_but_last = false;

    if let Some(c) = size_string.chars().next() {
        if c == '+' || c == '-' {
            // head: '+' is not documented (8.32 man pages)
            size_string = &size_string[1..];
            if c == '-' {
                all_but_last = true;
            }
        }
    } else {
        return Err(ParseSizeError::ParseFailure(src.to_string()));
    }

    parse_size(size_string).map(|n| (n, all_but_last))
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
