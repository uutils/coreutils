// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) kqueue Signum fundu

use crate::paths::Input;
use crate::{parse, platform, Quotable};
use clap::{crate_version, value_parser};
use clap::{Arg, ArgAction, ArgMatches, Command};
use fundu::{DurationParser, SaturatingInto};
use same_file::Handle;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::time::Duration;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::parse_size::{parse_size_u64, ParseSizeError};
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_usage, show_warning};

const ABOUT: &str = help_about!("tail.md");
const USAGE: &str = help_usage!("tail.md");

pub mod options {
    pub mod verbosity {
        pub const QUIET: &str = "quiet";
        pub const VERBOSE: &str = "verbose";
    }
    pub const BYTES: &str = "bytes";
    pub const FOLLOW: &str = "follow";
    pub const LINES: &str = "lines";
    pub const PID: &str = "pid";
    pub const SLEEP_INT: &str = "sleep-interval";
    pub const ZERO_TERM: &str = "zero-terminated";
    pub const DISABLE_INOTIFY_TERM: &str = "-disable-inotify"; // NOTE: three hyphens is correct
    pub const USE_POLLING: &str = "use-polling";
    pub const RETRY: &str = "retry";
    pub const FOLLOW_RETRY: &str = "F";
    pub const MAX_UNCHANGED_STATS: &str = "max-unchanged-stats";
    pub const ARG_FILES: &str = "files";
    pub const PRESUME_INPUT_PIPE: &str = "-presume-input-pipe"; // NOTE: three hyphens is correct
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
                        format!("invalid number of bytes: '{e}'"),
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
    /// `FILE(s)` positional arguments
    pub inputs: Vec<Input>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_unchanged_stats: 5,
            sleep_sec: Duration::from_secs_f32(1.0),
            follow: Option::default(),
            mode: FilterMode::default(),
            pid: Default::default(),
            retry: Default::default(),
            use_polling: Default::default(),
            verbose: Default::default(),
            presume_input_pipe: Default::default(),
            inputs: Vec::default(),
        }
    }
}

impl Settings {
    pub fn from_obsolete_args(args: &parse::ObsoleteArgs, name: Option<&OsString>) -> Self {
        let mut settings = Self::default();
        if args.follow {
            settings.follow = if name.is_some() {
                Some(FollowMode::Name)
            } else {
                Some(FollowMode::Descriptor)
            };
        }
        settings.mode = FilterMode::from_obsolete_args(args);
        let input = if let Some(name) = name {
            Input::from(name)
        } else {
            Input::default()
        };
        settings.inputs.push(input);
        settings
    }

