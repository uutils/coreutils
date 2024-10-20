// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::parser::ValuesRef;
use clap::{crate_version, Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::io::{self, StdoutLock, Write};
use std::iter::Peekable;
use std::ops::ControlFlow;
use std::slice::Iter;
use uucore::error::{UResult, USimpleError};
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

enum Base {
    Oct,
    Hex,
}

impl Base {
    fn radix(&self) -> u8 {
        match self {
            Self::Oct => 8,
            Self::Hex => 16,
        }
    }

    fn max_digits(&self) -> u8 {
        match self {
            Self::Oct => 3,
            Self::Hex => 2,
        }
    }
}

/// Parse the numeric part of the `\xHHH` and `\0NNN` escape sequences
fn parse_code(input: &mut Peekable<Iter<u8>>, base: Base) -> Option<u8> {
    // All arithmetic on `sum` needs to be wrapping, because octal input can
    // take 3 digits, which is 9 bits, and therefore more than what fits in a
    // `u8`. GNU just seems to wrap these values.
    let radix = base.radix();
    let radix_u_three_two = u32::from(radix);

    let mut sum = match input.peek() {
        Some(&&ue) => match char::from(ue).to_digit(radix_u_three_two) {
            // A u8 interpreted as a hexadecimal or octal digit is never more than 16
            Some(ut) => u8::try_from(ut).unwrap(),
            None => {
                return None;
            }
        },
        None => {
            return None;
        }
    };

    // We can safely ignore the None case because we just peeked it.
    let _ = input.next();

    for _ in 1..base.max_digits() {
        match input
            .peek()
            .and_then(|&&ue| char::from(ue).to_digit(radix_u_three_two))
        {
            Some(ut) => {
                // A u8 interpreted as a hexadecimal or octal digit is never more than 16
                let ue = u8::try_from(ut).unwrap();

                sum = sum.wrapping_mul(radix).wrapping_add(ue)
            }
            None => {
                break;
            }
        }

        // We can safely ignore the None case because we just peeked it.
        let _ = input.next();
    }

    Some(sum)
}

fn print_escaped(input: &[u8], output: &mut StdoutLock) -> io::Result<ControlFlow<()>> {
    let mut iter = input.iter().peekable();

    while let Some(&current_byte) = iter.next() {
        if current_byte != b'\\' {
            output.write_all(&[current_byte])?;

            continue;
        }

        // This is for the \NNN syntax for octal sequences.
        // Note that '0' is intentionally omitted because that
        // would be the \0NNN syntax.
        if let Some(b'1'..=b'8') = iter.peek() {
            if let Some(parsed) = parse_code(&mut iter, Base::Oct) {
                output.write_all(&[parsed])?;

                continue;
            }
        }

        if let Some(next) = iter.next() {
            // For extending lifetime
            let sl: [u8; 1_usize];
            let sli: [u8; 2_usize];

            let unescaped: &[u8] = match *next {
                b'\\' => br"\",
                b'a' => b"\x07",
                b'b' => b"\x08",
                b'c' => return Ok(ControlFlow::Break(())),
                b'e' => b"\x1b",
                b'f' => b"\x0c",
                b'n' => b"\n",
                b'r' => b"\r",
                b't' => b"\t",
                b'v' => b"\x0b",
                b'x' => {
                    if let Some(ue) = parse_code(&mut iter, Base::Hex) {
                        sl = [ue];

                        &sl
                    } else {
                        br"\x"
                    }
                }
                b'0' => {
                    // \0 with any non-octal digit after it is 0
                    let parsed_octal_number_or_zero =
                        parse_code(&mut iter, Base::Oct).unwrap_or(b'\0');

                    sl = [parsed_octal_number_or_zero];

                    &sl
                }
                ue => {
                    sli = [b'\\', ue];

                    &sli
                }
            };

            output.write_all(unescaped)?;
        } else {
            output.write_all(br"\")?;
        }
    }

    Ok(ControlFlow::Continue(()))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    // TODO
    // "If the POSIXLY_CORRECT environment variable is set, then when echo’s first argument is not -n it outputs option-like arguments instead of treating them as options."
    // https://www.gnu.org/software/coreutils/manual/html_node/echo-invocation.html

    let no_newline = matches.get_flag(options::NO_NEWLINE);
    let escaped = matches.get_flag(options::ENABLE_BACKSLASH_ESCAPE);

    let mut stdout_lock = io::stdout().lock();

    match matches.get_many::<OsString>(options::STRING) {
        Some(va) => {
            execute(&mut stdout_lock, no_newline, escaped, va)?;
        }
        None => {
            // No strings to print, so just handle newline setting
            if !no_newline {
                stdout_lock.write_all(b"\n")?;
            }
        }
    }

    Ok(())
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
        .arg(
            Arg::new(options::STRING)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string()),
        )
}

fn execute(
    stdout_lock: &mut StdoutLock,
    no_newline: bool,
    escaped: bool,
    non_option_arguments: ValuesRef<'_, OsString>,
) -> UResult<()> {
    for (i, input) in non_option_arguments.into_iter().enumerate() {
        let Some(bytes) = bytes_from_os_string(input.as_os_str()) else {
            return Err(USimpleError::new(
                1,
                "Non-UTF-8 arguments provided, but this platform does not support them",
            ));
        };

        if i > 0 {
            stdout_lock.write_all(b" ")?;
        }

        if escaped {
            if print_escaped(bytes, stdout_lock)?.is_break() {
                return Ok(());
            }
        } else {
            stdout_lock.write_all(bytes)?;
        }
    }

    if !no_newline {
        stdout_lock.write_all(b"\n")?;
    }

    Ok(())
}

fn bytes_from_os_string(input: &OsStr) -> Option<&[u8]> {
    let option = {
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::ffi::OsStrExt;

            Some(input.as_bytes())
        }

        #[cfg(not(target_family = "unix"))]
        {
            // TODO
            match input.to_str() {
                Some(st) => Some(st.as_bytes()),
                None => None,
            }
        }
    };

    option
}
