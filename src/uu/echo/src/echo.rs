// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, StdoutLock, Write};
use uucore::error::{UResult, USimpleError};
use uucore::format::{EscapedChar, FormatChar, OctalParsing, parse_escape_only};
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

struct EchoFlags {
    // n flag == true
    // default = false
    pub disable_newline: bool,
    // e flag == true, E flag == false
    // default = false
    pub escape: bool,
    // '-' argument
    // default = false
    pub is_single_hyphen: bool,
}

fn is_echo_flag(arg: &OsString) -> Option<EchoFlags> {
    let bytes = arg.as_encoded_bytes();
    if bytes.first() == Some(&b'-') {
        let mut flags = EchoFlags {
            disable_newline: false,
            escape: false,
            is_single_hyphen: false,
        };
        // this is a single hyphen which is pseudo flag (stops search for more flags but has no
        // effect)
        if arg.len() == 1 {
            flags.is_single_hyphen = true;
            return Some(flags);
        } else {
            for c in &bytes[1..] {
                match c {
                    b'e' => flags.escape = true,
                    b'E' => flags.escape = false,
                    b'n' => flags.disable_newline = true,
                    // if there is any char in an argument starting with '-' that doesn't match e/E/n
                    // present means that this argument is not a flag
                    _ => return None,
                }
            }

            return Some(flags);
        }
    }
    // argument doesn't start with '-' == no flag
    None
}

fn filter_echo_flags(args: impl uucore::Args) -> (Vec<OsString>, bool, bool) {
    let mut result = Vec::new();
    let mut trailing_newline = true;
    let mut escape = false;
    let mut args_iter = args.into_iter();

    // We need to skip any possible Flag arguments until we find the first argument to echo that
    // is not a flag. If the first argument is double hyphen we inject an additional '--'
    // otherwise we switch is_first_argument boolean to skip the checks for any further arguments
    for arg in &mut args_iter {
        if let Some(echo_flags) = is_echo_flag(&arg) {
            if echo_flags.is_single_hyphen {
                // a single hyphen also breaks search for flags
                result.push(arg);
                break;
            }
            if echo_flags.disable_newline {
                trailing_newline = false;
            }
            escape = echo_flags.escape;
        } else {
            // first found argument stops search for flags, from here everything is handled as a
            // normal attribute
            result.push(arg);
            break;
        }
    }
    // push the remaining arguments into result vector
    for arg in args_iter {
        result.push(arg);
    }
    (result, trailing_newline, escape)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let is_posixly_correct = if let Ok(posixly_correct) = env::var("POSIXLY_CORRECT") {
        posixly_correct == "1"
    } else {
        false
    };

    let args_iter = args.skip(1);
    let (args, trailing_newline, escaped) = if is_posixly_correct {
        let mut args_iter = args_iter.peekable();

        if args_iter.peek() == Some(&OsString::from("-n")) {
            // if POSIXLY_CORRECT is set and the first argument is the "-n" flag
            // we filter flags normally but 'escaped' is activated nonetheless
            let (args, _, _) = filter_echo_flags(args_iter);
            (args, false, true)
        } else {
            // if POSIXLY_CORRECT is set and the first argument is not the "-n" flag
            // we just collect all arguments as every argument is considered an argument
            let args: Vec<OsString> = args_iter.collect();
            (args, true, true)
        }
    } else {
        // if POSIXLY_CORRECT is not set we filter the flags normally
        let (args, trailing_newline, escaped) = filter_echo_flags(args_iter);
        (args, trailing_newline, escaped)
    };

    let mut stdout_lock = io::stdout().lock();
    execute(&mut stdout_lock, args, trailing_newline, escaped)?;

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
        .version(uucore::crate_version!())
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
    arguments_after_options: Vec<OsString>,
    trailing_newline: bool,
    escaped: bool,
) -> UResult<()> {
    for (i, input) in arguments_after_options.into_iter().enumerate() {
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
            for item in parse_escape_only(bytes, OctalParsing::ThreeDigits) {
                match item {
                    EscapedChar::End => return Ok(()),
                    c => c.write(&mut *stdout_lock)?,
                };
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
