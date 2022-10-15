//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) kqueue Signum

use crate::paths::Input;
use crate::{parse, platform, Quotable};
use clap::{parser::ValueSource, Arg, ArgAction, ArgMatches, Command};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::time::Duration;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::parse_size::{parse_size, ParseSizeError};

const ABOUT: &str = "\
    Print the last 10 lines of each FILE to standard output.\n\
    With more than one FILE, precede each with a header giving the file name.\n\
    With no FILE, or when FILE is -, read standard input.\n\
    \n\
    Mandatory arguments to long flags are mandatory for short flags too.\
    ";
const USAGE: &str = "{} [FLAG]... [FILE]...";

pub mod options {
    pub mod verbosity {
        pub static QUIET: &str = "quiet";
        pub static VERBOSE: &str = "verbose";
    }
    pub static BYTES: &str = "bytes";
    pub static FOLLOW: &str = "follow";
    pub static LINES: &str = "lines";
    pub static PID: &str = "pid";
    pub static SLEEP_INT: &str = "sleep-interval";
    pub static ZERO_TERM: &str = "zero-terminated";
    pub static DISABLE_INOTIFY_TERM: &str = "-disable-inotify"; // NOTE: three hyphens is correct
    pub static USE_POLLING: &str = "use-polling";
    pub static RETRY: &str = "retry";
    pub static FOLLOW_RETRY: &str = "F";
    pub static MAX_UNCHANGED_STATS: &str = "max-unchanged-stats";
    pub static ARG_FILES: &str = "files";
    pub static PRESUME_INPUT_PIPE: &str = "-presume-input-pipe"; // NOTE: three hyphens is correct
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signum {
    Negative(u64),
    Positive(u64),
    PlusZero,
    MinusZero,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FilterMode {
    Bytes(Signum),

    /// Mode for lines delimited by delimiter as u8
    Lines(Signum, u8),
}

impl FilterMode {
    fn from(matches: &ArgMatches) -> UResult<Self> {
        let zero_term = matches.get_flag(options::ZERO_TERM);
        let mode = if let Some(arg) = matches.get_one::<String>(options::BYTES) {
            match parse_num(arg) {
                Ok(signum) => Self::Bytes(signum),
                Err(e) => {
                    return Err(UUsageError::new(
                        1,
                        format!("invalid number of bytes: {}", e),
                    ))
                }
            }
        } else if let Some(arg) = matches.get_one::<String>(options::LINES) {
            match parse_num(arg) {
                Ok(signum) => {
                    let delimiter = if zero_term { 0 } else { b'\n' };
                    Self::Lines(signum, delimiter)
                }
                Err(e) => {
                    return Err(UUsageError::new(
                        1,
                        format!("invalid number of lines: {}", e),
                    ))
                }
            }
        } else if zero_term {
            Self::default_zero()
        } else {
            Self::default()
        };

        Ok(mode)
    }

    fn default_zero() -> Self {
        Self::Lines(Signum::Negative(10), 0)
    }
}

impl Default for FilterMode {
    fn default() -> Self {
        Self::Lines(Signum::Negative(10), b'\n')
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FollowMode {
    Descriptor,
    Name,
}

#[derive(Debug, Default)]
pub struct Settings {
    pub follow: Option<FollowMode>,
    pub max_unchanged_stats: u32,
    pub mode: FilterMode,
    pub pid: platform::Pid,
    pub retry: bool,
    pub sleep_sec: Duration,
    pub use_polling: bool,
    pub verbose: bool,
    pub presume_input_pipe: bool,
    pub inputs: VecDeque<Input>,
}

impl Settings {
    pub fn from(matches: &clap::ArgMatches) -> UResult<Self> {
        let mut settings: Self = Self {
            sleep_sec: Duration::from_secs_f32(1.0),
            max_unchanged_stats: 5,
            ..Default::default()
        };

        settings.follow = if matches.get_flag(options::FOLLOW_RETRY) {
            Some(FollowMode::Name)
        } else if matches.value_source(options::FOLLOW) != Some(ValueSource::CommandLine) {
            None
        } else if matches.get_one::<String>(options::FOLLOW) == Some(String::from("name")).as_ref()
        {
            Some(FollowMode::Name)
        } else {
            Some(FollowMode::Descriptor)
        };

        settings.retry =
            matches.get_flag(options::RETRY) || matches.get_flag(options::FOLLOW_RETRY);

        if settings.retry && settings.follow.is_none() {
            show_warning!("--retry ignored; --retry is useful only when following");
        }

        if let Some(s) = matches.get_one::<String>(options::SLEEP_INT) {
            settings.sleep_sec = match s.parse::<f32>() {
                Ok(s) => Duration::from_secs_f32(s),
                Err(_) => {
                    return Err(UUsageError::new(
                        1,
                        format!("invalid number of seconds: {}", s.quote()),
                    ))
                }
            }
        }

        settings.use_polling = matches.get_flag(options::USE_POLLING);

        if let Some(s) = matches.get_one::<String>(options::MAX_UNCHANGED_STATS) {
            settings.max_unchanged_stats = match s.parse::<u32>() {
                Ok(s) => s,
                Err(_) => {
                    return Err(UUsageError::new(
                        1,
                        format!(
                            "invalid maximum number of unchanged stats between opens: {}",
                            s.quote()
                        ),
                    ));
                }
            }
        }

        if let Some(pid_str) = matches.get_one::<String>(options::PID) {
            match pid_str.parse() {
                Ok(pid) => {
                    // NOTE: on unix platform::Pid is i32, on windows platform::Pid is u32
                    #[cfg(unix)]
                    if pid < 0 {
                        // NOTE: tail only accepts an unsigned pid
                        return Err(USimpleError::new(
                            1,
                            format!("invalid PID: {}", pid_str.quote()),
                        ));
                    }
                    settings.pid = pid;
                    if settings.follow.is_none() {
                        show_warning!("PID ignored; --pid=PID is useful only when following");
                    }
                    if !platform::supports_pid_checks(settings.pid) {
                        show_warning!("--pid=PID is not supported on this system");
                        settings.pid = 0;
                    }
                }
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid PID: {}: {}", pid_str.quote(), e),
                    ));
                }
            }
        }

        settings.mode = FilterMode::from(matches)?;

        // Mimic GNU's tail for -[nc]0 without -f and exit immediately
        if settings.follow.is_none()
            && matches!(
                settings.mode,
                FilterMode::Lines(Signum::MinusZero, _) | FilterMode::Bytes(Signum::MinusZero)
            )
        {
            std::process::exit(0)
        }

        let mut inputs: VecDeque<Input> = matches
            .get_many::<String>(options::ARG_FILES)
            .map(|v| v.map(|string| Input::from(string.clone())).collect())
            .unwrap_or_default();

        // apply default and add '-' to inputs if none is present
        if inputs.is_empty() {
            inputs.push_front(Input::default());
        }

        settings.verbose = (matches.get_flag(options::verbosity::VERBOSE) || inputs.len() > 1)
            && !matches.get_flag(options::verbosity::QUIET);

        settings.inputs = inputs;

        settings.presume_input_pipe = matches.get_flag(options::PRESUME_INPUT_PIPE);

        Ok(settings)
    }
}

pub fn arg_iterate<'a>(
    mut args: impl uucore::Args + 'a,
) -> UResult<Box<dyn Iterator<Item = OsString> + 'a>> {
    // argv[0] is always present
    let first = args.next().unwrap();
    if let Some(second) = args.next() {
        if let Some(s) = second.to_str() {
            match parse::parse_obsolete(s) {
                Some(Ok(iter)) => Ok(Box::new(vec![first].into_iter().chain(iter).chain(args))),
                Some(Err(e)) => Err(UUsageError::new(
                    1,
                    match e {
                        parse::ParseError::Syntax => format!("bad argument format: {}", s.quote()),
                        parse::ParseError::Overflow => format!(
                            "invalid argument: {} Value too large for defined datatype",
                            s.quote()
                        ),
                    },
                )),
                None => Ok(Box::new(vec![first, second].into_iter().chain(args))),
            }
        } else {
            Err(UUsageError::new(1, "bad argument encoding".to_owned()))
        }
    } else {
        Ok(Box::new(vec![first].into_iter()))
    }
}

