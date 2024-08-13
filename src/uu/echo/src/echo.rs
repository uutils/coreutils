// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use std::io::{self, Write};
use std::iter::Peekable;
use std::ops::ControlFlow;
use std::str::Chars;
use uucore::error::{FromIo, UResult};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("echo.md");
const USAGE: &str = help_usage!("echo.md");
const AFTER_HELP: &str = help_section!("after help", "echo.md");

mod options {
    pub const STRING: &str = "STRING";
    pub const NO_NEWLINE: &str = "no_newline";
    pub const ENABLE_BACKSLASH_ESCAPE: &str = "enable_backslash_escape";
    pub const DISABLE_BACKSLASH_ESCAPE: &str = "disable_backslash_escape";
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Base {
    Oct = 8,
    Hex = 16,
}

impl Base {
    fn max_digits(&self) -> u8 {
        match self {
            Self::Oct => 3,
            Self::Hex => 2,
        }
    }
}

/// Parse the numeric part of the `\xHHH` and `\0NNN` escape sequences
fn parse_code(input: &mut Peekable<Chars>, base: Base) -> Option<char> {
    // All arithmetic on `ret` needs to be wrapping, because octal input can
    // take 3 digits, which is 9 bits, and therefore more than what fits in a
    // `u8`. GNU just seems to wrap these values.
    // Note that if we instead make `ret` a `u32` and use `char::from_u32` will
    // yield incorrect results because it will interpret values larger than
    // `u8::MAX` as unicode.
    let mut ret = input.peek().and_then(|c| c.to_digit(base as u32))? as u8;

    // We can safely ignore the None case because we just peeked it.
    let _ = input.next();

    for _ in 1..base.max_digits() {
        match input.peek().and_then(|c| c.to_digit(base as u32)) {
            Some(n) => ret = ret.wrapping_mul(base as u8).wrapping_add(n as u8),
            None => break,
        }
        // We can safely ignore the None case because we just peeked it.
        let _ = input.next();
    }

    Some(ret.into())
}

fn print_escaped(input: &str, mut output: impl Write) -> io::Result<ControlFlow<()>> {
    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        if c != '\\' {
            write!(output, "{c}")?;
            continue;
        }

        // This is for the \NNN syntax for octal sequences.
        // Note that '0' is intentionally omitted because that
        // would be the \0NNN syntax.
        if let Some('1'..='8') = iter.peek() {
            if let Some(parsed) = parse_code(&mut iter, Base::Oct) {
                write!(output, "{parsed}")?;
                continue;
            }
        }

        if let Some(next) = iter.next() {
            let unescaped = match next {
                '\\' => '\\',
                'a' => '\x07',
                'b' => '\x08',
                'c' => return Ok(ControlFlow::Break(())),
                'e' => '\x1b',
                'f' => '\x0c',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'v' => '\x0b',
                'x' => {
                    if let Some(c) = parse_code(&mut iter, Base::Hex) {
                        c
                    } else {
                        write!(output, "\\")?;
                        'x'
                    }
                }
                '0' => parse_code(&mut iter, Base::Oct).unwrap_or('\0'),
                c => {
                    write!(output, "\\")?;
                    c
                }
            };
            write!(output, "{unescaped}")?;
        } else {
            write!(output, "\\")?;
        }
    }

    Ok(ControlFlow::Continue(()))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let no_newline = matches.get_flag(options::NO_NEWLINE);
    let escaped = matches.get_flag(options::ENABLE_BACKSLASH_ESCAPE);
    let values: Vec<String> = match matches.get_many::<String>(options::STRING) {
        Some(s) => s.map(|s| s.to_string()).collect(),
        None => vec![String::new()],
    };

    execute(no_newline, escaped, &values)
        .map_err_context(|| "could not write to stdout".to_string())
}

pub fn uu_app() -> Command {
    // Note: echo is different from the other utils in that it should **not**
    // have `infer_long_args(true)`, because, for example, `--ver` should be
    // printed as `--ver` and not show the version text.
    Command::new(uucore::util_name())
        // TrailingVarArg specifies the final positional argument is a VarArg
        // and it doesn't attempts the parse any further args.
        // Final argument must have multiple(true) or the usage string equivalent.
        .trailing_var_arg(true)
        .allow_hyphen_values(true)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new(options::NO_NEWLINE)
                .short('n')
                .help("do not output the trailing newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ENABLE_BACKSLASH_ESCAPE)
                .short('e')
                .help("enable interpretation of backslash escapes")
                .action(ArgAction::SetTrue)
                .overrides_with(options::DISABLE_BACKSLASH_ESCAPE),
        )
        .arg(
            Arg::new(options::DISABLE_BACKSLASH_ESCAPE)
                .short('E')
                .help("disable interpretation of backslash escapes (default)")
                .action(ArgAction::SetTrue)
                .overrides_with(options::ENABLE_BACKSLASH_ESCAPE),
        )
        .arg(Arg::new(options::STRING).action(ArgAction::Append))
}

fn execute(no_newline: bool, escaped: bool, free: &[String]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = stdout.lock();

    for (i, input) in free.iter().enumerate() {
        if i > 0 {
            write!(output, " ")?;
        }
        if escaped {
            if print_escaped(input, &mut output)?.is_break() {
                return Ok(());
            }
        } else {
            write!(output, "{input}")?;
        }
    }

    if !no_newline {
        writeln!(output)?;
    }

    Ok(())
}
