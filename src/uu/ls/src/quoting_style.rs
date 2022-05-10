use std::char::from_digit;
use std::ffi::OsStr;

// These are characters with special meaning in the shell (e.g. bash).
// The first const contains characters that only have a special meaning when they appear at the beginning of a name.
const SPECIAL_SHELL_CHARS_START: &[char] = &['~', '#'];
const SPECIAL_SHELL_CHARS: &str = "`$&*()|[]{};\\'\"<>?! ";

pub enum QuotingStyle {
    Shell {
        escape: bool,
        always_quote: bool,
        show_control: bool,
    },
    C {
        quotes: Quotes,
    },
    Literal {
        show_control: bool,
    },
}

#[derive(Clone, Copy)]
pub enum Quotes {
    None,
    Single,
    Double,
    // TODO: Locale
}

// This implementation is heavily inspired by the std::char::EscapeDefault implementation
// in the Rust standard library. This custom implementation is needed because the
// characters \a, \b, \e, \f & \v are not recognized by Rust.
struct EscapedChar {
    state: EscapeState,
}

enum EscapeState {
    Done,
    Char(char),
    Backslash(char),
    ForceQuote(char),
    Octal(EscapeOctal),
}

struct EscapeOctal {
    c: char,
    state: EscapeOctalState,
    idx: usize,
}

enum EscapeOctalState {
    Done,
    Backslash,
    Value,
}

impl Iterator for EscapeOctal {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeOctalState::Done => None,
            EscapeOctalState::Backslash => {
                self.state = EscapeOctalState::Value;
                Some('\\')
            }
            EscapeOctalState::Value => {
                let octal_digit = ((self.c as u32) >> (self.idx * 3)) & 0o7;
                if self.idx == 0 {
                    self.state = EscapeOctalState::Done;
                } else {
                    self.idx -= 1;
                }
                Some(from_digit(octal_digit, 8).unwrap())
            }
        }
    }
}

impl EscapeOctal {
    fn from(c: char) -> Self {
        Self {
            c,
            idx: 2,
            state: EscapeOctalState::Backslash,
        }
    }
}

impl EscapedChar {
    fn new_literal(c: char) -> Self {
        Self {
            state: EscapeState::Char(c),
        }
    }

    fn new_c(c: char, quotes: Quotes) -> Self {
        use EscapeState::*;
        let init_state = match c {
            '\x07' => Backslash('a'),
            '\x08' => Backslash('b'),
            '\t' => Backslash('t'),
            '\n' => Backslash('n'),
            '\x0B' => Backslash('v'),
            '\x0C' => Backslash('f'),
            '\r' => Backslash('r'),
            '\\' => Backslash('\\'),
            '\'' => match quotes {
                Quotes::Single => Backslash('\''),
                _ => Char('\''),
            },
            '"' => match quotes {
                Quotes::Double => Backslash('"'),
                _ => Char('"'),
            },
            ' ' => match quotes {
                Quotes::None => Backslash(' '),
                _ => Char(' '),
            },
            _ if c.is_ascii_control() => Octal(EscapeOctal::from(c)),
            _ => Char(c),
        };
        Self { state: init_state }
    }

    fn new_shell(c: char, escape: bool, quotes: Quotes) -> Self {
        use EscapeState::*;
        let init_state = match c {
            _ if !escape && c.is_control() => Char(c),
            '\x07' => Backslash('a'),
            '\x08' => Backslash('b'),
            '\t' => Backslash('t'),
            '\n' => Backslash('n'),
            '\x0B' => Backslash('v'),
            '\x0C' => Backslash('f'),
            '\r' => Backslash('r'),
            '\x00'..='\x1F' | '\x7F' => Octal(EscapeOctal::from(c)),
            '\'' => match quotes {
                Quotes::Single => Backslash('\''),
                _ => Char('\''),
            },
            _ if SPECIAL_SHELL_CHARS.contains(c) => ForceQuote(c),
            _ => Char(c),
        };
        Self { state: init_state }
    }

