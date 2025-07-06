// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions for escaping names according to different quoting styles.

use std::ffi::{OsStr, OsString};
use std::fmt;

use crate::i18n::{self, UEncoding};
use crate::quoting_style::c_quoter::CQuoter;
use crate::quoting_style::literal_quoter::LiteralQuoter;
use crate::quoting_style::shell_quoter::{EscapedShellQuoter, NonEscapedShellQuoter};

mod escaped_char;
pub use escaped_char::{EscapeState, EscapedChar};

mod c_quoter;
mod literal_quoter;
mod shell_quoter;

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

/// Provide sane defaults for quoting styles.
impl QuotingStyle {
    pub const SHELL: Self = Self::Shell {
        escape: false,
        always_quote: false,
        show_control: false,
    };

    pub const SHELL_ESCAPE: Self = Self::Shell {
        escape: true,
        always_quote: false,
        show_control: false,
    };

    pub const SHELL_QUOTE: Self = Self::Shell {
        escape: false,
        always_quote: true,
        show_control: false,
    };

    pub const SHELL_ESCAPE_QUOTE: Self = Self::Shell {
        escape: true,
        always_quote: true,
        show_control: false,
    };

    pub const C_NO_QUOTES: Self = Self::C {
        quotes: Quotes::None,
    };

    pub const C_DOUBLE: Self = Self::C {
        quotes: Quotes::Double,
    };

    /// Set the `show_control` field of the quoting style.
    /// Note: this is a no-op for the `C` variant.
    pub fn show_control(self, show_control: bool) -> Self {
        use QuotingStyle::*;
        match self {
            Shell {
                escape,
                always_quote,
                ..
            } => Shell {
                escape,
                always_quote,
                show_control,
            },
            Literal { .. } => Literal { show_control },
            C { .. } => self,
        }
    }
}

/// Common interface of quoting mechanisms.
trait Quoter {
    /// Push a valid character.
    fn push_char(&mut self, input: char);

    /// Push a sequence of valid characters.
    fn push_str(&mut self, input: &str) {
        for c in input.chars() {
            self.push_char(c);
        }
    }

    /// Push a continuous slice of invalid data wrt the encoding used to
    /// decode the stream.
    fn push_invalid(&mut self, input: &[u8]);

