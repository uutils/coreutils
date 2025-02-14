// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions for escaping names according to different quoting styles.

use std::char::from_digit;
use std::ffi::{OsStr, OsString};
use std::fmt;

// These are characters with special meaning in the shell (e.g. bash).
// The first const contains characters that only have a special meaning when they appear at the beginning of a name.
const SPECIAL_SHELL_CHARS_START: &[u8] = b"~#";
// PR#6559 : Remove `]{}` from special shell chars.
const SPECIAL_SHELL_CHARS: &str = "`$&*()|[;\\'\"<>?! ";

/// The quoting style to use when escaping a name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuotingStyle {
    /// Escape the name as a shell string.
    /// Used in, e.g., `ls --quoting-style=shell`.
    Shell {
        /// Whether to escape characters in the name.
        /// True in, e.g., `ls --quoting-style=shell-escape`.
        escape: bool,

        /// Whether to always quote the name.
        always_quote: bool,

        /// Whether to show control and non-unicode characters, or replace them with `?`.
        show_control: bool,
    },

    /// Escape the name as a C string.
    /// Used in, e.g., `ls --quote-name`.
    C {
        /// The type of quotes to use.
        quotes: Quotes,
    },

    /// Do not escape the string.
    /// Used in, e.g., `ls --literal`.
    Literal {
        /// Whether to show control and non-unicode characters, or replace them with `?`.
        show_control: bool,
    },
}

/// The type of quotes to use when escaping a name as a C string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quotes {
    /// Do not use quotes.
    None,

    /// Use single quotes.
    Single,

    /// Use double quotes.
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

/// Bytes we need to present as escaped octal, in the form of `\nnn` per byte.
/// Only supports characters up to 2 bytes long in UTF-8.
struct EscapeOctal {
    c: [u8; 2],
    state: EscapeOctalState,
    idx: u8,
}

enum EscapeOctalState {
    Done,
    FirstBackslash,
    FirstValue,
    LastBackslash,
    LastValue,
}

fn byte_to_octal_digit(byte: u8, idx: u8) -> u8 {
    (byte >> (idx * 3)) & 0o7
}

impl Iterator for EscapeOctal {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeOctalState::Done => None,
            EscapeOctalState::FirstBackslash => {
                self.state = EscapeOctalState::FirstValue;
                Some('\\')
            }
            EscapeOctalState::LastBackslash => {
                self.state = EscapeOctalState::LastValue;
                Some('\\')
            }
            EscapeOctalState::FirstValue => {
                let octal_digit = byte_to_octal_digit(self.c[0], self.idx);
                if self.idx == 0 {
                    self.state = EscapeOctalState::LastBackslash;
                    self.idx = 2;
                } else {
                    self.idx -= 1;
                }
                Some(from_digit(octal_digit.into(), 8).unwrap())
            }
            EscapeOctalState::LastValue => {
                let octal_digit = byte_to_octal_digit(self.c[1], self.idx);
                if self.idx == 0 {
                    self.state = EscapeOctalState::Done;
                } else {
                    self.idx -= 1;
                }
                Some(from_digit(octal_digit.into(), 8).unwrap())
            }
        }
    }
}

impl EscapeOctal {
    fn from_char(c: char) -> Self {
        if c.len_utf8() == 1 {
            return Self::from_byte(c as u8);
        }

        let mut buf = [0; 2];
        let _s = c.encode_utf8(&mut buf);
        Self {
            c: buf,
            idx: 2,
            state: EscapeOctalState::FirstBackslash,
        }
    }

    fn from_byte(b: u8) -> Self {
        Self {
            c: [0, b],
            idx: 2,
            state: EscapeOctalState::LastBackslash,
        }
    }
}

impl EscapedChar {
    fn new_literal(c: char) -> Self {
        Self {
            state: EscapeState::Char(c),
        }
    }

    fn new_octal(b: u8) -> Self {
        Self {
            state: EscapeState::Octal(EscapeOctal::from_byte(b)),
        }
    }