    fn hide_control(self) -> Self {
        match self.state {
            EscapeState::Char(c) if c.is_control() => Self {
                state: EscapeState::Char('?'),
            },
            _ => self,
        }
    }
}

impl Iterator for EscapedChar {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeState::Backslash(c) => {
                self.state = EscapeState::Char(c);
                Some('\\')
            }
            EscapeState::Char(c) | EscapeState::ForceQuote(c) => {
                self.state = EscapeState::Done;
                Some(c)
            }
            EscapeState::Done => None,
            EscapeState::Octal(ref mut iter) => iter.next(),
        }
    }
}

fn shell_without_escape(name: &str, quotes: Quotes, show_control_chars: bool) -> (String, bool) {
    let mut must_quote = false;
    let mut escaped_str = String::with_capacity(name.len());

    for c in name.chars() {
        let escaped = {
            let ec = EscapedChar::new_shell(c, false, quotes);
            if show_control_chars {
                ec
            } else {
                ec.hide_control()
            }
        };

        match escaped.state {
            EscapeState::Backslash('\'') => escaped_str.push_str("'\\''"),
            EscapeState::ForceQuote(x) => {
                must_quote = true;
                escaped_str.push(x);
            }
            _ => {
                for char in escaped {
                    escaped_str.push(char);
                }
            }
        }
    }

    must_quote = must_quote || name.starts_with(SPECIAL_SHELL_CHARS_START);
    (escaped_str, must_quote)
}

fn shell_with_escape(name: &str, quotes: Quotes) -> (String, bool) {
    // We need to keep track of whether we are in a dollar expression
    // because e.g. \b\n is escaped as $'\b\n' and not like $'b'$'n'
    let mut in_dollar = false;
    let mut must_quote = false;
    let mut escaped_str = String::with_capacity(name.len());

    for c in name.chars() {
        let escaped = EscapedChar::new_shell(c, true, quotes);
        match escaped.state {
            EscapeState::Char(x) => {
                if in_dollar {
                    escaped_str.push_str("''");
                    in_dollar = false;
                }
                escaped_str.push(x);
            }
            EscapeState::ForceQuote(x) => {
                if in_dollar {
                    escaped_str.push_str("''");
                    in_dollar = false;
                }
                must_quote = true;
                escaped_str.push(x);
            }
            // Single quotes are not put in dollar expressions, but are escaped
            // if the string also contains double quotes. In that case, they must
            // be handled separately.
            EscapeState::Backslash('\'') => {
                must_quote = true;
                in_dollar = false;
                escaped_str.push_str("'\\''");
            }
            _ => {
                if !in_dollar {
                    escaped_str.push_str("'$'");
                    in_dollar = true;
                }
                must_quote = true;
                for char in escaped {
                    escaped_str.push(char);
                }
            }
        }
    }
    must_quote = must_quote || name.starts_with(SPECIAL_SHELL_CHARS_START);
    (escaped_str, must_quote)
}

