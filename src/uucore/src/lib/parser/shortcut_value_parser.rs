// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore abcdefgh abef
use clap::{
    builder::{PossibleValue, TypedValueParser},
    error::{ContextKind, ContextValue, ErrorKind},
};

#[derive(Clone)]
pub struct ShortcutValueParser(Vec<PossibleValue>);

/// `ShortcutValueParser` is similar to clap's `PossibleValuesParser`: it verifies that the value is
/// from an enumerated set of `PossibleValue`.
///
/// Whereas `PossibleValuesParser` only accepts exact matches, `ShortcutValueParser` also accepts
/// shortcuts as long as they are unambiguous.
impl ShortcutValueParser {
    pub fn new(values: impl Into<Self>) -> Self {
        values.into()
    }

    fn generate_clap_error(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &str,
    ) -> clap::Error {
        let mut err = clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd);

        if let Some(arg) = arg {
            err.insert(
                ContextKind::InvalidArg,
                ContextValue::String(arg.to_string()),
            );
        }

        err.insert(
            ContextKind::InvalidValue,
            ContextValue::String(value.to_string()),
        );

        err.insert(
            ContextKind::ValidValue,
            ContextValue::Strings(self.0.iter().map(|x| x.get_name().to_string()).collect()),
        );

        err
    }
}

impl TypedValueParser for ShortcutValueParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value
            .to_str()
            .ok_or(clap::Error::new(ErrorKind::InvalidUtf8))?;

        let matched_values: Vec<_> = self
            .0
            .iter()
            .filter(|x| x.get_name_and_aliases().any(|name| name.starts_with(value)))
            .collect();

        match matched_values.len() {
            0 => Err(self.generate_clap_error(cmd, arg, value)),
            1 => Ok(matched_values[0].get_name().to_string()),
            _ => {
                if let Some(direct_match) = matched_values.iter().find(|x| x.get_name() == value) {
                    Ok(direct_match.get_name().to_string())
                } else {
                    Err(self.generate_clap_error(cmd, arg, value))
                }
            }
        }
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(self.0.iter().cloned()))
    }
}

impl<I, T> From<I> for ShortcutValueParser
where
    I: IntoIterator<Item = T>,
    T: Into<PossibleValue>,
{
    fn from(values: I) -> Self {
        Self(values.into_iter().map(|t| t.into()).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use clap::{builder::PossibleValue, builder::TypedValueParser, error::ErrorKind, Command};

    use super::ShortcutValueParser;

    #[test]
    fn test_parse_ref() {
        let cmd = Command::new("cmd");
        let parser = ShortcutValueParser::new(["abcd"]);
        let values = ["a", "ab", "abc", "abcd"];

        for value in values {
            let result = parser.parse_ref(&cmd, None, OsStr::new(value));
            assert_eq!("abcd", result.unwrap());
        }
    }

    #[test]
    fn test_parse_ref_with_invalid_value() {
        let cmd = Command::new("cmd");
        let parser = ShortcutValueParser::new(["abcd"]);
        let invalid_values = ["e", "abe", "abcde"];

        for invalid_value in invalid_values {
            let result = parser.parse_ref(&cmd, None, OsStr::new(invalid_value));
            assert_eq!(ErrorKind::InvalidValue, result.unwrap_err().kind());
        }
    }

    #[test]
    fn test_parse_ref_with_ambiguous_value() {
        let cmd = Command::new("cmd");
        let parser = ShortcutValueParser::new(["abcd", "abef"]);
        let ambiguous_values = ["a", "ab"];

        for ambiguous_value in ambiguous_values {
            let result = parser.parse_ref(&cmd, None, OsStr::new(ambiguous_value));
            assert_eq!(ErrorKind::InvalidValue, result.unwrap_err().kind());
        }

        let result = parser.parse_ref(&cmd, None, OsStr::new("abc"));
        assert_eq!("abcd", result.unwrap());

        let result = parser.parse_ref(&cmd, None, OsStr::new("abe"));
        assert_eq!("abef", result.unwrap());
    }

    #[test]
    fn test_parse_ref_with_ambiguous_value_that_is_a_possible_value() {
        let cmd = Command::new("cmd");
        let parser = ShortcutValueParser::new(["abcd", "abcdefgh"]);
        let result = parser.parse_ref(&cmd, None, OsStr::new("abcd"));
        assert_eq!("abcd", result.unwrap());
    }

    #[test]
    #[cfg(unix)]
    fn test_parse_ref_with_invalid_utf8() {
        use std::os::unix::prelude::OsStrExt;

        let parser = ShortcutValueParser::new(["abcd"]);
        let cmd = Command::new("cmd");

        let result = parser.parse_ref(&cmd, None, OsStr::from_bytes(&[0xc3, 0x28]));
        assert_eq!(ErrorKind::InvalidUtf8, result.unwrap_err().kind());
    }

    #[test]
    fn test_ambiguous_word_same_meaning() {
        let cmd = Command::new("cmd");
        let parser = ShortcutValueParser::new([
            PossibleValue::new("atime").alias("access"),
            "status".into(),
        ]);
        // Even though "a" is ambiguous (it might mean "atime" or "access"),
        // the meaning is uniquely defined, therefore accept it.
        let atime_values = [
            // spell-checker:disable-next-line
            "atime", "atim", "at", "a", "access", "acces", "acce", "acc", "ac",
        ];
        // spell-checker:disable-next-line
        let status_values = ["status", "statu", "stat", "sta", "st", "st"];

        for value in atime_values {
            let result = parser.parse_ref(&cmd, None, OsStr::new(value));
            assert_eq!("atime", result.unwrap());
        }
        for value in status_values {
            let result = parser.parse_ref(&cmd, None, OsStr::new(value));
            assert_eq!("status", result.unwrap());
        }
    }
}
