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
use uucore::format_usage;

use uucore::locale::get_message;

mod options {
    pub const STRING: &str = "STRING";
    pub const NO_NEWLINE: &str = "no_newline";
    pub const ENABLE_BACKSLASH_ESCAPE: &str = "enable_backslash_escape";
    pub const DISABLE_BACKSLASH_ESCAPE: &str = "disable_backslash_escape";
}

/// Options for the echo command.
#[derive(Debug, Clone, Copy)]
struct Options {
    /// Whether the output should have a trailing newline.
    ///
    /// True by default. `-n` disables it.
    pub trailing_newline: bool,

    /// Whether given string literals should be parsed for
    /// escape characters.
    ///
    /// False by default, can be enabled with `-e`. Always true if
    /// `POSIXLY_CORRECT` (cannot be disabled with `-E`).
    pub escape: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            trailing_newline: true,
            escape: false,
        }
    }
}

impl Options {
    fn posixly_correct_default() -> Self {
        Self {
            trailing_newline: true,
            escape: true,
        }
    }
}

/// Checks if an argument is a valid echo flag, and if
/// it is records the changes in [`Options`].
fn is_flag(arg: &OsStr, options: &mut Options) -> bool {
    let arg = arg.as_encoded_bytes();

    if arg.first() != Some(&b'-') || arg == b"-" {
        // Argument doesn't start with '-' or is '-' => not a flag.
        return false;
    }

    // We don't modify the given options until after
    // the loop because there is a chance the flag isn't
    // valid after all & shouldn't affect the options.
    let mut options_: Options = *options;

    // Skip the '-' when processing characters.
    for c in &arg[1..] {
        match c {
            b'e' => options_.escape = true,
            b'E' => options_.escape = false,
            b'n' => options_.trailing_newline = false,

            // If there is any character in an supposed flag
            // that is not a valid flag character, it is not
            // a flag.
            //
            // "-eeEnEe" => is a flag.
            // "-eeBne" => not a flag, short circuit at the B.
            _ => return false,
        }
    }

    // We are now sure that the argument is a
    // flag, and can apply the modified options.
    *options = options_;
    true
}

/// Processes command line arguments, separating flags from normal arguments.
///
/// # Returns
///
/// - Vector of non-flag arguments.
/// - [`Options`], describing how teh arguments should be interpreted.
fn filter_flags(mut args: impl uucore::Args) -> (Vec<OsString>, Options) {
    let mut arguments = Vec::with_capacity(args.size_hint().0);
    let mut options = Options::default();

    // Process arguments until first non-flag is found.
    for arg in &mut args {
        // We parse flags and aggregate the options in `options`.
        // First call to `is_echo_flag` to return false will break the loop.
        if !is_flag(&arg, &mut options) {
            // Not a flag. Can break out of flag-processing loop.
            // Don't forget to push it to the arguments too.
            arguments.push(arg);
            break;
        }
    }

    // Collect remaining non-flag arguments.
    arguments.extend(args);

    (arguments, options)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // args[0] is the name of the binary.
    let mut args = args.skip(1).peekable();

    // Check POSIX compatibility mode
    //
    // From the GNU manual, on what it should do:
    //
    // > If the POSIXLY_CORRECT environment variable is set, then when
    // > echo’s first argument is not -n it outputs option-like arguments
    // > instead of treating them as options. For example, echo -ne hello
    // > outputs ‘-ne hello’ instead of plain ‘hello’. Also backslash
    // > escapes are always enabled. To echo the string ‘-n’, one of the
    // > characters can be escaped in either octal or hexadecimal
    // > representation. For example, echo -e '\x2dn'.
    let is_posixly_correct = env::var_os("POSIXLY_CORRECT").is_some();

    let (args, options) = match is_posixly_correct {
        // if POSIXLY_CORRECT is not set we filter the flags normally
        false => filter_flags(args),

        true if args.peek().is_some_and(|arg| arg == "-n") => {
            // if POSIXLY_CORRECT is set and the first argument is the "-n" flag
            // we filter flags normally but 'escaped' is activated nonetheless.
            let (args, _) = filter_flags(args);
            (
                args,
                Options {
                    trailing_newline: false,
                    ..Options::posixly_correct_default()
                },
            )
        }

        true => {
            // if POSIXLY_CORRECT is set and the first argument is not the "-n" flag
            // we just collect all arguments as every argument is considered an argument.
            (args.collect(), Options::posixly_correct_default())
        }
    };

    execute(&mut io::stdout().lock(), args, options)?;

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
        .about(get_message("echo-about"))
        .after_help(get_message("echo-after-help"))
        .override_usage(format_usage(&get_message("echo-usage")))
        .arg(
            Arg::new(options::NO_NEWLINE)
                .short('n')
                .help(get_message("echo-help-no-newline"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ENABLE_BACKSLASH_ESCAPE)
                .short('e')
                .help(get_message("echo-help-enable-escapes"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::DISABLE_BACKSLASH_ESCAPE),
        )
        .arg(
            Arg::new(options::DISABLE_BACKSLASH_ESCAPE)
                .short('E')
                .help(get_message("echo-help-disable-escapes"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::ENABLE_BACKSLASH_ESCAPE),
        )
        .arg(
            Arg::new(options::STRING)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string()),
        )
}

fn execute(stdout: &mut StdoutLock, args: Vec<OsString>, options: Options) -> UResult<()> {
    for (i, arg) in args.into_iter().enumerate() {
        let Some(bytes) = bytes_from_os_string(arg.as_os_str()) else {
            return Err(USimpleError::new(1, get_message("echo-error-non-utf8")));
        };

        if i > 0 {
            stdout.write_all(b" ")?;
        }

        if options.escape {
            for item in parse_escape_only(bytes, OctalParsing::ThreeDigits) {
                if item.write(&mut *stdout)?.is_break() {
                    return Ok(());
                }
            }
        } else {
            stdout.write_all(bytes)?;
        }
    }

    if options.trailing_newline {
        stdout.write_all(b"\n")?;
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
        // TODO: Verify that this works correctly on these platforms
        input.to_str().map(|st| st.as_bytes())
    }
}
