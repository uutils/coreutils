// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;

use clap::{crate_version, value_parser};
use clap::{Arg, ArgAction, Command};

use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_usage};

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

#[allow(clippy::too_many_lines)]
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
