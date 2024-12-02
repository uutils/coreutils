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

enum BackslashNumberType {
    OctalStartingWithNonZero(u8),
    OctalStartingWithZero,
    Hexadecimal,
}

impl BackslashNumberType {
    fn base(&self) -> Base {
        match self {
            BackslashNumberType::OctalStartingWithZero
            | BackslashNumberType::OctalStartingWithNonZero(_) => Base::Octal,
            BackslashNumberType::Hexadecimal => Base::Hexadecimal,
        }
    }
}

enum Base {
    Octal,
    Hexadecimal,
}

impl Base {
    fn ascii_to_number(&self, digit: u8) -> Option<u8> {
        fn octal_ascii_digit_to_number(digit: u8) -> Option<u8> {
            let number = match digit {
                b'0' => 0,
                b'1' => 1,
                b'2' => 2,
                b'3' => 3,
                b'4' => 4,
                b'5' => 5,
                b'6' => 6,
                b'7' => 7,
                _ => {
                    return None;
                }
            };

            Some(number)
        }

        fn hexadecimal_ascii_digit_to_number(digit: u8) -> Option<u8> {
            let number = match digit {
                b'0' => 0,
                b'1' => 1,
                b'2' => 2,
                b'3' => 3,
                b'4' => 4,
                b'5' => 5,
                b'6' => 6,
                b'7' => 7,
                b'8' => 8,
                b'9' => 9,
                b'A' | b'a' => 10,
                b'B' | b'b' => 11,
                b'C' | b'c' => 12,
                b'D' | b'd' => 13,
                b'E' | b'e' => 14,
                b'F' | b'f' => 15,
                _ => {
                    return None;
                }
            };

            Some(number)
        }

        match self {
            Self::Octal => octal_ascii_digit_to_number(digit),
            Self::Hexadecimal => hexadecimal_ascii_digit_to_number(digit),
        }
    }

    fn maximum_number_of_digits(&self) -> u8 {
        match self {
            Self::Octal => 3,
            Self::Hexadecimal => 2,
        }
    }

    fn radix(&self) -> u8 {
        match self {
            Self::Octal => 8,
            Self::Hexadecimal => 16,
        }
    }
}

