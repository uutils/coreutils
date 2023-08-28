// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use std::io::{self, Write};
use std::iter::Peekable;
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

/// Parse the numeric part of the `\xHHH` and `\0NNN` escape sequences
fn parse_code(input: &mut Peekable<Chars>, base: u8, max_digits: u32) -> Option<char> {
    // All arithmetic on `ret` needs to be wrapping, because octal input can
    // take 3 digits, which is 9 bits, and therefore more than what fits in a
    // `u8`. GNU just seems to wrap these values.
    // Note that if we instead make `ret` a `u32` and use `char::from_u32` will
    // yield incorrect results because it will interpret values larger than
    // `u8::MAX` as unicode.
    let mut ret = input.peek().and_then(|c| c.to_digit(base as u32))? as u8;

    // We can safely ifgnore the None case because we just peeked it.
    let _ = input.next();

    for _ in 1..max_digits {
        match input.peek().and_then(|c| c.to_digit(base as u32)) {
            Some(n) => ret = ret.wrapping_mul(base).wrapping_add(n as u8),
            None => break,
        }
        // We can safely ifgnore the None case because we just peeked it.
        let _ = input.next();
    }

    Some(ret.into())
}

fn print_escaped(input: &str, mut output: impl Write) -> io::Result<bool> {
    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        if c != '\\' {
            write!(output, "{c}")?;
            continue;
        }

        if let Some(next) = iter.next() {
            let unescaped = match next {
                '\\' => '\\',
                'a' => '\x07',
                'b' => '\x08',
                'c' => return Ok(true),
                'e' => '\x1b',
                'f' => '\x0c',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'v' => '\x0b',
                'x' => {
                    if let Some(c) = parse_code(&mut iter, 16, 2) {
                        c
                    } else {
                        write!(output, "\\")?;
                        'x'
                    }
                }
                '0' => parse_code(&mut iter, 8, 3).unwrap_or('\0'),
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

    Ok(false)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_lossy();
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
    Command::new(uucore::util_name())
        // TrailingVarArg specifies the final positional argument is a VarArg
        // and it doesn't attempts the parse any further args.
        // Final argument must have multiple(true) or the usage string equivalent.
        .trailing_var_arg(true)
        .allow_hyphen_values(true)
        .infer_long_args(true)
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
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DISABLE_BACKSLASH_ESCAPE)
                .short('E')
                .help("disable interpretation of backslash escapes (default)")
                .action(ArgAction::SetTrue),
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
            let should_stop = print_escaped(input, &mut output)?;
            if should_stop {
                break;
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