    fn new_c(c: char, quotes: Quotes, dirname: bool) -> Self {
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
            ' ' if !dirname => match quotes {
                Quotes::None => Backslash(' '),
                _ => Char(' '),
            },
            ':' if dirname => Backslash(':'),
            _ if c.is_control() => Octal(EscapeOctal::from_char(c)),
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
            '\'' => match quotes {
                Quotes::Single => Backslash('\''),
                _ => Char('\''),
            },
            _ if c.is_control() => Octal(EscapeOctal::from_char(c)),
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

/// Check whether `bytes` starts with any byte in `pattern`.
fn bytes_start_with(bytes: &[u8], pattern: &[u8]) -> bool {
    !bytes.is_empty() && pattern.contains(&bytes[0])
}

fn shell_without_escape(name: &[u8], quotes: Quotes, show_control_chars: bool) -> (Vec<u8>, bool) {
    let mut must_quote = false;
    let mut escaped_str = Vec::with_capacity(name.len());
    let mut utf8_buf = vec![0; 4];

    for s in name.utf8_chunks() {
        for c in s.valid().chars() {
            let escaped = {
                let ec = EscapedChar::new_shell(c, false, quotes);
                if show_control_chars {
                    ec
                } else {
                    ec.hide_control()
                }
            };

            match escaped.state {
                EscapeState::Backslash('\'') => escaped_str.extend_from_slice(b"'\\''"),
                EscapeState::ForceQuote(x) => {
                    must_quote = true;
                    escaped_str.extend_from_slice(x.encode_utf8(&mut utf8_buf).as_bytes());
                }
                _ => {
                    for c in escaped {
                        escaped_str.extend_from_slice(c.encode_utf8(&mut utf8_buf).as_bytes());
                    }
                }
            }
        }

        if show_control_chars {
            escaped_str.extend_from_slice(s.invalid());
        } else {
            escaped_str.resize(escaped_str.len() + s.invalid().len(), b'?');
        }
    }

    must_quote = must_quote || bytes_start_with(name, SPECIAL_SHELL_CHARS_START);
    (escaped_str, must_quote)
}

fn shell_with_escape(name: &[u8], quotes: Quotes) -> (Vec<u8>, bool) {
    // We need to keep track of whether we are in a dollar expression
    // because e.g. \b\n is escaped as $'\b\n' and not like $'b'$'n'
    let mut in_dollar = false;
    let mut must_quote = false;
    let mut escaped_str = String::with_capacity(name.len());

    for s in name.utf8_chunks() {
        for c in s.valid().chars() {
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
        if !s.invalid().is_empty() {
            if !in_dollar {
                escaped_str.push_str("'$'");
                in_dollar = true;
            }
            must_quote = true;
            let escaped_bytes: String = s
                .invalid()
                .iter()
                .flat_map(|b| EscapedChar::new_octal(*b))
                .collect();
            escaped_str.push_str(&escaped_bytes);
        }
    }
    must_quote = must_quote || bytes_start_with(name, SPECIAL_SHELL_CHARS_START);
    (escaped_str.into(), must_quote)
}

/// Return a set of characters that implies quoting of the word in
/// shell-quoting mode.
fn shell_escaped_char_set(is_dirname: bool) -> &'static [u8] {
    const ESCAPED_CHARS: &[u8] = b":\"`$\\^\n\t\r=";
    // the ':' colon character only induce quoting in the
    // context of ls displaying a directory name before listing its content.
    // (e.g. with the recursive flag -R)
    let start_index = if is_dirname { 0 } else { 1 };
    &ESCAPED_CHARS[start_index..]
}

/// Escape a name according to the given quoting style.
///
/// This inner function provides an additional flag `dirname` which
/// is meant for ls' directory name display.
fn escape_name_inner(name: &[u8], style: &QuotingStyle, dirname: bool) -> Vec<u8> {
    match style {
        QuotingStyle::Literal { show_control } => {
            if *show_control {
                name.to_owned()
            } else {
                name.utf8_chunks()
                    .map(|s| {
                        let valid: String = s
                            .valid()
                            .chars()
                            .flat_map(|c| EscapedChar::new_literal(c).hide_control())
                            .collect();
                        let invalid = "?".repeat(s.invalid().len());
                        valid + &invalid
                    })
                    .collect::<String>()
                    .into()
            }
        }
        QuotingStyle::C { quotes } => {
            let escaped_str: String = name
                .utf8_chunks()
                .flat_map(|s| {
                    let valid = s
                        .valid()
                        .chars()
                        .flat_map(|c| EscapedChar::new_c(c, *quotes, dirname));
                    let invalid = s.invalid().iter().flat_map(|b| EscapedChar::new_octal(*b));
                    valid.chain(invalid)
                })
                .collect::<String>();

            match quotes {
                Quotes::Single => format!("'{escaped_str}'"),
                Quotes::Double => format!("\"{escaped_str}\""),
                Quotes::None => escaped_str,
            }
            .into()
        }
        QuotingStyle::Shell {
            escape,
            always_quote,
            show_control,
        } => {
            let (quotes, must_quote) = if name
                .iter()
                .any(|c| shell_escaped_char_set(dirname).contains(c))
            {
                (Quotes::Single, true)
            } else if name.contains(&b'\'') {
                (Quotes::Double, true)
            } else if *always_quote {
                (Quotes::Single, true)
            } else {
                (Quotes::Single, false)
            };

            let (escaped_str, contains_quote_chars) = if *escape {
                shell_with_escape(name, quotes)
            } else {
                shell_without_escape(name, quotes, *show_control)
            };

            if must_quote | contains_quote_chars && quotes != Quotes::None {
                let mut quoted_str = Vec::<u8>::with_capacity(escaped_str.len() + 2);
                let quote = if quotes == Quotes::Single {
                    b'\''
                } else {
                    b'"'
                };
                quoted_str.push(quote);
                quoted_str.extend(escaped_str);
                quoted_str.push(quote);
                quoted_str
            } else {
                escaped_str
            }
        }
    }
}

/// Escape a filename with respect to the given style.
pub fn escape_name(name: &OsStr, style: &QuotingStyle) -> OsString {
    let name = crate::os_str_as_bytes_lossy(name);
    crate::os_string_from_vec(escape_name_inner(&name, style, false))
        .expect("all byte sequences should be valid for platform, or already replaced in name")
}

/// Escape a directory name with respect to the given style.
/// This is mainly meant to be used for ls' directory name printing and is not
/// likely to be used elsewhere.
pub fn escape_dir_name(dir_name: &OsStr, style: &QuotingStyle) -> OsString {
    let name = crate::os_str_as_bytes_lossy(dir_name);
    crate::os_string_from_vec(escape_name_inner(&name, style, true))
        .expect("all byte sequences should be valid for platform, or already replaced in name")
}

impl fmt::Display for QuotingStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Shell {
                escape,
                always_quote,
                show_control,
            } => {
                let mut style = "shell".to_string();
                if escape {
                    style.push_str("-escape");
                }
                if always_quote {
                    style.push_str("-always-quote");
                }
                if show_control {
                    style.push_str("-show-control");
                }
                f.write_str(&style)
            }
            Self::C { .. } => f.write_str("C"),
            Self::Literal { .. } => f.write_str("literal"),
        }
    }
}

