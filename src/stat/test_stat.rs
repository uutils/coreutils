pub use super::*;

#[test]
fn test_scanutil() {
    assert_eq!(Some((-5, 2)), "-5zxc".scan_num::<i32>());
    assert_eq!(Some((51, 2)), "51zxc".scan_num::<u32>());
    assert_eq!(Some((192, 4)), "+192zxc".scan_num::<i32>());
    assert_eq!(None, "z192zxc".scan_num::<i32>());

    assert_eq!(Some(('a', 3)), "141zxc".scan_char(8));
    assert_eq!(Some(('\n', 2)), "12qzxc".scan_char(8));
    assert_eq!(Some(('\r', 1)), "dqzxc".scan_char(16));
    assert_eq!(None, "z2qzxc".scan_char(8));
}

#[cfg(test)]
mod test_generate_tokens {
    use super::*;

    #[test]
    fn test_normal_format() {
        let s = "%10.2ac%-5.w\n";
        let expected = vec![Token::Directive {
                                flag: 0,
                                width: 10,
                                precision: 2,
                                format: 'a',
                            },
                            Token::Char('c'),
                            Token::Directive {
                                flag: F_LEFT,
                                width: 5,
                                precision: 0,
                                format: 'w',
                            },
                            Token::Char('\n')];
        assert_eq!(&expected, &Stater::generate_tokens(s, false).unwrap());
    }

    #[test]
    fn test_printf_format() {
        let s = "%-# 15a\\r\\\"\\\\\\a\\b\\e\\f\\v%+020.-23w\\x12\\167\\132\\112\\n";
        let expected = vec![Token::Directive {
                                flag: F_LEFT | F_ALTER | F_SPACE,
                                width: 15,
                                precision: -1,
                                format: 'a',
                            },
                            Token::Char('\r'),
                            Token::Char('"'),
                            Token::Char('\\'),
                            Token::Char('\x07'),
                            Token::Char('\x08'),
                            Token::Char('\x1B'),
                            Token::Char('\x0C'),
                            Token::Char('\x0B'),
                            Token::Directive {
                                flag: F_SIGN | F_ZERO,
                                width: 20,
                                precision: -1,
                                format: 'w',
                            },
                            Token::Char('\x12'),
                            Token::Char('w'),
                            Token::Char('Z'),
                            Token::Char('J'),
                            Token::Char('\n')];
        assert_eq!(&expected, &Stater::generate_tokens(s, true).unwrap());
    }
}