    /// Apply post-processing on the constructed buffer and return it.
    fn finalize(self: Box<Self>) -> Vec<u8>;
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

/// Escape a name according to the given quoting style.
///
/// This inner function provides an additional flag `dirname` which
/// is meant for ls' directory name display.
fn escape_name_inner(
    name: &[u8],
    style: QuotingStyle,
    dirname: bool,
    encoding: UEncoding,
) -> Vec<u8> {
    // Early handle Literal with show_control style
    if let QuotingStyle::Literal { show_control: true } = style {
        return name.to_owned();
    }

    let mut quoter: Box<dyn Quoter> = match style {
        QuotingStyle::Literal { .. } => Box::new(LiteralQuoter::new(name.len())),
        QuotingStyle::C { quotes } => Box::new(CQuoter::new(quotes, dirname, name.len())),
        QuotingStyle::Shell {
            escape: true,
            always_quote,
            ..
        } => Box::new(EscapedShellQuoter::new(
            name,
            always_quote,
            dirname,
            name.len(),
        )),
        QuotingStyle::Shell {
            escape: false,
            always_quote,
            show_control,
        } => Box::new(NonEscapedShellQuoter::new(
            name,
            show_control,
            always_quote,
            dirname,
            name.len(),
        )),
    };

    match encoding {
        UEncoding::Ascii => {
            for b in name {
                if b.is_ascii() {
                    quoter.push_char(*b as char);
                } else {
                    quoter.push_invalid(&[*b]);
                }
            }
        }
        UEncoding::Utf8 => {
            for chunk in name.utf8_chunks() {
                quoter.push_str(chunk.valid());
                quoter.push_invalid(chunk.invalid());
            }
        }
    }

    quoter.finalize()
}

/// Escape a filename with respect to the given style.
pub fn escape_name(name: &OsStr, style: QuotingStyle, encoding: UEncoding) -> OsString {
    let name = crate::os_str_as_bytes_lossy(name);
    crate::os_string_from_vec(escape_name_inner(&name, style, false, encoding))
        .expect("all byte sequences should be valid for platform, or already replaced in name")
}

/// Retrieve the encoding from the locale and pass it to `escape_name`.
pub fn locale_aware_escape_name(name: &OsStr, style: QuotingStyle) -> OsString {
    escape_name(name, style, i18n::get_locale_encoding())
}

/// Escape a directory name with respect to the given style.
/// This is mainly meant to be used for ls' directory name printing and is not
/// likely to be used elsewhere.
pub fn escape_dir_name(dir_name: &OsStr, style: QuotingStyle, encoding: UEncoding) -> OsString {
    let name = crate::os_str_as_bytes_lossy(dir_name);
    crate::os_string_from_vec(escape_name_inner(&name, style, true, encoding))
        .expect("all byte sequences should be valid for platform, or already replaced in name")
}

/// Retrieve the encoding from the locale and pass it to `escape_dir_name`.
pub fn locale_aware_escape_dir_name(name: &OsStr, style: QuotingStyle) -> OsString {
    escape_dir_name(name, style, i18n::get_locale_encoding())
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
    use crate::{
        i18n::UEncoding,
        quoting_style::{Quotes, QuotingStyle, escape_name_inner},
    };

    // spell-checker:ignore (tests/words) one\'two one'two

    fn get_style(s: &str) -> QuotingStyle {
        match s {
            "literal" => QuotingStyle::Literal {
                show_control: false,
            },
            "literal-show" => QuotingStyle::Literal { show_control: true },
            "escape" => QuotingStyle::C_NO_QUOTES,
            "c" => QuotingStyle::C_DOUBLE,
            "shell" => QuotingStyle::SHELL,
            "shell-show" => QuotingStyle::SHELL.show_control(true),
            "shell-always" => QuotingStyle::SHELL_QUOTE,
            "shell-always-show" => QuotingStyle::SHELL_QUOTE.show_control(true),
            "shell-escape" => QuotingStyle::SHELL_ESCAPE,
            "shell-escape-always" => QuotingStyle::SHELL_ESCAPE_QUOTE,
            _ => panic!("Invalid name!"),
        }
    }

    fn check_names_inner<T>(encoding: UEncoding, name: &[u8], map: &[(T, &str)]) -> Vec<Vec<u8>> {
        map.iter()
            .map(|(_, style)| escape_name_inner(name, get_style(style), false, encoding))
            .collect()
    }

    fn check_names_encoding(encoding: UEncoding, name: &str, map: &[(&str, &str)]) {
        assert_eq!(
            map.iter()
                .map(|(correct, _)| *correct)
                .collect::<Vec<&str>>(),
            check_names_inner(encoding, name.as_bytes(), map)
                .iter()
                .map(|bytes| std::str::from_utf8(bytes)
                    .expect("valid str goes in, valid str comes out"))
                .collect::<Vec<&str>>()
        );
    }

    fn check_names_both(name: &str, map: &[(&str, &str)]) {
        check_names_encoding(UEncoding::Utf8, name, map);
        check_names_encoding(UEncoding::Ascii, name, map);
    }

    fn check_names_encoding_raw(encoding: UEncoding, name: &[u8], map: &[(&[u8], &str)]) {
        assert_eq!(
            map.iter()
                .map(|(correct, _)| *correct)
                .collect::<Vec<&[u8]>>(),
            check_names_inner(encoding, name, map)
        );
    }

    fn check_names_raw_both(name: &[u8], map: &[(&[u8], &str)]) {
        check_names_encoding_raw(UEncoding::Utf8, name, map);
        check_names_encoding_raw(UEncoding::Ascii, name, map);
    }

    #[test]
    fn test_simple_names() {
        check_names_both(
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
    fn test_empty_string() {
        check_names_both(
            "",
            &[
                ("", "literal"),
                ("", "literal-show"),
                ("", "escape"),
                ("\"\"", "c"),
                ("''", "shell"),
                ("''", "shell-show"),
                ("''", "shell-always"),
                ("''", "shell-always-show"),
                ("''", "shell-escape"),
                ("''", "shell-escape-always"),
            ],
        );
    }

    #[test]
    fn test_spaces() {
        check_names_both(
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

        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both(
            test_str,
            &[
                (test_str, "literal-show"),
                (
                    "\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217",
                    "escape",
                ),
                (
                    "\"\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217\"",
                    "c",
                ),
                (test_str, "shell-show"),
                (&format!("'{test_str}'"), "shell-always-show"),
                (
                    "''$'\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217'",
                    "shell-escape",
                ),
                (
                    "''$'\\302\\200\\302\\201\\302\\202\\302\\203\\302\\204\\302\\205\\302\\206\\302\\207\\302\\210\\302\\211\\302\\212\\302\\213\\302\\214\\302\\215\\302\\216\\302\\217'",
                    "shell-escape-always",
                ),
            ],
        );
        // Different expected output for UTF-8 and ASCII in these cases.
        check_names_encoding(
            UEncoding::Utf8,
            test_str,
            &[
                ("????????????????", "literal"),
                ("????????????????", "shell"),
                ("'????????????????'", "shell-always"),
            ],
        );
        check_names_encoding(
            UEncoding::Ascii,
            test_str,
            &[
                ("????????????????????????????????", "literal"),
                ("????????????????????????????????", "shell"),
                ("'????????????????????????????????'", "shell-always"),
            ],
        );

        // The last 16 Unicode control characters.
        let test_str = std::str::from_utf8(b"\xC2\x90\xC2\x91\xC2\x92\xC2\x93\xC2\x94\xC2\x95\xC2\x96\xC2\x97\xC2\x98\xC2\x99\xC2\x9A\xC2\x9B\xC2\x9C\xC2\x9D\xC2\x9E\xC2\x9F").unwrap();
        check_names_both(
            test_str,
            &[
                (test_str, "literal-show"),
                (
                    "\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237",
                    "escape",
                ),
                (
                    "\"\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237\"",
                    "c",
                ),
                (test_str, "shell-show"),
                (&format!("'{test_str}'"), "shell-always-show"),
                (
                    "''$'\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237'",
                    "shell-escape",
                ),
                (
                    "''$'\\302\\220\\302\\221\\302\\222\\302\\223\\302\\224\\302\\225\\302\\226\\302\\227\\302\\230\\302\\231\\302\\232\\302\\233\\302\\234\\302\\235\\302\\236\\302\\237'",
                    "shell-escape-always",
                ),
            ],
        );
        // Different expected output for UTF-8 and ASCII in these cases.
        check_names_encoding(
            UEncoding::Utf8,
            test_str,
            &[
                ("????????????????", "literal"),
                ("????????????????", "shell"),
                ("'????????????????'", "shell-always"),
            ],
        );
        check_names_encoding(
            UEncoding::Ascii,
            test_str,
            &[
                ("????????????????????????????????", "literal"),
                ("????????????????????????????????", "shell"),
                ("'????????????????????????????????'", "shell-always"),
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
        check_names_raw_both(
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
        let input = &[first2byte, continuation];
        check_names_raw_both(
            input,
            &[
                (b"\xC2\xA7", "literal-show"),
                (b"\xC2\xA7", "shell-show"),
                (b"'\xC2\xA7'", "shell-always-show"),
            ],
        );
        // Different expected output for UTF-8 and ASCII in these cases.
        check_names_encoding_raw(
            UEncoding::Utf8,
            input,
            &[
                (b"\xC2\xA7", "literal"),
                (b"\xC2\xA7", "escape"),
                (b"\"\xC2\xA7\"", "c"),
                (b"\xC2\xA7", "shell"),
                (b"'\xC2\xA7'", "shell-always"),
                (b"\xC2\xA7", "shell-escape"),
                (b"'\xC2\xA7'", "shell-escape-always"),
            ],
        );
        check_names_encoding_raw(
            UEncoding::Ascii,
            input,
            &[
                (b"??", "literal"),
                (b"\\302\\247", "escape"),
                (b"\"\\302\\247\"", "c"),
                (b"??", "shell"),
                (b"'??'", "shell-always"),
                (b"''$'\\302\\247'", "shell-escape"),
                (b"''$'\\302\\247'", "shell-escape-always"),
            ],
        );

        // mixed with valid characters
        check_names_raw_both(
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
        check_names_raw_both(
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
        check_names_raw_both(
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
        check_names_raw_both(
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
        check_names_raw_both(
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
        check_names_raw_both(
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

        let input = &[first2byte, first2byte, continuation];
        check_names_raw_both(input, &[(b"\xC2\xC2\xA7", "literal-show")]);
        // Different expected output for UTF-8 and ASCII in these cases.
        check_names_encoding_raw(
            UEncoding::Utf8,
            input,
            &[
                (b"?\xC2\xA7", "literal"),
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
        check_names_encoding_raw(
            UEncoding::Ascii,
            input,
            &[
                (b"???", "literal"),
                (b"\\302\\302\\247", "escape"),
                (b"\"\\302\\302\\247\"", "c"),
                (b"???", "shell"),
                (b"\xC2\xC2\xA7", "shell-show"),
                (b"'???'", "shell-always"),
                (b"'\xC2\xC2\xA7'", "shell-always-show"),
                (b"''$'\\302\\302\\247'", "shell-escape"),
                (b"''$'\\302\\302\\247'", "shell-escape-always"),
            ],
        );

        check_names_raw_both(
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
        check_names_raw_both(
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
        check_names_both(
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
        check_names_both(
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
        check_names_both("~", &[("'~'", "shell"), ("'~'", "shell-escape")]);
        check_names_both(
            "~name",
            &[("'~name'", "shell"), ("'~name'", "shell-escape")],
        );
        check_names_both(
            "some~name",
            &[("some~name", "shell"), ("some~name", "shell-escape")],
        );
        check_names_both("name~", &[("name~", "shell"), ("name~", "shell-escape")]);

        check_names_both("#", &[("'#'", "shell"), ("'#'", "shell-escape")]);
        check_names_both(
            "#name",
            &[("'#name'", "shell"), ("'#name'", "shell-escape")],
        );
        check_names_both(
            "some#name",
            &[("some#name", "shell"), ("some#name", "shell-escape")],
        );
        check_names_both("name#", &[("name#", "shell"), ("name#", "shell-escape")]);
    }

    #[test]
    fn test_special_chars_in_double_quotes() {
        check_names_both(
            "can'$t",
            &[
                ("'can'\\''$t'", "shell"),
                ("'can'\\''$t'", "shell-always"),
                ("'can'\\''$t'", "shell-escape"),
                ("'can'\\''$t'", "shell-escape-always"),
            ],
        );

        check_names_both(
            "can'`t",
            &[
                ("'can'\\''`t'", "shell"),
                ("'can'\\''`t'", "shell-always"),
                ("'can'\\''`t'", "shell-escape"),
                ("'can'\\''`t'", "shell-escape-always"),
            ],
        );

        check_names_both(
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
        let style = QuotingStyle::SHELL_ESCAPE;
        assert_eq!(format!("{style}"), "shell-escape");

        let style = QuotingStyle::SHELL_QUOTE;
        assert_eq!(format!("{style}"), "shell-always-quote");

        let style = QuotingStyle::SHELL.show_control(true);
        assert_eq!(format!("{style}"), "shell-show-control");

        let style = QuotingStyle::C_DOUBLE;
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
