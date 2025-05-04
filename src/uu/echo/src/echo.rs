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
use uucore::format::{FormatChar, OctalParsing, parse_escape_only};
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

/// Holds the options for echo command:
/// -n (disable newline)
/// -e/-E (escape handling),
struct EchoOptions {
    /// -n flag option: if true, output a trailing newline (-n disables it)
    /// Default: true
    pub trailing_newline: bool,

    /// -e enables escape interpretation, -E disables it
    /// Default: false (escape interpretation disabled)
    pub escape: bool,
}

/// Checks if an argument is a valid echo flag
/// Returns true if valid echo flag found
fn is_echo_flag(arg: &OsString, echo_options: &mut EchoOptions) -> bool {
    let bytes = arg.as_encoded_bytes();
    if bytes.first() == Some(&b'-') && arg != "-" {
        // we initialize our local variables to the "current" options so we don't override
        // previous found flags
        let mut escape = echo_options.escape;
        let mut trailing_newline = echo_options.trailing_newline;

        // Process characters after the '-'
        for c in &bytes[1..] {
            match c {
                b'e' => escape = true,
                b'E' => escape = false,
                b'n' => trailing_newline = false,
                // if there is any char in an argument starting with '-' that doesn't match e/E/n
                // present means that this argument is not a flag
                _ => return false,
            }
        }

        // we only override the options with flags being found once we parsed the whole argument
        echo_options.escape = escape;
        echo_options.trailing_newline = trailing_newline;
        return true;
    }

    // argument doesn't start with '-' or is "-" => no flag
    false
}

/// Processes command line arguments, separating flags from normal arguments
/// Returns:
/// - Vector of non-flag arguments
/// - trailing_newline: whether to print a trailing newline
/// - escape: whether to process escape sequences
fn filter_echo_flags(args: impl uucore::Args) -> (Vec<OsString>, bool, bool) {
    let mut result = Vec::new();
    let mut echo_options = EchoOptions {
        trailing_newline: true,
        escape: false,
    };
    let mut args_iter = args.into_iter();

    // Process arguments until first non-flag is found
    for arg in &mut args_iter {
        // we parse flags and store options found in "echo_option". First is_echo_flag
        // call to return false will break the loop and we will collect the remaining arguments
        if !is_echo_flag(&arg, &mut echo_options) {
            // First non-flag argument stops flag processing
            result.push(arg);
            break;
        }
    }
    // Collect remaining arguments
    for arg in args_iter {
        result.push(arg);
    }
    (result, echo_options.trailing_newline, echo_options.escape)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // Check POSIX compatibility mode
    let is_posixly_correct = env::var_os("POSIXLY_CORRECT").is_some();

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
                if item.write(&mut *stdout_lock)?.is_break() {
                    return Ok(());
                }
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
}
