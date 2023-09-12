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

fn parse_code(
    input: &mut Peekable<Chars>,
    base: u32,
    max_digits: u32,
    bits_per_digit: u32,
) -> Option<char> {
    let mut ret = 0x8000_0000;
    for _ in 0..max_digits {
        match input.peek().and_then(|c| c.to_digit(base)) {
            Some(n) => ret = (ret << bits_per_digit) | n,
            None => break,
        }
        input.next();
    }
    std::char::from_u32(ret)
}

fn print_escaped(input: &str, mut output: impl Write) -> io::Result<bool> {
    let mut should_stop = false;

    let mut buffer = ['\\'; 2];

    // TODO `cargo +nightly clippy` complains that `.peek()` is never
    // called on `iter`. However, `peek()` is called inside the
    // `parse_code()` function that borrows `iter`.
    let mut iter = input.chars().peekable();
    while let Some(mut c) = iter.next() {
        let mut start = 1;

        if c == '\\' {
            if let Some(next) = iter.next() {
                c = match next {
                    '\\' => '\\',
                    'a' => '\x07',
                    'b' => '\x08',
                    'c' => {
                        should_stop = true;
                        break;
                    }
                    'e' => '\x1b',
                    'f' => '\x0c',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'v' => '\x0b',
                    'x' => parse_code(&mut iter, 16, 2, 4).unwrap_or_else(|| {
                        start = 0;
                        next
                    }),
                    '0' => parse_code(&mut iter, 8, 3, 3).unwrap_or('\0'),
                    _ => {
                        start = 0;
                        next
                    }
                };
            }
        }

        buffer[1] = c;

        // because printing char slices is apparently not available in the standard library
        for ch in &buffer[start..] {
            write!(output, "{ch}")?;
        }
    }

    Ok(should_stop)
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