    pub fn from(matches: &clap::ArgMatches) -> UResult<Self> {
        // We're parsing --follow, -F and --retry under the following conditions:
        // * -F sets --retry and --follow=name
        // * plain --follow or short -f is the same like specifying --follow=descriptor
        // * All these options and flags can occur multiple times as command line arguments
        let follow_retry = matches.get_flag(options::FOLLOW_RETRY);
        // We don't need to check for occurrences of --retry if -F was specified which already sets
        // retry
        let retry = follow_retry || matches.get_flag(options::RETRY);
        let follow = match (
            follow_retry,
            matches
                .get_one::<String>(options::FOLLOW)
                .map(|s| s.as_str()),
        ) {
            // -F and --follow if -F is specified after --follow. We don't need to care about the
            // value of --follow.
            (true, Some(_))
                // It's ok to use `index_of` instead of `indices_of` since -F and  --follow
                // overwrite themselves (not only the value but also the index).
                if matches.index_of(options::FOLLOW_RETRY) > matches.index_of(options::FOLLOW) =>
            {
                Some(FollowMode::Name)
            }
            // * -F and --follow=name if --follow=name is specified after -F
            // * No occurrences of -F but --follow=name
            // * -F and no occurrences of --follow
            (_, Some("name")) | (true, None) => Some(FollowMode::Name),
            // * -F and --follow=descriptor (or plain --follow, -f) if --follow=descriptor is
            // specified after -F
            // * No occurrences of -F but --follow=descriptor, --follow, -f
            (_, Some(_)) => Some(FollowMode::Descriptor),
            // The default for no occurrences of -F or --follow
            (false, None) => None,
        };

        let mut settings: Self = Self {
            follow,
            retry,
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
            settings.sleep_sec = match DurationParser::without_time_units().parse(source) {
                Ok(duration) => SaturatingInto::<std::time::Duration>::saturating_into(duration),
                Err(_) => {
                    return Err(UUsageError::new(
                        1,
                        format!("invalid number of seconds: '{source}'"),
                    ))
                }
            }
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

        settings.inputs = matches
            .get_many::<OsString>(options::ARG_FILES)
            .map(|v| v.map(Input::from).collect())
            .unwrap_or_else(|| vec![Input::default()]);

        settings.verbose =
            settings.inputs.len() > 1 && !matches.get_flag(options::verbosity::QUIET);

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
}

pub fn parse_obsolete(arg: &OsString, input: Option<&OsString>) -> UResult<Option<Settings>> {
    match parse::parse_obsolete(arg) {
        Some(Ok(args)) => Ok(Some(Settings::from_obsolete_args(&args, input))),
        None => Ok(None),
        Some(Err(e)) => {
            let arg_str = arg.to_string_lossy();
            Err(USimpleError::new(
                1,
                match e {
                    parse::ParseError::OutOfRange => format!(
                        "invalid number: {}: Numerical result out of range",
                        arg_str.quote()
                    ),
                    parse::ParseError::Overflow => format!("invalid number: {}", arg_str.quote()),
                    // this ensures compatibility to GNU's error message (as tested in misc/tail)
                    parse::ParseError::Context => format!(
                        "option used in invalid context -- {}",
                        arg_str.chars().nth(1).unwrap_or_default()
                    ),
                    parse::ParseError::InvalidEncoding => {
                        format!("bad argument encoding: '{arg_str}'")
                    }
                },
            ))
        }
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
    }

    match parse_size_u64(size_string) {
        Ok(n) => match (n, starting_with) {
            (0, true) => Ok(Signum::PlusZero),
            (0, false) => Ok(Signum::MinusZero),
            (n, true) => Ok(Signum::Positive(n)),
            (n, false) => Ok(Signum::Negative(n)),
        },
        Err(_) => Err(ParseSizeError::ParseFailure(size_string.to_string())),
    }
}

pub fn parse_args(args: impl uucore::Args) -> UResult<Settings> {
    let args_vec: Vec<OsString> = args.collect();
    let clap_args = uu_app().try_get_matches_from(args_vec.clone());
    let clap_result = match clap_args {
        Ok(matches) => Ok(Settings::from(&matches)?),
        Err(err) => Err(err.into()),
    };

    // clap isn't able to handle obsolete syntax.
    // therefore, we want to check further for obsolete arguments.
    // argv[0] is always present, argv[1] might be obsolete arguments
    // argv[2] might contain an input file, argv[3] isn't allowed in obsolete mode
    if args_vec.len() != 2 && args_vec.len() != 3 {
        return clap_result;
    }

    // At this point, there are a few possible cases:
    //
    //    1. clap has succeeded and the arguments would be invalid for the obsolete syntax.
    //    2. The case of `tail -c 5` is ambiguous. clap parses this as `tail -c5`,
    //       but it could also be interpreted as valid obsolete syntax (tail -c on file '5').
    //       GNU chooses to interpret this as `tail -c5`, like clap.
    //    3. `tail -f foo` is also ambiguous, but has the same effect in both cases. We can safely
    //        use the clap result here.
    //    4. clap succeeded for obsolete arguments starting with '+', but misinterprets them as
    //       input files (e.g. 'tail +f').
    //    5. clap failed because of unknown flags, but possibly valid obsolete arguments
    //        (e.g. tail -l; tail -10c).
    //
    // In cases 4 & 5, we want to try parsing the obsolete arguments, which corresponds to
    // checking whether clap succeeded or the first argument starts with '+'.
    let possible_obsolete_args = &args_vec[1];
    if clap_result.is_ok() && !possible_obsolete_args.to_string_lossy().starts_with('+') {
        return clap_result;
    }
    match parse_obsolete(possible_obsolete_args, args_vec.get(2))? {
        Some(settings) => Ok(settings),
        None => clap_result,
    }
}

pub fn uu_app() -> Command {
    #[cfg(target_os = "linux")]
    const POLLING_HELP: &str = "Disable 'inotify' support and use polling instead";
    #[cfg(all(unix, not(target_os = "linux")))]
    const POLLING_HELP: &str = "Disable 'kqueue' support and use polling instead";
    #[cfg(target_os = "windows")]
    const POLLING_HELP: &str = "Disable 'ReadDirectoryChanges' support and use polling instead";

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
                .default_missing_value("descriptor")
                .num_args(0..=1)
                .require_equals(true)
                .value_parser(ShortcutValueParser::new(["descriptor", "name"]))
                .overrides_with(options::FOLLOW)
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
                .help("With -f, terminate after process ID, PID dies")
                .overrides_with(options::PID),
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
                .overrides_with(options::RETRY)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FOLLOW_RETRY)
                .short('F')
                .help("Same as --follow=name --retry")
                .overrides_with(options::FOLLOW_RETRY)
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
                .value_parser(value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath),
        )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

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

