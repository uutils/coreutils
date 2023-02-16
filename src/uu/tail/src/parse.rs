//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct ObsoleteArgs {
    pub num: u64,
    pub plus: bool,
    pub lines: bool,
    pub follow: bool,
}

impl Default for ObsoleteArgs {
    fn default() -> Self {
        Self {
            num: 10,
            plus: false,
            lines: true,
            follow: false,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum ParseError {
    OutOfRange,
    Overflow,
    Context,
}
/// Parses obsolete syntax
/// tail -\[NUM\]\[bl\]\[f\] and tail +\[NUM\]\[bcl\]\[f\] // spell-checker:disable-line
pub fn parse_obsolete(src: &str) -> Option<Result<ObsoleteArgs, ParseError>> {
    let mut chars = src.chars();
    let sign = chars.next()?;
    if sign != '+' && sign != '-' {
        return None;
    }

    let numbers: String = chars.clone().take_while(|&c| c.is_ascii_digit()).collect();
    let has_num = !numbers.is_empty();
    let num: u64 = if has_num {
        if let Ok(num) = numbers.parse() {
            num
        } else {
            return Some(Err(ParseError::OutOfRange));
        }
    } else {
        10
    };

    let mut follow = false;
    let mut mode = 'l';
    let mut first_char = true;
    for char in chars.skip_while(|&c| c.is_ascii_digit()) {
        if !has_num && first_char && sign == '-' && (char == 'c' || char == 'f') {
            // special cases: -c, -f should be handled by clap (are ambiguous)
            return None;
        } else if char == 'f' {
            follow = true;
        } else if first_char && (char == 'b' || char == 'c' || char == 'l') {
            mode = char;
        } else if has_num && sign == '-' {
            return Some(Err(ParseError::Context));
        } else {
            return None;
        }
        first_char = false;
    }
    let multiplier = if mode == 'b' { 512 } else { 1 };
    let num = match num.checked_mul(multiplier) {
        Some(n) => n,
        None => return Some(Err(ParseError::Overflow)),
    };

    Some(Ok(ObsoleteArgs {
        num,
        plus: sign == '+',
        lines: mode == 'l',
        follow,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_numbers_obsolete() {
        assert_eq!(
            parse_obsolete("+2c"),
            Some(Ok(ObsoleteArgs {
                num: 2,
                plus: true,
                lines: false,
                follow: false,
            }))
        );
        assert_eq!(
            parse_obsolete("-5"),
            Some(Ok(ObsoleteArgs {
                num: 5,
                plus: false,
                lines: true,
                follow: false,
            }))
        );
        assert_eq!(
            parse_obsolete("+100f"),
            Some(Ok(ObsoleteArgs {
                num: 100,
                plus: true,
                lines: true,
                follow: true,
            }))
        );
        assert_eq!(
            parse_obsolete("-2b"),
            Some(Ok(ObsoleteArgs {
                num: 1024,
                plus: false,
                lines: false,
                follow: false,
            }))
        );
    }
    #[test]
    fn test_parse_errors_obsolete() {
        assert_eq!(parse_obsolete("-5n"), Some(Err(ParseError::Context)));
        assert_eq!(parse_obsolete("-5c5"), Some(Err(ParseError::Context)));
        assert_eq!(parse_obsolete("-1vzc"), Some(Err(ParseError::Context)));
        assert_eq!(parse_obsolete("-5m"), Some(Err(ParseError::Context)));
        assert_eq!(parse_obsolete("-1k"), Some(Err(ParseError::Context)));
        assert_eq!(parse_obsolete("-1mmk"), Some(Err(ParseError::Context)));
        assert_eq!(parse_obsolete("-105kzm"), Some(Err(ParseError::Context)));
        assert_eq!(parse_obsolete("-1vz"), Some(Err(ParseError::Context)));
        assert_eq!(
            parse_obsolete("-1vzqvq"), // spell-checker:disable-line
            Some(Err(ParseError::Context))
        );
    }
    #[test]
    fn test_parse_obsolete_no_match() {
        assert_eq!(parse_obsolete("-k"), None);
        assert_eq!(parse_obsolete("asd"), None);
        assert_eq!(parse_obsolete("-cc"), None);
    }
}