pub(super) fn escape_name(name: &OsStr, style: &QuotingStyle) -> String {
    match style {
        QuotingStyle::Literal { show_control } => {
            if !show_control {
                name.to_string_lossy()
                    .chars()
                    .flat_map(|c| EscapedChar::new_literal(c).hide_control())
                    .collect()
            } else {
                name.to_string_lossy().into_owned()
            }
        }
        QuotingStyle::C { quotes } => {
            let escaped_str: String = name
                .to_string_lossy()
                .chars()
                .flat_map(|c| EscapedChar::new_c(c, *quotes))
                .collect();

            match quotes {
                Quotes::Single => format!("'{}'", escaped_str),
                Quotes::Double => format!("\"{}\"", escaped_str),
                _ => escaped_str,
            }
        }
        QuotingStyle::Shell {
            escape,
            always_quote,
            show_control,
        } => {
            let name = name.to_string_lossy();
            let (quotes, must_quote) = if name.contains(&['"', '`', '$', '\\'][..]) {
                (Quotes::Single, true)
            } else if name.contains('\'') {
                (Quotes::Double, true)
            } else if *always_quote {
                (Quotes::Single, true)
            } else {
                (Quotes::Single, false)
            };

            let (escaped_str, contains_quote_chars) = if *escape {
                shell_with_escape(&name, quotes)
            } else {
                shell_without_escape(&name, quotes, *show_control)
            };

            match (must_quote | contains_quote_chars, quotes) {
                (true, Quotes::Single) => format!("'{}'", escaped_str),
                (true, Quotes::Double) => format!("\"{}\"", escaped_str),
                _ => escaped_str,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // spell-checker:ignore (tests/words) one\'two one'two

    use crate::quoting_style::{escape_name, Quotes, QuotingStyle};
    fn get_style(s: &str) -> QuotingStyle {
        match s {
            "literal" => QuotingStyle::Literal {
                show_control: false,
            },
            "literal-show" => QuotingStyle::Literal { show_control: true },
            "escape" => QuotingStyle::C {
                quotes: Quotes::None,
            },
            "c" => QuotingStyle::C {
                quotes: Quotes::Double,
            },
            "shell" => QuotingStyle::Shell {
                escape: false,
                always_quote: false,
                show_control: false,
            },
            "shell-show" => QuotingStyle::Shell {
                escape: false,
                always_quote: false,
                show_control: true,
            },
            "shell-always" => QuotingStyle::Shell {
                escape: false,
                always_quote: true,
                show_control: false,
            },
            "shell-always-show" => QuotingStyle::Shell {
                escape: false,
                always_quote: true,
                show_control: true,
            },
            "shell-escape" => QuotingStyle::Shell {
                escape: true,
                always_quote: false,
                show_control: false,
            },
            "shell-escape-always" => QuotingStyle::Shell {
                escape: true,
                always_quote: true,
                show_control: false,
            },
            _ => panic!("Invalid name!"),
        }
    }

    fn check_names(name: &str, map: &[(&str, &str)]) {
        assert_eq!(
            map.iter()
                .map(|(_, style)| escape_name(name.as_ref(), &get_style(style)))
                .collect::<Vec<String>>(),
            map.iter()
                .map(|(correct, _)| correct.to_string())
                .collect::<Vec<String>>()
        );
    }

    #[test]
    fn test_simple_names() {
        check_names(
            "one_two",
            &[
                ("one_two", "literal"),
                ("one_two", "literal-show"),
                ("one_two", "escape"),
                ("\"one_two\"", "c"),
                ("one_two", "shell"),
                ("one_two", "shell-show"),
                ("\'one_two\'", "shell-always"),
                ("\'one_two\'", "shell-always-show"),
                ("one_two", "shell-escape"),
                ("\'one_two\'", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_spaces() {
        check_names(
            "one two",
            &[
                ("one two", "literal"),
                ("one two", "literal-show"),
                ("one\\ two", "escape"),
                ("\"one two\"", "c"),
                ("\'one two\'", "shell"),
                ("\'one two\'", "shell-show"),
                ("\'one two\'", "shell-always"),
                ("\'one two\'", "shell-always-show"),
                ("\'one two\'", "shell-escape"),
                ("\'one two\'", "shell-escape-always"),
            ],
        );

        check_names(
            " one",
            &[
                (" one", "literal"),
                (" one", "literal-show"),
                ("\\ one", "escape"),
                ("\" one\"", "c"),
                ("' one'", "shell"),
                ("' one'", "shell-show"),
                ("' one'", "shell-always"),
                ("' one'", "shell-always-show"),
                ("' one'", "shell-escape"),
                ("' one'", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_quotes() {
        // One double quote
        check_names(
            "one\"two",
            &[
                ("one\"two", "literal"),
                ("one\"two", "literal-show"),
                ("one\"two", "escape"),
                ("\"one\\\"two\"", "c"),
                ("'one\"two'", "shell"),
                ("'one\"two'", "shell-show"),
                ("'one\"two'", "shell-always"),
                ("'one\"two'", "shell-always-show"),
                ("'one\"two'", "shell-escape"),
                ("'one\"two'", "shell-escape-always"),
            ],
        );

        // One single quote
        check_names(
            "one\'two",
            &[
                ("one'two", "literal"),
                ("one'two", "literal-show"),
                ("one'two", "escape"),
                ("\"one'two\"", "c"),
                ("\"one'two\"", "shell"),
                ("\"one'two\"", "shell-show"),
                ("\"one'two\"", "shell-always"),
                ("\"one'two\"", "shell-always-show"),
                ("\"one'two\"", "shell-escape"),
                ("\"one'two\"", "shell-escape-always"),
            ],
        );

        // One single quote and one double quote
        check_names(
            "one'two\"three",
            &[
                ("one'two\"three", "literal"),
                ("one'two\"three", "literal-show"),
                ("one'two\"three", "escape"),
                ("\"one'two\\\"three\"", "c"),
                ("'one'\\''two\"three'", "shell"),
                ("'one'\\''two\"three'", "shell-show"),
                ("'one'\\''two\"three'", "shell-always"),
                ("'one'\\''two\"three'", "shell-always-show"),
                ("'one'\\''two\"three'", "shell-escape"),
                ("'one'\\''two\"three'", "shell-escape-always"),
            ],
        );

        // Consecutive quotes
        check_names(
            "one''two\"\"three",
            &[
                ("one''two\"\"three", "literal"),
                ("one''two\"\"three", "literal-show"),
                ("one''two\"\"three", "escape"),
                ("\"one''two\\\"\\\"three\"", "c"),
                ("'one'\\'''\\''two\"\"three'", "shell"),
                ("'one'\\'''\\''two\"\"three'", "shell-show"),
                ("'one'\\'''\\''two\"\"three'", "shell-always"),
                ("'one'\\'''\\''two\"\"three'", "shell-always-show"),
                ("'one'\\'''\\''two\"\"three'", "shell-escape"),
                ("'one'\\'''\\''two\"\"three'", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_control_chars() {
        // A simple newline
        check_names(
            "one\ntwo",
            &[
                ("one?two", "literal"),
                ("one\ntwo", "literal-show"),
                ("one\\ntwo", "escape"),
                ("\"one\\ntwo\"", "c"),
                ("one?two", "shell"),
                ("one\ntwo", "shell-show"),
                ("'one?two'", "shell-always"),
                ("'one\ntwo'", "shell-always-show"),
                ("'one'$'\\n''two'", "shell-escape"),
                ("'one'$'\\n''two'", "shell-escape-always"),
            ],
        );

        // A control character followed by a special shell character
        check_names(
            "one\n&two",
            &[
                ("one?&two", "literal"),
                ("one\n&two", "literal-show"),
                ("one\\n&two", "escape"),
                ("\"one\\n&two\"", "c"),
                ("'one?&two'", "shell"),
                ("'one\n&two'", "shell-show"),
                ("'one?&two'", "shell-always"),
                ("'one\n&two'", "shell-always-show"),
                ("'one'$'\\n''&two'", "shell-escape"),
                ("'one'$'\\n''&two'", "shell-escape-always"),
            ],
        );

        // The first 16 control characters. NUL is also included, even though it is of
        // no importance for file names.
        check_names(
            "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
            &[
                ("????????????????", "literal"),
                (
                    "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
                    "literal-show",
                ),
                (
                    "\\000\\001\\002\\003\\004\\005\\006\\a\\b\\t\\n\\v\\f\\r\\016\\017",
                    "escape",
                ),
                (
                    "\"\\000\\001\\002\\003\\004\\005\\006\\a\\b\\t\\n\\v\\f\\r\\016\\017\"",
                    "c",
                ),
                ("????????????????", "shell"),
                (
                    "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F",
                    "shell-show",
                ),
                ("'????????????????'", "shell-always"),
                (
                    "'\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F'",
                    "shell-always-show",
                ),
                (
                    "''$'\\000\\001\\002\\003\\004\\005\\006\\a\\b\\t\\n\\v\\f\\r\\016\\017'",
                    "shell-escape",
                ),
                (
                    "''$'\\000\\001\\002\\003\\004\\005\\006\\a\\b\\t\\n\\v\\f\\r\\016\\017'",
                    "shell-escape-always",
                ),
            ],
        );

        // The last 16 control characters.
        check_names(
            "\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
            &[
                ("????????????????", "literal"),
                (
                    "\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
                    "literal-show",
                ),
                (
                    "\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037",
                    "escape",
                ),
                (
                    "\"\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037\"",
                    "c",
                ),
                ("????????????????", "shell"),
                (
                    "\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F",
                    "shell-show",
                ),
                ("'????????????????'", "shell-always"),
                (
                    "'\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F'",
                    "shell-always-show",
                ),
                (
                    "''$'\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037'",
                    "shell-escape",
                ),
                (
                    "''$'\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037'",
                    "shell-escape-always",
                ),
            ],
        );

        // DEL
        check_names(
            "\x7F",
            &[
                ("?", "literal"),
                ("\x7F", "literal-show"),
                ("\\177", "escape"),
                ("\"\\177\"", "c"),
                ("?", "shell"),
                ("\x7F", "shell-show"),
                ("'?'", "shell-always"),
                ("'\x7F'", "shell-always-show"),
                ("''$'\\177'", "shell-escape"),
                ("''$'\\177'", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_question_mark() {
        // A question mark must force quotes in shell and shell-always, unless
        // it is in place of a control character (that case is already covered
        // in other tests)
        check_names(
            "one?two",
            &[
                ("one?two", "literal"),
                ("one?two", "literal-show"),
                ("one?two", "escape"),
                ("\"one?two\"", "c"),
                ("'one?two'", "shell"),
                ("'one?two'", "shell-show"),
                ("'one?two'", "shell-always"),
                ("'one?two'", "shell-always-show"),
                ("'one?two'", "shell-escape"),
                ("'one?two'", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_backslash() {
        // Escaped in C-style, but not in Shell-style escaping
        check_names(
            "one\\two",
            &[
                ("one\\two", "literal"),
                ("one\\two", "literal-show"),
                ("one\\\\two", "escape"),
                ("\"one\\\\two\"", "c"),
                ("'one\\two'", "shell"),
                ("\'one\\two\'", "shell-always"),
                ("'one\\two'", "shell-escape"),
                ("'one\\two'", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_tilde_and_hash() {
        check_names("~", &[("'~'", "shell"), ("'~'", "shell-escape")]);
        check_names(
            "~name",
            &[("'~name'", "shell"), ("'~name'", "shell-escape")],
        );
        check_names(
            "some~name",
            &[("some~name", "shell"), ("some~name", "shell-escape")],
        );
        check_names("name~", &[("name~", "shell"), ("name~", "shell-escape")]);

        check_names("#", &[("'#'", "shell"), ("'#'", "shell-escape")]);
        check_names(
            "#name",
            &[("'#name'", "shell"), ("'#name'", "shell-escape")],
        );
        check_names(
            "some#name",
            &[("some#name", "shell"), ("some#name", "shell-escape")],
        );
        check_names("name#", &[("name#", "shell"), ("name#", "shell-escape")]);
    }

    #[test]
    fn test_special_chars_in_double_quotes() {
        check_names(
            "can'$t",
            &[
                ("'can'\\''$t'", "shell"),
                ("'can'\\''$t'", "shell-always"),
                ("'can'\\''$t'", "shell-escape"),
                ("'can'\\''$t'", "shell-escape-always"),
            ],
        );

        check_names(
            "can'`t",
            &[
                ("'can'\\''`t'", "shell"),
                ("'can'\\''`t'", "shell-always"),
                ("'can'\\''`t'", "shell-escape"),
                ("'can'\\''`t'", "shell-escape-always"),
            ],
        );

        check_names(
            "can'\\t",
            &[
                ("'can'\\''\\t'", "shell"),
                ("'can'\\''\\t'", "shell-always"),
                ("'can'\\''\\t'", "shell-escape"),
                ("'can'\\''\\t'", "shell-escape-always"),
            ],
        );
    }
}