fn parse_num(src: &str) -> Result<Signum, ParseSizeError> {
    let mut size_string = src.trim();
    let mut starting_with = false;

    if let Some(c) = size_string.chars().next() {
        if c == '+' || c == '-' {
            // tail: '-' is not documented (8.32 man pages)
            size_string = &size_string[1..];
            if c == '+' {
                starting_with = true;
            }
        }
    } else {
        return Err(ParseSizeError::ParseFailure(src.to_string()));
    }

    parse_size(size_string).map(|n| match (n, starting_with) {
        (0, true) => Signum::PlusZero,
        (0, false) => Signum::MinusZero,
        (n, true) => Signum::Positive(n),
        (n, false) => Signum::Negative(n),
    })
}

pub fn stdin_is_pipe_or_fifo() -> bool {
    #[cfg(unix)]
    {
        platform::stdin_is_pipe_or_fifo()
    }
    #[cfg(windows)]
    {
        winapi_util::file::typ(winapi_util::HandleRef::stdin())
            .map(|t| t.is_disk() || t.is_pipe())
            .unwrap_or(false)
    }
}

pub fn parse_args(args: impl uucore::Args) -> UResult<Settings> {
    let matches = uu_app().try_get_matches_from(arg_iterate(args)?)?;
    Settings::from(&matches)
}

