// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Main entry point for our implementation of printf.
//!
//! The [`printf`] and [`sprintf`] closely match the behavior of the
//! corresponding C functions: the former renders a formatted string
//! to stdout, the latter renders to a new [`String`] object.
use crate::display::Quotable;
use crate::error::{UResult, USimpleError};
use crate::features::tokenize::sub::SubParser;
use crate::features::tokenize::token::Token;
use crate::features::tokenize::unescaped_text::UnescapedText;
use crate::show_warning;
use itertools::put_back_n;
use std::io::{stdout, Cursor, Write};
use std::iter::Peekable;
use std::slice::Iter;

/// Memo runner of printf
/// Takes a format string and arguments
/// 1. tokenize format string into tokens, consuming
/// any subst. arguments along the way.
/// 2. feeds remaining arguments into function
/// that prints tokens.
struct Memo {
    tokens: Vec<Token>,
}

fn warn_excess_args(first_arg: &str) {
    show_warning!(
        "ignoring excess arguments, starting with {}",
        first_arg.quote()
    );
}

impl Memo {
    fn new<W>(
        writer: &mut W,
        pf_string: &str,
        pf_args_it: &mut Peekable<Iter<String>>,
    ) -> UResult<Self>
    where
        W: Write,
    {
        let mut pm = Self { tokens: Vec::new() };
        let mut it = put_back_n(pf_string.chars());
        let mut has_sub = false;
        loop {
            if let Some(x) = UnescapedText::from_it_core(writer, &mut it, false) {
                pm.tokens.push(x);
            }
            if let Some(x) = SubParser::from_it(writer, &mut it, pf_args_it)? {
                if !has_sub {
                    has_sub = true;
                }
                pm.tokens.push(x);
            }
            if let Some(x) = it.next() {
                it.put_back(x);
            } else {
                break;
            }
        }
        if !has_sub {
            let mut drain = false;
            if let Some(first_arg) = pf_args_it.peek() {
                warn_excess_args(first_arg);
                drain = true;
            }
            if drain {
                loop {
                    // drain remaining args;
                    if pf_args_it.next().is_none() {
                        break;
                    }
                }
            }
        }
        Ok(pm)
    }
    fn apply<W>(&self, writer: &mut W, pf_args_it: &mut Peekable<Iter<String>>)
    where
        W: Write,
    {
        for tkn in &self.tokens {
            tkn.write(writer, pf_args_it);
        }
    }
    fn run_all<W>(writer: &mut W, pf_string: &str, pf_args: &[String]) -> UResult<()>
    where
        W: Write,
    {
        let mut arg_it = pf_args.iter().peekable();
        let pm = Self::new(writer, pf_string, &mut arg_it)?;
        loop {
            if arg_it.peek().is_none() {
                return Ok(());
            }
            pm.apply(writer, &mut arg_it);
        }
    }
}

/// Write a formatted string to stdout.
///
/// `format_string` contains the template and `args` contains the
/// arguments to render into the template.
///
/// See also [`sprintf`], which creates a new formatted [`String`].
///
/// # Examples
///
/// ```rust
/// use uucore::memo::printf;
///
/// printf("hello %s", &["world".to_string()]).unwrap();
/// // prints "hello world"
/// ```
pub fn printf(format_string: &str, args: &[String]) -> UResult<()> {
    let mut writer = stdout();
    Memo::run_all(&mut writer, format_string, args)
}

/// Create a new formatted string.
///
/// `format_string` contains the template and `args` contains the
/// arguments to render into the template.
///
/// See also [`printf`], which prints to stdout.
///
/// # Examples
///
/// ```rust
/// use uucore::memo::sprintf;
///
/// let s = sprintf("hello %s", &["world".to_string()]).unwrap();
/// assert_eq!(s, "hello world".to_string());
/// ```
pub fn sprintf(format_string: &str, args: &[String]) -> UResult<String> {
    let mut writer = Cursor::new(vec![]);
    Memo::run_all(&mut writer, format_string, args)?;
    let buf = writer.into_inner();
    match String::from_utf8(buf) {
        Ok(s) => Ok(s),
        Err(e) => Err(USimpleError::new(
            1,
            format!("failed to parse formatted string as UTF-8: {e}"),
        )),
    }
}

#[cfg(test)]
mod tests {

    use crate::memo::sprintf;

    #[test]
    fn test_sprintf_smoke() {
        assert_eq!(sprintf("", &[]).unwrap(), "".to_string());
    }

    #[test]
    fn test_sprintf_no_args() {
        assert_eq!(
            sprintf("hello world", &[]).unwrap(),
            "hello world".to_string()
        );
    }

    #[test]
    fn test_sprintf_string() {
        assert_eq!(
            sprintf("hello %s", &["world".to_string()]).unwrap(),
            "hello world".to_string()
        );
    }
}