impl fmt::Display for Quotes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::None => f.write_str("None"),
            Self::Single => f.write_str("Single"),
            Self::Double => f.write_str("Double"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::quoting_style::{escape_name_inner, Quotes, QuotingStyle};

    // spell-checker:ignore (tests/words) one\'two one'two

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

    fn check_names_inner<T>(name: &[u8], map: &[(T, &str)]) -> Vec<Vec<u8>> {
        map.iter()
            .map(|(_, style)| escape_name_inner(name, &get_style(style), false))
            .collect()
    }

    fn check_names(name: &str, map: &[(&str, &str)]) {
        assert_eq!(
            map.iter()
                .map(|(correct, _)| *correct)
                .collect::<Vec<&str>>(),
            check_names_inner(name.as_bytes(), map)
                .iter()
                .map(|bytes| std::str::from_utf8(bytes)
                    .expect("valid str goes in, valid str comes out"))
                .collect::<Vec<&str>>()
        );
    }

    fn check_names_raw(name: &[u8], map: &[(&[u8], &str)]) {
        assert_eq!(
            map.iter()
                .map(|(correct, _)| *correct)
                .collect::<Vec<&[u8]>>(),
            check_names_inner(name, map)
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
                ("'one_two'", "shell-always"),
                ("'one_two'", "shell-always-show"),
                ("one_two", "shell-escape"),
                ("'one_two'", "shell-escape-always"),
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
                ("'one two'", "shell"),
                ("'one two'", "shell-show"),
                ("'one two'", "shell-always"),
                ("'one two'", "shell-always-show"),
                ("'one two'", "shell-escape"),
                ("'one two'", "shell-escape-always"),
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
            "one'two",
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
                ("'one?two'", "shell"),
                ("'one\ntwo'", "shell-show"),
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

        // The first 16 ASCII control characters. NUL is also included, even though it is of
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
                ("'????????????????'", "shell"),
                (
                    "'\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F'",
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

        // The last 16 ASCII control characters.
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

        // The first 16 Unicode control characters.
        let test_str = std::str::from_utf8(b"\xC2\x80\xC2\x81\xC2\x82\xC2\x83\xC2\x84\xC2\x85\xC2\x86\xC2\x87\xC2\x88\xC2\x89\xC2\x8A\xC2\x8B\xC2\x8C\xC2\x8D\xC2\x8E\xC2\x8F").unwrap();
        check_names(
            test_str,
            &[
                ("????????????????", "literal"),
                (test_str, "literal-show"),
                ("\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217", "escape"),
                ("\"\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217\"", "c"),
                ("????????????????", "shell"),
                (test_str, "shell-show"),
                ("'????????????????'", "shell-always"),
                (&format!("'{}'", test_str), "shell-always-show"),
                ("''$'\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217'", "shell-escape"),
                ("''$'\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217'", "shell-escape-always"),
            ],
        );

        // The last 16 Unicode control characters.
        let test_str = std::str::from_utf8(b"\xC2\x90\xC2\x91\xC2\x92\xC2\x93\xC2\x94\xC2\x95\xC2\x96\xC2\x97\xC2\x98\xC2\x99\xC2\x9A\xC2\x9B\xC2\x9C\xC2\x9D\xC2\x9E\xC2\x9F").unwrap();
        check_names(
            test_str,
            &[
                ("????????????????", "literal"),
                (test_str, "literal-show"),
                ("\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237", "escape"),
                ("\"\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237\"", "c"),
                ("????????????????", "shell"),
                (test_str, "shell-show"),
                ("'????????????????'", "shell-always"),
                (&format!("'{}'", test_str), "shell-always-show"),
                ("''$'\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237'", "shell-escape"),
                ("''$'\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237'", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_non_unicode_bytes() {
        let ascii = b'_';
        let continuation = b'\xA7';
        let first2byte = b'\xC2';
        let first3byte = b'\xE0';
        let first4byte = b'\xF0';
        let invalid = b'\xC0';

        // a single byte value invalid outside of additional context in UTF-8
        check_names_raw(
            &[continuation],
            &[
                (b"?", "literal"),
                (b"\xA7", "literal-show"),
                (b"\\247", "escape"),
                (b"\"\\247\"", "c"),
                (b"?", "shell"),
                (b"\xA7", "shell-show"),
                (b"'?'", "shell-always"),
                (b"'\xA7'", "shell-always-show"),
                (b"''$'\\247'", "shell-escape"),
                (b"''$'\\247'", "shell-escape-always"),
            ],
        );

        // ...but the byte becomes valid with appropriate context
        // (this is just the § character in UTF-8, written as bytes)
        check_names_raw(
            &[first2byte, continuation],
            &[
                (b"\xC2\xA7", "literal"),
                (b"\xC2\xA7", "literal-show"),
                (b"\xC2\xA7", "escape"),
                (b"\"\xC2\xA7\"", "c"),
                (b"\xC2\xA7", "shell"),
                (b"\xC2\xA7", "shell-show"),
                (b"'\xC2\xA7'", "shell-always"),
                (b"'\xC2\xA7'", "shell-always-show"),
                (b"\xC2\xA7", "shell-escape"),
                (b"'\xC2\xA7'", "shell-escape-always"),
            ],
        );

        // mixed with valid characters
        check_names_raw(
            &[continuation, ascii],
            &[
                (b"?_", "literal"),
                (b"\xA7_", "literal-show"),
                (b"\\247_", "escape"),
                (b"\"\\247_\"", "c"),
                (b"?_", "shell"),
                (b"\xA7_", "shell-show"),
                (b"'?_'", "shell-always"),
                (b"'\xA7_'", "shell-always-show"),
                (b"''$'\\247''_'", "shell-escape"),
                (b"''$'\\247''_'", "shell-escape-always"),
            ],
        );
        check_names_raw(
            &[ascii, continuation],
            &[
                (b"_?", "literal"),
                (b"_\xA7", "literal-show"),
                (b"_\\247", "escape"),
                (b"\"_\\247\"", "c"),
                (b"_?", "shell"),
                (b"_\xA7", "shell-show"),
                (b"'_?'", "shell-always"),
                (b"'_\xA7'", "shell-always-show"),
                (b"'_'$'\\247'", "shell-escape"),
                (b"'_'$'\\247'", "shell-escape-always"),
            ],
        );
        check_names_raw(
            &[ascii, continuation, ascii],
            &[
                (b"_?_", "literal"),
                (b"_\xA7_", "literal-show"),
                (b"_\\247_", "escape"),
                (b"\"_\\247_\"", "c"),
                (b"_?_", "shell"),
                (b"_\xA7_", "shell-show"),
                (b"'_?_'", "shell-always"),
                (b"'_\xA7_'", "shell-always-show"),
                (b"'_'$'\\247''_'", "shell-escape"),
                (b"'_'$'\\247''_'", "shell-escape-always"),
            ],
        );
        check_names_raw(
            &[continuation, ascii, continuation],
            &[
                (b"?_?", "literal"),
                (b"\xA7_\xA7", "literal-show"),
                (b"\\247_\\247", "escape"),
                (b"\"\\247_\\247\"", "c"),
                (b"?_?", "shell"),
                (b"\xA7_\xA7", "shell-show"),
                (b"'?_?'", "shell-always"),
                (b"'\xA7_\xA7'", "shell-always-show"),
                (b"''$'\\247''_'$'\\247'", "shell-escape"),
                (b"''$'\\247''_'$'\\247'", "shell-escape-always"),
            ],
        );

        // contiguous invalid bytes
        check_names_raw(
            &[
                ascii,
                invalid,
                ascii,
                continuation,
                continuation,
                ascii,
                continuation,
                continuation,
                continuation,
                ascii,
                continuation,
                continuation,
                continuation,
                continuation,
                ascii,
            ],
            &[
                (b"_?_??_???_????_", "literal"),
                (
                    b"_\xC0_\xA7\xA7_\xA7\xA7\xA7_\xA7\xA7\xA7\xA7_",
                    "literal-show",
                ),
                (
                    b"_\\300_\\247\\247_\\247\\247\\247_\\247\\247\\247\\247_",
                    "escape",
                ),
                (
                    b"\"_\\300_\\247\\247_\\247\\247\\247_\\247\\247\\247\\247_\"",
                    "c",
                ),
                (b"_?_??_???_????_", "shell"),
                (
                    b"_\xC0_\xA7\xA7_\xA7\xA7\xA7_\xA7\xA7\xA7\xA7_",
                    "shell-show",
                ),
                (b"'_?_??_???_????_'", "shell-always"),
                (
                    b"'_\xC0_\xA7\xA7_\xA7\xA7\xA7_\xA7\xA7\xA7\xA7_'",
                    "shell-always-show",
                ),
                (
                    b"'_'$'\\300''_'$'\\247\\247''_'$'\\247\\247\\247''_'$'\\247\\247\\247\\247''_'",
                    "shell-escape",
                ),
                (
                    b"'_'$'\\300''_'$'\\247\\247''_'$'\\247\\247\\247''_'$'\\247\\247\\247\\247''_'",
                    "shell-escape-always",
                ),
            ],
        );

        // invalid multi-byte sequences that start valid
        check_names_raw(
            &[first2byte, ascii],
            &[
                (b"?_", "literal"),
                (b"\xC2_", "literal-show"),
                (b"\\302_", "escape"),
                (b"\"\\302_\"", "c"),
                (b"?_", "shell"),
                (b"\xC2_", "shell-show"),
                (b"'?_'", "shell-always"),
                (b"'\xC2_'", "shell-always-show"),
                (b"''$'\\302''_'", "shell-escape"),
                (b"''$'\\302''_'", "shell-escape-always"),
            ],
        );
        check_names_raw(
            &[first2byte, first2byte, continuation],
            &[
                (b"?\xC2\xA7", "literal"),
                (b"\xC2\xC2\xA7", "literal-show"),
                (b"\\302\xC2\xA7", "escape"),
                (b"\"\\302\xC2\xA7\"", "c"),
                (b"?\xC2\xA7", "shell"),
                (b"\xC2\xC2\xA7", "shell-show"),
                (b"'?\xC2\xA7'", "shell-always"),
                (b"'\xC2\xC2\xA7'", "shell-always-show"),
                (b"''$'\\302''\xC2\xA7'", "shell-escape"),
                (b"''$'\\302''\xC2\xA7'", "shell-escape-always"),
            ],
        );
        check_names_raw(
            &[first3byte, continuation, ascii],
            &[
                (b"??_", "literal"),
                (b"\xE0\xA7_", "literal-show"),
                (b"\\340\\247_", "escape"),
                (b"\"\\340\\247_\"", "c"),
                (b"??_", "shell"),
                (b"\xE0\xA7_", "shell-show"),
                (b"'??_'", "shell-always"),
                (b"'\xE0\xA7_'", "shell-always-show"),
                (b"''$'\\340\\247''_'", "shell-escape"),
                (b"''$'\\340\\247''_'", "shell-escape-always"),
            ],
        );
        check_names_raw(
            &[first4byte, continuation, continuation, ascii],
            &[
                (b"???_", "literal"),
                (b"\xF0\xA7\xA7_", "literal-show"),
                (b"\\360\\247\\247_", "escape"),
                (b"\"\\360\\247\\247_\"", "c"),
                (b"???_", "shell"),
                (b"\xF0\xA7\xA7_", "shell-show"),
                (b"'???_'", "shell-always"),
                (b"'\xF0\xA7\xA7_'", "shell-always-show"),
                (b"''$'\\360\\247\\247''_'", "shell-escape"),
                (b"''$'\\360\\247\\247''_'", "shell-escape-always"),
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
                ("'one\\two'", "shell-always"),
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

    #[test]
    fn test_quoting_style_display() {
        let style = QuotingStyle::Shell {
            escape: true,
            always_quote: false,
            show_control: false,
        };
        assert_eq!(format!("{style}"), "shell-escape");

        let style = QuotingStyle::Shell {
            escape: false,
            always_quote: true,
            show_control: false,
        };
        assert_eq!(format!("{style}"), "shell-always-quote");

        let style = QuotingStyle::Shell {
            escape: false,
            always_quote: false,
            show_control: true,
        };
        assert_eq!(format!("{style}"), "shell-show-control");

        let style = QuotingStyle::C {
            quotes: Quotes::Double,
        };
        assert_eq!(format!("{style}"), "C");

        let style = QuotingStyle::Literal {
            show_control: false,
        };
        assert_eq!(format!("{style}"), "literal");
    }

    #[test]
    fn test_quotes_display() {
        assert_eq!(format!("{}", Quotes::None), "None");
        assert_eq!(format!("{}", Quotes::Single), "Single");
        assert_eq!(format!("{}", Quotes::Double), "Double");
    }
}
