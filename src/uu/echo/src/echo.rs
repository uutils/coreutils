// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, StdoutLock, Write};
use uucore::error::{UResult, USimpleError};
use uucore::format::{parse_escape_only, EscapedChar, FormatChar};
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

// A workaround because clap interprets the first '--' as a marker that a value
// follows. In order to use '--' as a value, we have to inject an additional '--'
fn handle_double_hyphens(args: impl uucore::Args) -> impl uucore::Args {
    let mut result = Vec::new();
    let mut is_first_double_hyphen = true;

    for arg in args {
        if arg == "--" && is_first_double_hyphen {
            result.push(OsString::from("--"));
            is_first_double_hyphen = false;
        }
        result.push(arg);
    }

    result.into_iter()
}

fn collect_args(matches: &ArgMatches) -> Vec<OsString> {
    matches
        .get_many::<OsString>(options::STRING)
        .map_or_else(Vec::new, |values| values.cloned().collect())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let is_posixly_correct = env::var("POSIXLY_CORRECT").is_ok();

    let (args, trailing_newline, escaped) = if is_posixly_correct {
        let mut args_iter = args.skip(1).peekable();

        if args_iter.peek() == Some(&OsString::from("-n")) {
            let matches = uu_app().get_matches_from(handle_double_hyphens(args_iter));
            let args = collect_args(&matches);
            (args, false, true)
        } else {
            let args: Vec<_> = args_iter.collect();
            (args, true, true)
        }
    } else {
        let matches = uu_app().get_matches_from(handle_double_hyphens(args.into_iter()));
        let trailing_newline = !matches.get_flag(options::NO_NEWLINE);
        let escaped = matches.get_flag(options::ENABLE_BACKSLASH_ESCAPE);
        let args = collect_args(&matches);
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
            for item in parse_escape_only(bytes) {
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