/// Parse the numeric part of `\xHHH`, `\0NNN`, and `\NNN` escape sequences
fn parse_backslash_number(
    input: &mut Peekable<Iter<u8>>,
    backslash_number_type: BackslashNumberType,
) -> Option<u8> {
    let first_digit_ascii = match backslash_number_type {
        BackslashNumberType::OctalStartingWithZero | BackslashNumberType::Hexadecimal => {
            match input.peek() {
                Some(&&digit_ascii) => digit_ascii,
                None => {
                    // One of the following cases: argument ends with "\0" or "\x"
                    // If "\0" (octal): caller will print not ASCII '0', 0x30, but ASCII '\0' (NUL), 0x00
                    // If "\x" (hexadecimal): caller will print literal "\x"
                    return None;
                }
            }
        }
        // Never returns early when backslash number starts with "\1" through "\7", because caller provides the
        // first digit
        BackslashNumberType::OctalStartingWithNonZero(digit_ascii) => digit_ascii,
    };

    let base = backslash_number_type.base();

    let first_digit_number = match base.ascii_to_number(first_digit_ascii) {
        Some(digit_number) => {
            // Move past byte, since it was successfully parsed
            let _ = input.next();

            digit_number
        }
        None => {
            // The first digit was not a valid octal or hexadecimal digit
            // This should never be the case when the backslash number starts with "\1" through "\7"
            // (caller unwraps to verify this)
            return None;
        }
    };

    let radix = base.radix();

    let mut sum = first_digit_number;

    for _ in 1..(base.maximum_number_of_digits()) {
        match input
            .peek()
            .and_then(|&&digit_ascii| base.ascii_to_number(digit_ascii))
        {
            Some(digit_number) => {
                // Move past byte, since it was successfully parsed
                let _ = input.next();

                // All arithmetic on `sum` needs to be wrapping, because octal input can
                // take 3 digits, which is 9 bits, and therefore more than what fits in a
                // `u8`.
                //
                // GNU Core Utilities: "if nnn is a nine-bit value, the ninth bit is ignored"
                // https://www.gnu.org/software/coreutils/manual/html_node/echo-invocation.html
                sum = sum.wrapping_mul(radix).wrapping_add(digit_number);
            }
            None => {
                break;
            }
        }
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

        // This is for the \NNN syntax for octal sequences
        // Note that '0' is intentionally omitted, because the \0NNN syntax is handled below
        if let Some(&&first_digit @ b'1'..=b'7') = iter.peek() {
            // Unwrap because anything starting with "\1" through "\7" can be successfully parsed
            let parsed_octal_number = parse_backslash_number(
                &mut iter,
                BackslashNumberType::OctalStartingWithNonZero(first_digit),
            )
            .unwrap();

            output.write_all(&[parsed_octal_number])?;

            continue;
        }

        if let Some(next) = iter.next() {
            // For extending lifetime
            // Unnecessary when using Rust >= 1.79.0
            // https://github.com/rust-lang/rust/pull/121346
            // TODO: when we have a MSRV >= 1.79.0, delete these "hold" bindings
            let hold_one_byte_outside_of_match: [u8; 1_usize];
            let hold_two_bytes_outside_of_match: [u8; 2_usize];

            let unescaped: &[u8] = match *next {
                b'\\' => br"\",
                b'a' => b"\x07",
                b'b' => b"\x08",
                b'c' => return Ok(ControlFlow::Break(())),
                b'e' => b"\x1B",
                b'f' => b"\x0C",
                b'n' => b"\n",
                b'r' => b"\r",
                b't' => b"\t",
                b'v' => b"\x0B",
                b'x' => {
                    if let Some(parsed_hexadecimal_number) =
                        parse_backslash_number(&mut iter, BackslashNumberType::Hexadecimal)
                    {
                        // TODO: remove when we have a MSRV >= 1.79.0
                        hold_one_byte_outside_of_match = [parsed_hexadecimal_number];

                        // TODO: when we have a MSRV >= 1.79.0, return reference to a temporary array:
                        // &[parsed_hexadecimal_number]
                        &hold_one_byte_outside_of_match
                    } else {
                        // "\x" with any non-hexadecimal digit after means "\x" is treated literally
                        br"\x"
                    }
                }
                b'0' => {
                    if let Some(parsed_octal_number) = parse_backslash_number(
                        &mut iter,
                        BackslashNumberType::OctalStartingWithZero,
                    ) {
                        // TODO: remove when we have a MSRV >= 1.79.0
                        hold_one_byte_outside_of_match = [parsed_octal_number];

                        // TODO: when we have a MSRV >= 1.79.0, return reference to a temporary array:
                        // &[parsed_octal_number]
                        &hold_one_byte_outside_of_match
                    } else {
                        // "\0" with any non-octal digit after it means "\0" is treated as ASCII '\0' (NUL), 0x00
                        b"\0"
                    }
                }
                other_byte => {
                    // Backslash and the following byte are treated literally
                    hold_two_bytes_outside_of_match = [b'\\', other_byte];

                    &hold_two_bytes_outside_of_match
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

    let trailing_newline = !matches.get_flag(options::NO_NEWLINE);
    let escaped = matches.get_flag(options::ENABLE_BACKSLASH_ESCAPE);

    let mut stdout_lock = io::stdout().lock();

    match matches.get_many::<OsString>(options::STRING) {
        Some(arguments_after_options) => {
            execute(
                &mut stdout_lock,
                trailing_newline,
                escaped,
                arguments_after_options,
            )?;
        }
        None => {
            // No strings to print, so just handle newline setting
            if trailing_newline {
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
    trailing_newline: bool,
    escaped: bool,
    arguments_after_options: ValuesRef<'_, OsString>,
) -> UResult<()> {
    for (i, input) in arguments_after_options.enumerate() {
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

    if trailing_newline {
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
            // Verify that this works correctly on these platforms
            input.to_str().map(|st| st.as_bytes())
        }
    };

    option
}