pub fn uu_app() -> Command {
    #[cfg(target_os = "linux")]
    pub static POLLING_HELP: &str = "Disable 'inotify' support and use polling instead";
    #[cfg(all(unix, not(target_os = "linux")))]
    pub static POLLING_HELP: &str = "Disable 'kqueue' support and use polling instead";
    #[cfg(target_os = "windows")]
    pub static POLLING_HELP: &str =
        "Disable 'ReadDirectoryChanges' support and use polling instead";

    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .short('c')
                .long(options::BYTES)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of bytes to print"),
        )
        .arg(
            Arg::new(options::FOLLOW)
                .short('f')
                .long(options::FOLLOW)
                .default_value("descriptor")
                .num_args(0..=1)
                .require_equals(true)
                .value_parser(["descriptor", "name"])
                .help("Print the file as it grows"),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long(options::LINES)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of lines to print"),
        )
        .arg(
            Arg::new(options::PID)
                .long(options::PID)
                .value_name("PID")
                .help("With -f, terminate after process ID, PID dies"),
        )
        .arg(
            Arg::new(options::verbosity::QUIET)
                .short('q')
                .long(options::verbosity::QUIET)
                .visible_alias("silent")
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("Never output headers giving file names")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SLEEP_INT)
                .short('s')
                .value_name("N")
                .long(options::SLEEP_INT)
                .help("Number of seconds to sleep between polling the file when running with -f"),
        )
        .arg(
            Arg::new(options::MAX_UNCHANGED_STATS)
                .value_name("N")
                .long(options::MAX_UNCHANGED_STATS)
                .help(
                    "Reopen a FILE which has not changed size after N (default 5) iterations \
                        to see if it has been unlinked or renamed (this is the usual case of rotated \
                        log files); This option is meaningful only when polling \
                        (i.e., with --use-polling) and when --follow=name",
                ),
        )
        .arg(
            Arg::new(options::verbosity::VERBOSE)
                .short('v')
                .long(options::verbosity::VERBOSE)
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("Always output headers giving file names")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERM)
                .short('z')
                .long(options::ZERO_TERM)
                .help("Line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USE_POLLING)
                .alias(options::DISABLE_INOTIFY_TERM) // NOTE: Used by GNU's test suite
                .alias("dis") // NOTE: Used by GNU's test suite
                .long(options::USE_POLLING)
                .help(POLLING_HELP)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RETRY)
                .long(options::RETRY)
                .help("Keep trying to open a file if it is inaccessible")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FOLLOW_RETRY)
                .short('F')
                .help("Same as --follow=name --retry")
                .overrides_with_all(&[options::RETRY, options::FOLLOW])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRESUME_INPUT_PIPE)
                .long("presume-input-pipe")
                .alias(options::PRESUME_INPUT_PIPE)
                .hide(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_hint(clap::ValueHint::FilePath),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_num_when_sign_is_given() {
        let result = parse_num("+0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Signum::PlusZero);

        let result = parse_num("+1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Signum::Positive(1));

        let result = parse_num("-0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Signum::MinusZero);

        let result = parse_num("-1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Signum::Negative(1));
    }

    #[test]
    fn test_parse_num_when_no_sign_is_given() {
        let result = parse_num("0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Signum::MinusZero);

        let result = parse_num("1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Signum::Negative(1));
    }
}
