//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::ffi::OsString;

#[derive(PartialEq, Eq, Debug)]
pub enum ParseError {
    Overflow,
    Context,
}
/// Parses obsolete syntax
/// tail -\[NUM\]\[bl\]\[f\] and tail +\[NUM\]\[bcl\]\[f\] // spell-checker:disable-line
pub fn parse_obsolete(src: &str) -> Option<Result<impl Iterator<Item = OsString>, ParseError>> {
    let mut chars = src.chars();
    let sign = chars.next()?;
    if sign != '+' && sign != '-' {
        return None;
    }

    let numbers: String = chars.clone().take_while(|&c| c.is_ascii_digit()).collect();
    let has_num = !numbers.is_empty();
    let num: usize = if has_num {
        if let Ok(num) = numbers.parse() {
            num
        } else {
            return Some(Err(ParseError::Overflow));
        }
    } else {
        10
    };

    let mut follow = false;
    let mut mode = None;
    let mut first_char = true;
    for char in chars.skip_while(|&c| c.is_ascii_digit()) {
        if sign == '-' && char == 'c' && !has_num {
            // special case: -c should be handled by clap (is ambiguous)
            return None;
        } else if char == 'f' {
            follow = true;
        } else if first_char && (char == 'b' || char == 'c' || char == 'l') {
            mode = Some(char);
        } else if has_num && sign == '-' {
            return Some(Err(ParseError::Context));
        } else {
            return None;
        }
        first_char = false;
    }

    let mut options = Vec::new();
    if follow {
        options.push(OsString::from("-f"));
    }
    let mode = mode.unwrap_or('l');
    if mode == 'b' || mode == 'c' {
        options.push(OsString::from("-c"));
        let n = if mode == 'b' { 512 } else { 1 };
        let num = match num.checked_mul(n) {
            Some(n) => n,
            None => return Some(Err(ParseError::Overflow)),
        };
        options.push(OsString::from(format!("{sign}{num}")));
    } else {
        options.push(OsString::from("-n"));
        options.push(OsString::from(format!("{sign}{num}")));
    }
    Some(Ok(options.into_iter()))
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
        assert_eq!(obsolete("+2c"), obsolete_result(&["-c", "+2"]));
        assert_eq!(obsolete("-5"), obsolete_result(&["-n", "-5"]));
        assert_eq!(obsolete("-100"), obsolete_result(&["-n", "-100"]));
        assert_eq!(obsolete("-2b"), obsolete_result(&["-c", "-1024"]));
    }
    #[test]
    fn test_parse_errors_obsolete() {
        assert_eq!(obsolete("-5n"), Some(Err(ParseError::Context)));
        assert_eq!(obsolete("-5c5"), Some(Err(ParseError::Context)));
        assert_eq!(obsolete("-1vzc"), Some(Err(ParseError::Context)));
        assert_eq!(obsolete("-5m"), Some(Err(ParseError::Context)));
        assert_eq!(obsolete("-1k"), Some(Err(ParseError::Context)));
        assert_eq!(obsolete("-1mmk"), Some(Err(ParseError::Context)));
        assert_eq!(obsolete("-105kzm"), Some(Err(ParseError::Context)));
        assert_eq!(obsolete("-1vz"), Some(Err(ParseError::Context)));
        assert_eq!(
            obsolete("-1vzqvq"), // spell-checker:disable-line
            Some(Err(ParseError::Context))
        );
    }
    #[test]
    fn test_parse_obsolete_no_match() {
        assert_eq!(obsolete("-k"), None);
        assert_eq!(obsolete("asd"), None);
        assert_eq!(obsolete("-cc"), None);
    }
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_obsolete_overflow_x64() {
        assert_eq!(
            obsolete("-10000000000000000000000"),
            Some(Err(ParseError::Overflow))
        );
    }
    #[test]
    #[cfg(target_pointer_width = "32")]
    fn test_parse_obsolete_overflow_x32() {
        assert_eq!(obsolete("-42949672960"), Some(Err(ParseError::Overflow)));
    }
}
