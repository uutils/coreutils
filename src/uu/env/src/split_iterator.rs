// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// This file is based on work from Tomasz Miąsko who published it as "shell_words" crate,
// licensed under the Apache License, Version 2.0 <LICENSE-APACHE>
// or the MIT license <LICENSE-MIT>, at your option.
//
//! Process command line according to parsing rules of original GNU env.
//! Even though it looks quite like a POSIX syntax, the original
//! "shell_words" implementation had to be adapted significantly.
//!
//! Apart from the grammar differences, there is a new feature integrated: $VARIABLE expansion.
//!
//! [GNU env] <https://www.gnu.org/software/coreutils/manual/html_node/env-invocation.html#g_t_002dS_002f_002d_002dsplit_002dstring-syntax>
// spell-checker:ignore (words) Tomasz Miąsko rntfv FFFD varname

#![forbid(unsafe_code)]

use std::borrow::Cow;

use crate::native_int_str::from_native_int_representation;
use crate::native_int_str::NativeCharInt;
use crate::native_int_str::NativeIntStr;
use crate::native_int_str::NativeIntString;
use crate::parse_error::ParseError;
use crate::string_expander::StringExpander;
use crate::string_parser::StringParser;
use crate::variable_parser::VariableParser;

const BACKSLASH: char = '\\';
const DOUBLE_QUOTES: char = '\"';
const SINGLE_QUOTES: char = '\'';
const NEW_LINE: char = '\n';
const DOLLAR: char = '$';

const REPLACEMENTS: [(char, char); 9] = [
    ('r', '\r'),
    ('n', '\n'),
    ('t', '\t'),
    ('f', '\x0C'),
    ('v', '\x0B'),
    ('_', ' '),
    ('#', '#'),
    ('$', '$'),
    ('"', '"'),
];

const ASCII_WHITESPACE_CHARS: [char; 6] = [' ', '\t', '\r', '\n', '\x0B', '\x0C'];

pub struct SplitIterator<'a> {
    expander: StringExpander<'a>,
    words: Vec<Vec<NativeCharInt>>,
}

impl<'a> SplitIterator<'a> {
    pub fn new(s: &'a NativeIntStr) -> Self {
        Self {
            expander: StringExpander::new(s),
            words: Vec::new(),
        }
    }

    fn skip_one(&mut self) -> Result<(), ParseError> {
        self.expander
            .get_parser_mut()
            .consume_one_ascii_or_all_non_ascii()?;
        Ok(())
    }

    fn take_one(&mut self) -> Result<(), ParseError> {
        Ok(self.expander.take_one()?)
    }

    fn get_current_char(&self) -> Option<char> {
        self.expander.peek().ok()
    }

    fn push_char_to_word(&mut self, c: char) {
        self.expander.put_one_char(c);
    }

    fn push_word_to_words(&mut self) {
        let word = self.expander.take_collected_output();
        self.words.push(word);
    }

