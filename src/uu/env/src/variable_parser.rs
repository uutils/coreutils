// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ops::Range;

use crate::EnvError;
use crate::{native_int_str::NativeIntStr, string_parser::StringParser};

pub struct VariableParser<'a, 'b> {
    pub parser: &'b mut StringParser<'a>,
}

impl<'a> VariableParser<'a, '_> {
    fn get_current_char(&self) -> Option<char> {
        self.parser.peek().ok()
    }

    fn check_variable_name_start(&self) -> Result<(), EnvError> {
        if let Some(c) = self.get_current_char() {
            if c.is_ascii_digit() {
                return Err(EnvError::EnvParsingOfVariableUnexpectedNumber(
                    self.parser.get_peek_position(),
                    c.to_string(),
                ));
            }
        }
        Ok(())
    }

    fn skip_one(&mut self) -> Result<(), EnvError> {
        self.parser.consume_chunk()?;
        Ok(())
    }

    fn parse_braced_variable_name(
        &mut self,
    ) -> Result<(&'a NativeIntStr, Option<&'a NativeIntStr>), EnvError> {
        let pos_start = self.parser.get_peek_position();

        self.check_variable_name_start()?;

        let (varname_end, default_end);
        loop {
            match self.get_current_char() {
                None => {
                    return Err(EnvError::EnvParsingOfVariableMissingClosingBrace(
                        self.parser.get_peek_position(),
                    ));
                }
                Some(c) if !c.is_ascii() || c.is_ascii_alphanumeric() || c == '_' => {
                    self.skip_one()?;
                }
                Some(':') => {
                    varname_end = self.parser.get_peek_position();
                    loop {
                        match self.get_current_char() {
                            None => {
                                return Err(
                                    EnvError::EnvParsingOfVariableMissingClosingBraceAfterValue(
                                        self.parser.get_peek_position(),
                                    ),
                                );
                            }
                            Some('}') => {
                                default_end = Some(self.parser.get_peek_position());
                                self.skip_one()?;
                                break;
                            }
                            Some(_) => {
                                self.skip_one()?;
                            }
                        }
                    }
                    break;
                }
                Some('}') => {
                    varname_end = self.parser.get_peek_position();
                    default_end = None;
                    self.skip_one()?;
                    break;
                }
                Some(c) => {
                    return Err(EnvError::EnvParsingOfVariableExceptedBraceOrColon(
                        self.parser.get_peek_position(),
                        c.to_string(),
                    ));
                }
            }
        }

        let default_opt = if let Some(default_end) = default_end {
            Some(self.parser.substring(&Range {
                start: varname_end + 1,
                end: default_end,
            }))
        } else {
            None
        };

        let varname = self.parser.substring(&Range {
            start: pos_start,
            end: varname_end,
        });

        Ok((varname, default_opt))
    }

    fn parse_unbraced_variable_name(&mut self) -> Result<&'a NativeIntStr, EnvError> {
        let pos_start = self.parser.get_peek_position();

        self.check_variable_name_start()?;

        loop {
            match self.get_current_char() {
                None => break,
                Some(c) if c.is_ascii_alphanumeric() || c == '_' => {
                    self.skip_one()?;
                }
                Some(_) => break,
            }
        }

        let pos_end = self.parser.get_peek_position();

        if pos_end == pos_start {
            return Err(EnvError::EnvParsingOfMissingVariable(pos_start));
        }

        let varname = self.parser.substring(&Range {
            start: pos_start,
            end: pos_end,
        });

        Ok(varname)
    }

    pub fn parse_variable(
        &mut self,
    ) -> Result<(&'a NativeIntStr, Option<&'a NativeIntStr>), EnvError> {
        self.skip_one()?;

        let (name, default) = match self.get_current_char() {
            None => {
                return Err(EnvError::EnvParsingOfMissingVariable(
                    self.parser.get_peek_position(),
                ));
            }
            Some('{') => {
                self.skip_one()?;
                self.parse_braced_variable_name()?
            }
            Some(_) => (self.parse_unbraced_variable_name()?, None),
        };

        Ok((name, default))
    }
}