    #[rstest]
    #[case::default(vec![], None, false)]
    #[case::retry(vec!["--retry"], None, true)]
    #[case::multiple_retry(vec!["--retry", "--retry"], None, true)]
    #[case::follow_long(vec!["--follow"], Some(FollowMode::Descriptor), false)]
    #[case::follow_short(vec!["-f"], Some(FollowMode::Descriptor), false)]
    #[case::follow_long_with_retry(vec!["--follow", "--retry"], Some(FollowMode::Descriptor), true)]
    #[case::follow_short_with_retry(vec!["-f", "--retry"], Some(FollowMode::Descriptor), true)]
    #[case::follow_overwrites_previous_selection_1(vec!["--follow=name", "--follow=descriptor"], Some(FollowMode::Descriptor), false)]
    #[case::follow_overwrites_previous_selection_2(vec!["--follow=descriptor", "--follow=name"], Some(FollowMode::Name), false)]
    #[case::big_f(vec!["-F"], Some(FollowMode::Name), true)]
    #[case::multiple_big_f(vec!["-F", "-F"], Some(FollowMode::Name), true)]
    #[case::big_f_with_retry_then_does_not_change(vec!["-F", "--retry"], Some(FollowMode::Name), true)]
    #[case::big_f_with_follow_descriptor_then_change(vec!["-F", "--follow=descriptor"], Some(FollowMode::Descriptor), true)]
    #[case::multiple_big_f_with_follow_descriptor_then_no_change(vec!["-F", "--follow=descriptor", "-F"], Some(FollowMode::Name), true)]
    #[case::big_f_with_follow_short_then_change(vec!["-F", "-f"], Some(FollowMode::Descriptor), true)]
    #[case::follow_descriptor_with_big_f_then_change(vec!["--follow=descriptor", "-F"], Some(FollowMode::Name), true)]
    #[case::follow_short_with_big_f_then_change(vec!["-f", "-F"], Some(FollowMode::Name), true)]
    #[case::big_f_with_follow_name_then_not_change(vec!["-F", "--follow=name"], Some(FollowMode::Name), true)]
    #[case::follow_name_with_big_f_then_not_change(vec!["--follow=name", "-F"], Some(FollowMode::Name), true)]
    #[case::big_f_with_multiple_long_follow(vec!["--follow=name", "-F", "--follow=descriptor"], Some(FollowMode::Descriptor), true)]
    #[case::big_f_with_multiple_long_follow_name(vec!["--follow=name", "-F", "--follow=name"], Some(FollowMode::Name), true)]
    #[case::big_f_with_multiple_short_follow(vec!["-f", "-F", "-f"], Some(FollowMode::Descriptor), true)]
    #[case::multiple_big_f_with_multiple_short_follow(vec!["-f", "-F", "-f", "-F"], Some(FollowMode::Name), true)]
    fn test_parse_settings_follow_mode_and_retry(
        #[case] args: Vec<&str>,
        #[case] expected_follow_mode: Option<FollowMode>,
        #[case] expected_retry: bool,
    ) {
        let settings =
            Settings::from(&uu_app().no_binary_name(true).get_matches_from(args)).unwrap();
        assert_eq!(settings.follow, expected_follow_mode);
        assert_eq!(settings.retry, expected_retry);
    }
}