    fn get_parser(&self) -> &StringParser<'a> {
        self.expander.get_parser()
    }

    fn get_parser_mut(&mut self) -> &mut StringParser<'a> {
        self.expander.get_parser_mut()
    }

    fn substitute_variable<'x>(&'x mut self) -> Result<(), ParseError> {
        let mut var_parse = VariableParser::<'a, '_> {
            parser: self.get_parser_mut(),
        };

        let (name, default) = var_parse.parse_variable()?;

        let varname_os_str_cow = from_native_int_representation(Cow::Borrowed(name));
        let value = std::env::var_os(varname_os_str_cow);
        match (&value, default) {
            (None, None) => {} // do nothing, just replace it with ""
            (Some(value), _) => {
                self.expander.put_string(value);
            }
            (None, Some(default)) => {
                self.expander.put_native_string(default);
            }
        };

        Ok(())
    }

    fn check_and_replace_ascii_escape_code(&mut self, c: char) -> Result<bool, ParseError> {
        if let Some(replace) = REPLACEMENTS.iter().find(|&x| x.0 == c) {
            self.skip_one()?;
            self.push_char_to_word(replace.1);
            return Ok(true);
        }

        Ok(false)
    }

    fn make_invalid_sequence_backslash_xin_minus_s(&self, c: char) -> ParseError {
        ParseError::InvalidSequenceBackslashXInMinusS {
            pos: self.expander.get_parser().get_peek_position(),
            c,
        }
    }

    fn state_root(&mut self) -> Result<(), ParseError> {
        loop {
            match self.state_delimiter() {
                Err(ParseError::ContinueWithDelimiter) => {}
                Err(ParseError::ReachedEnd) => return Ok(()),
                result => return result,
            }
        }
    }

    fn state_delimiter(&mut self) -> Result<(), ParseError> {
        loop {
            match self.get_current_char() {
                None => return Ok(()),
                Some('#') => {
                    self.skip_one()?;
                    self.state_comment()?;
                }
                Some(BACKSLASH) => {
                    self.skip_one()?;
                    self.state_delimiter_backslash()?;
                }
                Some(c) if ASCII_WHITESPACE_CHARS.contains(&c) => {
                    self.skip_one()?;
                }
                Some(_) => {
                    // Don't consume char. Will be done in unquoted state.
                    self.state_unquoted()?;
                }
            }
        }
    }

    fn state_delimiter_backslash(&mut self) -> Result<(), ParseError> {
        match self.get_current_char() {
            None => Err(ParseError::InvalidBackslashAtEndOfStringInMinusS {
                pos: self.get_parser().get_peek_position(),
                quoting: "Delimiter".into(),
            }),
            Some('_') | Some(NEW_LINE) => {
                self.skip_one()?;
                Ok(())
            }
            Some(DOLLAR) | Some(BACKSLASH) | Some('#') | Some(SINGLE_QUOTES)
            | Some(DOUBLE_QUOTES) => {
                self.take_one()?;
                self.state_unquoted()
            }
            Some('c') => Err(ParseError::ReachedEnd),
            Some(c) if self.check_and_replace_ascii_escape_code(c)? => self.state_unquoted(),
            Some(c) => Err(self.make_invalid_sequence_backslash_xin_minus_s(c)),
        }
    }

    fn state_unquoted(&mut self) -> Result<(), ParseError> {
        loop {
            match self.get_current_char() {
                None => {
                    self.push_word_to_words();
                    return Err(ParseError::ReachedEnd);
                }
                Some(DOLLAR) => {
                    self.substitute_variable()?;
                }
                Some(SINGLE_QUOTES) => {
                    self.skip_one()?;
                    self.state_single_quoted()?;
                }
                Some(DOUBLE_QUOTES) => {
                    self.skip_one()?;
                    self.state_double_quoted()?;
                }
                Some(BACKSLASH) => {
                    self.skip_one()?;
                    self.state_unquoted_backslash()?;
                }
                Some(c) if ASCII_WHITESPACE_CHARS.contains(&c) => {
                    self.push_word_to_words();
                    self.skip_one()?;
                    return Ok(());
                }
                Some(_) => {
                    self.take_one()?;
                }
            }
        }
    }

    fn state_unquoted_backslash(&mut self) -> Result<(), ParseError> {
        match self.get_current_char() {
            None => Err(ParseError::InvalidBackslashAtEndOfStringInMinusS {
                pos: self.get_parser().get_peek_position(),
                quoting: "Unquoted".into(),
            }),
            Some(NEW_LINE) => {
                self.skip_one()?;
                Ok(())
            }
            Some('_') => {
                self.skip_one()?;
                self.push_word_to_words();
                Err(ParseError::ContinueWithDelimiter)
            }
            Some('c') => {
                self.push_word_to_words();
                Err(ParseError::ReachedEnd)
            }
            Some(DOLLAR) | Some(BACKSLASH) | Some(SINGLE_QUOTES) | Some(DOUBLE_QUOTES) => {
                self.take_one()?;
                Ok(())
            }
            Some(c) if self.check_and_replace_ascii_escape_code(c)? => Ok(()),
            Some(c) => Err(self.make_invalid_sequence_backslash_xin_minus_s(c)),
        }
    }

    fn state_single_quoted(&mut self) -> Result<(), ParseError> {
        loop {
            match self.get_current_char() {
                None => {
                    return Err(ParseError::MissingClosingQuote {
                        pos: self.get_parser().get_peek_position(),
                        c: '\'',
                    })
                }
                Some(SINGLE_QUOTES) => {
                    self.skip_one()?;
                    return Ok(());
                }
                Some(BACKSLASH) => {
                    self.skip_one()?;
                    self.split_single_quoted_backslash()?;
                }
                Some(_) => {
                    self.take_one()?;
                }
            }
        }
    }

    fn split_single_quoted_backslash(&mut self) -> Result<(), ParseError> {
        match self.get_current_char() {
            None => Err(ParseError::MissingClosingQuote {
                pos: self.get_parser().get_peek_position(),
                c: '\'',
            }),
            Some(NEW_LINE) => {
                self.skip_one()?;
                Ok(())
            }
            Some(SINGLE_QUOTES) | Some(BACKSLASH) => {
                self.take_one()?;
                Ok(())
            }
            Some(c) if REPLACEMENTS.iter().any(|&x| x.0 == c) => {
                // See GNU test-suite e11: In single quotes, \t remains as it is.
                // Comparing with GNU behavior: \a is not accepted and issues an error.
                // So apparently only known sequences are allowed, even though they are not expanded.... bug of GNU?
                self.push_char_to_word(BACKSLASH);
                self.take_one()?;
                Ok(())
            }
            Some(c) => Err(self.make_invalid_sequence_backslash_xin_minus_s(c)),
        }
    }

    fn state_double_quoted(&mut self) -> Result<(), ParseError> {
        loop {
            match self.get_current_char() {
                None => {
                    return Err(ParseError::MissingClosingQuote {
                        pos: self.get_parser().get_peek_position(),
                        c: '"',
                    })
                }
                Some(DOLLAR) => {
                    self.substitute_variable()?;
                }
                Some(DOUBLE_QUOTES) => {
                    self.skip_one()?;
                    return Ok(());
                }
                Some(BACKSLASH) => {
                    self.skip_one()?;
                    self.state_double_quoted_backslash()?;
                }
                Some(_) => {
                    self.take_one()?;
                }
            }
        }
    }

    fn state_double_quoted_backslash(&mut self) -> Result<(), ParseError> {
        match self.get_current_char() {
            None => Err(ParseError::MissingClosingQuote {
                pos: self.get_parser().get_peek_position(),
                c: '"',
            }),
            Some(NEW_LINE) => {
                self.skip_one()?;
                Ok(())
            }
            Some(DOUBLE_QUOTES) | Some(DOLLAR) | Some(BACKSLASH) => {
                self.take_one()?;
                Ok(())
            }
            Some('c') => Err(ParseError::BackslashCNotAllowedInDoubleQuotes {
                pos: self.get_parser().get_peek_position(),
            }),
            Some(c) if self.check_and_replace_ascii_escape_code(c)? => Ok(()),
            Some(c) => Err(self.make_invalid_sequence_backslash_xin_minus_s(c)),
        }
    }

    fn state_comment(&mut self) -> Result<(), ParseError> {
        loop {
            match self.get_current_char() {
                None => return Err(ParseError::ReachedEnd),
                Some(NEW_LINE) => {
                    self.skip_one()?;
                    return Ok(());
                }
                Some(_) => {
                    self.get_parser_mut().skip_until_char_or_end(NEW_LINE);
                }
            }
        }
    }

    pub fn split(mut self) -> Result<Vec<NativeIntString>, ParseError> {
        self.state_root()?;
        Ok(self.words)
    }
}

pub fn split(s: &NativeIntStr) -> Result<Vec<NativeIntString>, ParseError> {
    let split_args = SplitIterator::new(s).split()?;
    Ok(split_args)
}
