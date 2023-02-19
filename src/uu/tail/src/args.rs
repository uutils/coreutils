//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) kqueue Signum fundu

use crate::paths::Input;
use crate::{parse, platform, Quotable};
use clap::crate_version;
use clap::{parser::ValueSource, Arg, ArgAction, ArgMatches, Command};
use fundu::DurationParser;
use is_terminal::IsTerminal;
use same_file::Handle;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::time::Duration;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::{format_usage, help_about, help_usage, show_warning};

const ABOUT: &str = help_about!("tail.md");
const USAGE: &str = help_usage!("tail.md");

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
    fn from_obsolete_args(args: &parse::ObsoleteArgs) -> Self {
        let signum = if args.plus {
            Signum::Positive(args.num)
        } else {
            Signum::Negative(args.num)
        };
        if args.lines {
            Self::Lines(signum, b'\n')
        } else {
            Self::Bytes(signum)
        }
    }

    fn from(matches: &ArgMatches) -> UResult<Self> {
        let zero_term = matches.get_flag(options::ZERO_TERM);
        let mode = if let Some(arg) = matches.get_one::<String>(options::BYTES) {
            match parse_num(arg) {
                Ok(signum) => Self::Bytes(signum),
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid number of bytes: {e}"),
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
                    return Err(USimpleError::new(
                        1,
                        format!("invalid number of lines: {e}"),
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

#[derive(Debug)]
pub enum VerificationResult {
    Ok,
    CannotFollowStdinByName,
    NoOutput,
}

#[derive(Debug)]
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

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_unchanged_stats: 5,
            sleep_sec: Duration::from_secs_f32(1.0),
            follow: Default::default(),
            mode: Default::default(),
            pid: Default::default(),
            retry: Default::default(),
            use_polling: Default::default(),
            verbose: Default::default(),
            presume_input_pipe: Default::default(),
            inputs: Default::default(),
        }
    }
}

impl Settings {
    pub fn from_obsolete_args(args: &parse::ObsoleteArgs, name: Option<&OsString>) -> Self {
        let mut settings: Self = Default::default();
        if args.follow {
            settings.follow = if name.is_some() {
                Some(FollowMode::Name)
            } else {
                Some(FollowMode::Descriptor)
            };
        }
        settings.mode = FilterMode::from_obsolete_args(args);
        let input = if let Some(name) = name {
            Input::from(&name)
        } else {
            Input::default()
        };
        settings.inputs.push_back(input);
        settings
    }

    pub fn from(matches: &clap::ArgMatches) -> UResult<Self> {
        let mut settings: Self = Self {
            follow: if matches.get_flag(options::FOLLOW_RETRY) {
                Some(FollowMode::Name)
            } else if matches.value_source(options::FOLLOW) != Some(ValueSource::CommandLine) {
                None
            } else if matches.get_one::<String>(options::FOLLOW)
                == Some(String::from("name")).as_ref()
            {
                Some(FollowMode::Name)
            } else {
                Some(FollowMode::Descriptor)
            },
            retry: matches.get_flag(options::RETRY) || matches.get_flag(options::FOLLOW_RETRY),
            use_polling: matches.get_flag(options::USE_POLLING),
            mode: FilterMode::from(matches)?,
            verbose: matches.get_flag(options::verbosity::VERBOSE),
            presume_input_pipe: matches.get_flag(options::PRESUME_INPUT_PIPE),
            ..Default::default()
        };

        if let Some(source) = matches.get_one::<String>(options::SLEEP_INT) {
            // Advantage of `fundu` over `Duration::(try_)from_secs_f64(source.parse().unwrap())`:
            // * doesn't panic on errors like `Duration::from_secs_f64` would.
            // * no precision loss, rounding errors or other floating point problems.
            // * evaluates to `Duration::MAX` if the parsed number would have exceeded
            //   `DURATION::MAX` or `infinity` was given
            // * not applied here but it supports customizable time units and provides better error
            //   messages
            settings.sleep_sec =
                DurationParser::without_time_units()
                    .parse(source)
                    .map_err(|_| {
                        UUsageError::new(1, format!("invalid number of seconds: '{source}'"))
                    })?;
        }

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
                }
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid PID: {}: {}", pid_str.quote(), e),
                    ));
                }
            }
        }

        let mut inputs: VecDeque<Input> = matches
            .get_many::<String>(options::ARG_FILES)
            .map(|v| v.map(|string| Input::from(&string)).collect())
            .unwrap_or_default();

        // apply default and add '-' to inputs if none is present
        if inputs.is_empty() {
            inputs.push_front(Input::default());
        }

        settings.verbose = inputs.len() > 1 && !matches.get_flag(options::verbosity::QUIET);

        settings.inputs = inputs;

        Ok(settings)
    }

    pub fn has_only_stdin(&self) -> bool {
        self.inputs.iter().all(|input| input.is_stdin())
    }

    pub fn has_stdin(&self) -> bool {
        self.inputs.iter().any(|input| input.is_stdin())
    }

    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    /// Check [`Settings`] for problematic configurations of tail originating from user provided
    /// command line arguments and print appropriate warnings.
    pub fn check_warnings(&self) {
        if self.retry {
            if self.follow.is_none() {
                show_warning!("--retry ignored; --retry is useful only when following");
            } else if self.follow == Some(FollowMode::Descriptor) {
                show_warning!("--retry only effective for the initial open");
            }
        }

        if self.pid != 0 {
            if self.follow.is_none() {
                show_warning!("PID ignored; --pid=PID is useful only when following");
            } else if !platform::supports_pid_checks(self.pid) {
                show_warning!("--pid=PID is not supported on this system");
            }
        }

        // This warning originates from gnu's tail implementation of the equivalent warning. If the
        // user wants to follow stdin, but tail is blocking indefinitely anyways, because of stdin
        // as `tty` (but no otherwise blocking stdin), then we print a warning that `--follow`
        // cannot be applied under these circumstances and is therefore ineffective.
        if self.follow.is_some() && self.has_stdin() {
            let blocking_stdin = self.pid == 0
                && self.follow == Some(FollowMode::Descriptor)
                && self.num_inputs() == 1
                && Handle::stdin().map_or(false, |handle| {
                    handle
                        .as_file()
                        .metadata()
                        .map_or(false, |meta| !meta.is_file())
                });

            if !blocking_stdin && std::io::stdin().is_terminal() {
                show_warning!("following standard input indefinitely is ineffective");
            }
        }
    }

    /// Verify [`Settings`] and try to find unsolvable misconfigurations of tail originating from
    /// user provided command line arguments. In contrast to [`Settings::check_warnings`] these
    /// misconfigurations usually lead to the immediate exit or abortion of the running `tail`
    /// process.
    pub fn verify(&self) -> VerificationResult {
        // Mimic GNU's tail for `tail -F`
        if self.inputs.iter().any(|i| i.is_stdin()) && self.follow == Some(FollowMode::Name) {
            return VerificationResult::CannotFollowStdinByName;
        }

        // Mimic GNU's tail for -[nc]0 without -f and exit immediately
        if self.follow.is_none()
            && matches!(
                self.mode,
                FilterMode::Lines(Signum::MinusZero, _) | FilterMode::Bytes(Signum::MinusZero)
            )
        {
            return VerificationResult::NoOutput;
        }

        VerificationResult::Ok
    }

    pub fn is_default(&self) -> bool {
        let default = Self::default();
        self.max_unchanged_stats == default.max_unchanged_stats
            && self.sleep_sec == default.sleep_sec
            && self.follow == default.follow
            && self.mode == default.mode
            && self.pid == default.pid
            && self.retry == default.retry
            && self.use_polling == default.use_polling
            && (self.verbose == default.verbose || self.inputs.len() > 1)
            && self.presume_input_pipe == default.presume_input_pipe
    }
}

pub fn parse_obsolete(args: &str) -> UResult<Option<parse::ObsoleteArgs>> {
    match parse::parse_obsolete(args) {
        Some(Ok(args)) => Ok(Some(args)),
        None => Ok(None),
        Some(Err(e)) => Err(USimpleError::new(
            1,
            match e {
                parse::ParseError::OutOfRange => format!(
                    "invalid number: {}: Numerical result out of range",
                    args.quote()
                ),
                parse::ParseError::Overflow => format!("invalid number: {}", args.quote()),
                parse::ParseError::Context => format!(
                    "option used in invalid context -- {}",
                    args.chars().nth(1).unwrap_or_default()
                ),
            },
        )),
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

pub fn parse_args(args: impl uucore::Args) -> UResult<Settings> {
    let args_vec: Vec<OsString> = args.collect();
    let clap_result = match uu_app().try_get_matches_from(args_vec.clone()) {
        Ok(matches) => {
            let settings = Settings::from(&matches)?;
            if !settings.is_default() {
                // non-default settings can't have obsolete arguments
                return Ok(settings);
            }
            Ok(settings)
        }
        Err(err) => Err(err.into()),
    };

    // clap parsing failed or resulted to default -> check for obsolete/deprecated args
    // argv[0] is always present
    let second = match args_vec.get(1) {
        Some(second) => second,
        None => return clap_result,
    };
    let second_str = match second.to_str() {
        Some(second_str) => second_str,
        None => {
            let invalid_string = second.to_string_lossy();
            return Err(USimpleError::new(
                1,
                format!("bad argument encoding: '{invalid_string}'"),
            ));
        }
    };
    match parse_obsolete(second_str)? {
        Some(obsolete_args) => Ok(Settings::from_obsolete_args(
            &obsolete_args,
            args_vec.get(2),
        )),
        None => clap_result,
    }
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
                .overrides_with_all([options::BYTES, options::LINES])
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
                .overrides_with_all([options::BYTES, options::LINES])
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
                .overrides_with_all([options::verbosity::QUIET, options::verbosity::VERBOSE])
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
                .overrides_with_all([options::verbosity::QUIET, options::verbosity::VERBOSE])
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
                .overrides_with_all([options::RETRY, options::FOLLOW])
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
    use crate::parse::ObsoleteArgs;

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

    #[test]
    fn test_parse_obsolete_settings_f() {
        let args = ObsoleteArgs {
            follow: true,
            ..Default::default()
        };
        let result = Settings::from_obsolete_args(&args, None);
        assert_eq!(result.follow, Some(FollowMode::Descriptor));

        let result = Settings::from_obsolete_args(&args, Some(&"file".into()));
        assert_eq!(result.follow, Some(FollowMode::Name));
    }
}
