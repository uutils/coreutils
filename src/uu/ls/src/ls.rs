// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) somegroup nlink tabsize dired subdired dtype colorterm stringly

use clap::{
    builder::{NonEmptyStringValueParser, PossibleValue, ValueParser},
    crate_version, Arg, ArgAction, Command,
};
use glob::{MatchOptions, Pattern};
use lscolors::LsColors;

use ansi_width::ansi_width;
use std::{cell::OnceCell, num::IntErrorKind};
use std::{collections::HashSet, io::IsTerminal};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::{
    cmp::Reverse,
    error::Error,
    ffi::OsString,
    fmt::{Display, Write as FmtWrite},
    fs::{self, DirEntry, FileType, Metadata, ReadDir},
    io::{stdout, BufWriter, ErrorKind, Stdout, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(unix)]
use std::{
    collections::HashMap,
    os::unix::fs::{FileTypeExt, MetadataExt},
    time::Duration,
};
use term_grid::{Direction, Filling, Grid, GridOptions};
use uucore::error::USimpleError;
use uucore::format::human::{human_readable, SizeFormat};
#[cfg(all(unix, not(any(target_os = "android", target_os = "macos"))))]
use uucore::fsxattr::has_acl;
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "illumos",
    target_os = "solaris"
))]
use uucore::libc::{dev_t, major, minor};
#[cfg(unix)]
use uucore::libc::{S_IXGRP, S_IXOTH, S_IXUSR};
use uucore::line_ending::LineEnding;
use uucore::quoting_style::{escape_name, QuotingStyle};
use uucore::{
    display::Quotable,
    error::{set_exit_code, UError, UResult},
    format_usage,
    fs::display_permissions,
    parse_size::parse_size_u64,
    shortcut_value_parser::ShortcutValueParser,
    version_cmp::version_cmp,
};
use uucore::{help_about, help_section, help_usage, parse_glob, show, show_error, show_warning};
mod dired;
use dired::{is_dired_arg_present, DiredOutput};
mod colors;
use colors::{color_name, StyleManager};
#[cfg(not(feature = "selinux"))]
static CONTEXT_HELP_TEXT: &str = "print any security context of each file (not enabled)";
#[cfg(feature = "selinux")]
static CONTEXT_HELP_TEXT: &str = "print any security context of each file";

const ABOUT: &str = help_about!("ls.md");
const AFTER_HELP: &str = help_section!("after help", "ls.md");
const USAGE: &str = help_usage!("ls.md");

pub mod options {
    pub mod format {
        pub static ONE_LINE: &str = "1";
        pub static LONG: &str = "long";
        pub static COLUMNS: &str = "C";
        pub static ACROSS: &str = "x";
        pub static TAB_SIZE: &str = "tabsize"; // silently ignored (see #3624)
        pub static COMMAS: &str = "m";
        pub static LONG_NO_OWNER: &str = "g";
        pub static LONG_NO_GROUP: &str = "o";
        pub static LONG_NUMERIC_UID_GID: &str = "numeric-uid-gid";
    }

    pub mod files {
        pub static ALL: &str = "all";
        pub static ALMOST_ALL: &str = "almost-all";
    }

    pub mod sort {
        pub static SIZE: &str = "S";
        pub static TIME: &str = "t";
        pub static NONE: &str = "U";
        pub static VERSION: &str = "v";
        pub static EXTENSION: &str = "X";
    }

    pub mod time {
        pub static ACCESS: &str = "u";
        pub static CHANGE: &str = "c";
    }

    pub mod size {
        pub static ALLOCATION_SIZE: &str = "size";
        pub static BLOCK_SIZE: &str = "block-size";
        pub static HUMAN_READABLE: &str = "human-readable";
        pub static SI: &str = "si";
        pub static KIBIBYTES: &str = "kibibytes";
    }

    pub mod quoting {
        pub static ESCAPE: &str = "escape";
        pub static LITERAL: &str = "literal";
        pub static C: &str = "quote-name";
    }

    pub mod indicator_style {
        pub static SLASH: &str = "p";
        pub static FILE_TYPE: &str = "file-type";
        pub static CLASSIFY: &str = "classify";
    }

    pub mod dereference {
        pub static ALL: &str = "dereference";
        pub static ARGS: &str = "dereference-command-line";
        pub static DIR_ARGS: &str = "dereference-command-line-symlink-to-dir";
    }

    pub static HELP: &str = "help";
    pub static QUOTING_STYLE: &str = "quoting-style";
    pub static HIDE_CONTROL_CHARS: &str = "hide-control-chars";
    pub static SHOW_CONTROL_CHARS: &str = "show-control-chars";
    pub static WIDTH: &str = "width";
    pub static AUTHOR: &str = "author";
    pub static NO_GROUP: &str = "no-group";
    pub static FORMAT: &str = "format";
    pub static SORT: &str = "sort";
    pub static TIME: &str = "time";
    pub static IGNORE_BACKUPS: &str = "ignore-backups";
    pub static DIRECTORY: &str = "directory";
    pub static INODE: &str = "inode";
    pub static REVERSE: &str = "reverse";
    pub static RECURSIVE: &str = "recursive";
    pub static COLOR: &str = "color";
    pub static PATHS: &str = "paths";
    pub static INDICATOR_STYLE: &str = "indicator-style";
    pub static TIME_STYLE: &str = "time-style";
    pub static FULL_TIME: &str = "full-time";
    pub static HIDE: &str = "hide";
    pub static IGNORE: &str = "ignore";
    pub static CONTEXT: &str = "context";
    pub static GROUP_DIRECTORIES_FIRST: &str = "group-directories-first";
    pub static ZERO: &str = "zero";
    pub static DIRED: &str = "dired";
    pub static HYPERLINK: &str = "hyperlink";
}

const DEFAULT_TERM_WIDTH: u16 = 80;
const POSIXLY_CORRECT_BLOCK_SIZE: u64 = 512;
const DEFAULT_BLOCK_SIZE: u64 = 1024;
const DEFAULT_FILE_SIZE_BLOCK_SIZE: u64 = 1;

#[derive(Debug)]
enum LsError {
    InvalidLineWidth(String),
    IOError(std::io::Error),
    IOErrorContext(std::io::Error, PathBuf, bool),
    BlockSizeParseError(String),
    DiredAndZeroAreIncompatible,
    AlreadyListedError(PathBuf),
    TimeStyleParseError(String, Vec<String>),
}

impl UError for LsError {
    fn code(&self) -> i32 {
        match self {
            Self::InvalidLineWidth(_) => 2,
            Self::IOError(_) => 1,
            Self::IOErrorContext(_, _, false) => 1,
            Self::IOErrorContext(_, _, true) => 2,
            Self::BlockSizeParseError(_) => 2,
            Self::DiredAndZeroAreIncompatible => 2,
            Self::AlreadyListedError(_) => 2,
            Self::TimeStyleParseError(_, _) => 2,
        }
    }
}

impl Error for LsError {}

impl Display for LsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlockSizeParseError(s) => {
                write!(f, "invalid --block-size argument {}", s.quote())
            }
            Self::DiredAndZeroAreIncompatible => {
                write!(f, "--dired and --zero are incompatible")
            }
            Self::TimeStyleParseError(s, possible_time_styles) => {
                write!(
                    f,
                    "invalid --time-style argument {}\nPossible values are: {:?}\n\nFor more information try --help",
                    s.quote(),
                    possible_time_styles
                )
            }
            Self::InvalidLineWidth(s) => write!(f, "invalid line width: {}", s.quote()),
            Self::IOError(e) => write!(f, "general io error: {e}"),
            Self::IOErrorContext(e, p, _) => {
                let error_kind = e.kind();
                let errno = e.raw_os_error().unwrap_or(1i32);

                match error_kind {
                    // No such file or directory
                    ErrorKind::NotFound => {
                        write!(
                            f,
                            "cannot access '{}': No such file or directory",
                            p.to_string_lossy(),
                        )
                    }
                    // Permission denied and Operation not permitted
                    ErrorKind::PermissionDenied =>
                    {
                        #[allow(clippy::wildcard_in_or_patterns)]
                        match errno {
                            1i32 => {
                                write!(
                                    f,
                                    "cannot access '{}': Operation not permitted",
                                    p.to_string_lossy(),
                                )
                            }
                            13i32 | _ => {
                                if p.is_dir() {
                                    write!(
                                        f,
                                        "cannot open directory '{}': Permission denied",
                                        p.to_string_lossy(),
                                    )
                                } else {
                                    write!(
                                        f,
                                        "cannot open file '{}': Permission denied",
                                        p.to_string_lossy(),
                                    )
                                }
                            }
                        }
                    }
                    _ => match errno {
                        9i32 => {
                            // only should ever occur on a read_dir on a bad fd
                            write!(
                                f,
                                "cannot open directory '{}': Bad file descriptor",
                                p.to_string_lossy(),
                            )
                        }
                        _ => {
                            write!(
                                f,
                                "unknown io error: '{:?}', '{:?}'",
                                p.to_string_lossy(),
                                e
                            )
                        }
                    },
                }
            }
            Self::AlreadyListedError(path) => {
                write!(
                    f,
                    "{}: not listing already-listed directory",
                    path.to_string_lossy()
                )
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Format {
    Columns,
    Long,
    OneLine,
    Across,
    Commas,
}

#[derive(PartialEq, Eq)]
enum Sort {
    None,
    Name,
    Size,
    Time,
    Version,
    Extension,
    Width,
}

#[derive(PartialEq, Eq)]
enum Files {
    All,
    AlmostAll,
    Normal,
}

enum Time {
    Modification,
    Access,
    Change,
    Birth,
}

#[derive(Debug)]
enum TimeStyle {
    FullIso,
    LongIso,
    Iso,
    Locale,
    Format(String),
}

fn parse_time_style(options: &clap::ArgMatches) -> Result<TimeStyle, LsError> {
    let possible_time_styles = vec![
        "full-iso".to_string(),
        "long-iso".to_string(),
        "iso".to_string(),
        "locale".to_string(),
        "+FORMAT (e.g., +%H:%M) for a 'date'-style format".to_string(),
    ];
    if let Some(field) = options.get_one::<String>(options::TIME_STYLE) {
        //If both FULL_TIME and TIME_STYLE are present
        //The one added last is dominant
        if options.get_flag(options::FULL_TIME)
            && options.indices_of(options::FULL_TIME).unwrap().last()
                > options.indices_of(options::TIME_STYLE).unwrap().last()
        {
            Ok(TimeStyle::FullIso)
        } else {
            match field.as_str() {
                "full-iso" => Ok(TimeStyle::FullIso),
                "long-iso" => Ok(TimeStyle::LongIso),
                "iso" => Ok(TimeStyle::Iso),
                "locale" => Ok(TimeStyle::Locale),
                _ => match field.chars().next().unwrap() {
                    '+' => Ok(TimeStyle::Format(String::from(&field[1..]))),
                    _ => Err(LsError::TimeStyleParseError(
                        String::from(field),
                        possible_time_styles,
                    )),
                },
            }
        }
    } else if options.get_flag(options::FULL_TIME) {
        Ok(TimeStyle::FullIso)
    } else {
        Ok(TimeStyle::Locale)
    }
}

enum Dereference {
    None,
    DirArgs,
    Args,
    All,
}

#[derive(PartialEq, Eq)]
enum IndicatorStyle {
    None,
    Slash,
    FileType,
    Classify,
}

pub struct Config {
    // Dir and vdir needs access to this field
    pub format: Format,
    files: Files,
    sort: Sort,
    recursive: bool,
    reverse: bool,
    dereference: Dereference,
    ignore_patterns: Vec<Pattern>,
    size_format: SizeFormat,
    directory: bool,
    time: Time,
    #[cfg(unix)]
    inode: bool,
    color: Option<LsColors>,
    long: LongFormat,
    alloc_size: bool,
    file_size_block_size: u64,
    #[allow(dead_code)]
    block_size: u64, // is never read on Windows
    width: u16,
    // Dir and vdir needs access to this field
    pub quoting_style: QuotingStyle,
    indicator_style: IndicatorStyle,
    time_style: TimeStyle,
    context: bool,
    selinux_supported: bool,
    group_directories_first: bool,
    line_ending: LineEnding,
    dired: bool,
    hyperlink: bool,
}

// Fields that can be removed or added to the long format
struct LongFormat {
    author: bool,
    group: bool,
    owner: bool,
    #[cfg(unix)]
    numeric_uid_gid: bool,
}

struct PaddingCollection {
    #[cfg(unix)]
    inode: usize,
    link_count: usize,
    uname: usize,
    group: usize,
    context: usize,
    size: usize,
    #[cfg(unix)]
    major: usize,
    #[cfg(unix)]
    minor: usize,
    block_size: usize,
}

/// Extracts the format to display the information based on the options provided.
///
/// # Returns
///
/// A tuple containing the Format variant and an Option containing a &'static str
/// which corresponds to the option used to define the format.
fn extract_format(options: &clap::ArgMatches) -> (Format, Option<&'static str>) {
    if let Some(format_) = options.get_one::<String>(options::FORMAT) {
        (
            match format_.as_str() {
                "long" | "verbose" => Format::Long,
                "single-column" => Format::OneLine,
                "columns" | "vertical" => Format::Columns,
                "across" | "horizontal" => Format::Across,
                "commas" => Format::Commas,
                // below should never happen as clap already restricts the values.
                _ => unreachable!("Invalid field for --format"),
            },
            Some(options::FORMAT),
        )
    } else if options.get_flag(options::format::LONG) {
        (Format::Long, Some(options::format::LONG))
    } else if options.get_flag(options::format::ACROSS) {
        (Format::Across, Some(options::format::ACROSS))
    } else if options.get_flag(options::format::COMMAS) {
        (Format::Commas, Some(options::format::COMMAS))
    } else if options.get_flag(options::format::COLUMNS) {
        (Format::Columns, Some(options::format::COLUMNS))
    } else if std::io::stdout().is_terminal() {
        (Format::Columns, None)
    } else {
        (Format::OneLine, None)
    }
}

/// Extracts the type of files to display
///
/// # Returns
///
/// A Files variant representing the type of files to display.
fn extract_files(options: &clap::ArgMatches) -> Files {
    if options.get_flag(options::files::ALL) {
        Files::All
    } else if options.get_flag(options::files::ALMOST_ALL) {
        Files::AlmostAll
    } else {
        Files::Normal
    }
}

/// Extracts the sorting method to use based on the options provided.
///
/// # Returns
///
/// A Sort variant representing the sorting method to use.
fn extract_sort(options: &clap::ArgMatches) -> Sort {
    if let Some(field) = options.get_one::<String>(options::SORT) {
        match field.as_str() {
            "none" => Sort::None,
            "name" => Sort::Name,
            "time" => Sort::Time,
            "size" => Sort::Size,
            "version" => Sort::Version,
            "extension" => Sort::Extension,
            "width" => Sort::Width,
            // below should never happen as clap already restricts the values.
            _ => unreachable!("Invalid field for --sort"),
        }
    } else if options.get_flag(options::sort::TIME) {
        Sort::Time
    } else if options.get_flag(options::sort::SIZE) {
        Sort::Size
    } else if options.get_flag(options::sort::NONE) {
        Sort::None
    } else if options.get_flag(options::sort::VERSION) {
        Sort::Version
    } else if options.get_flag(options::sort::EXTENSION) {
        Sort::Extension
    } else {
        Sort::Name
    }
}

/// Extracts the time to use based on the options provided.
///
/// # Returns
///
/// A Time variant representing the time to use.
fn extract_time(options: &clap::ArgMatches) -> Time {
    if let Some(field) = options.get_one::<String>(options::TIME) {
        match field.as_str() {
            "ctime" | "status" => Time::Change,
            "access" | "atime" | "use" => Time::Access,
            "birth" | "creation" => Time::Birth,
            // below should never happen as clap already restricts the values.
            _ => unreachable!("Invalid field for --time"),
        }
    } else if options.get_flag(options::time::ACCESS) {
        Time::Access
    } else if options.get_flag(options::time::CHANGE) {
        Time::Change
    } else {
        Time::Modification
    }
}

// Some env variables can be passed
// For now, we are only verifying if empty or not and known for TERM
fn is_color_compatible_term() -> bool {
    let is_term_set = std::env::var("TERM").is_ok();
    let is_colorterm_set = std::env::var("COLORTERM").is_ok();

    let term = std::env::var("TERM").unwrap_or_default();
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();

    // Search function in the TERM struct to manage the wildcards
    let term_matches = |term: &str| -> bool {
        uucore::colors::TERMS.iter().any(|&pattern| {
            term == pattern
                || (pattern.ends_with('*') && term.starts_with(&pattern[..pattern.len() - 1]))
        })
    };

    if is_term_set && term.is_empty() && is_colorterm_set && colorterm.is_empty() {
        return false;
    }

    if !term.is_empty() && !term_matches(&term) {
        return false;
    }
    true
}

/// Extracts the color option to use based on the options provided.
///
/// # Returns
///
/// A boolean representing whether or not to use color.
fn extract_color(options: &clap::ArgMatches) -> bool {
    if !is_color_compatible_term() {
        return false;
    }

    match options.get_one::<String>(options::COLOR) {
        None => options.contains_id(options::COLOR),
        Some(val) => match val.as_str() {
            "" | "always" | "yes" | "force" => true,
            "auto" | "tty" | "if-tty" => std::io::stdout().is_terminal(),
            /* "never" | "no" | "none" | */ _ => false,
        },
    }
}

/// Extracts the hyperlink option to use based on the options provided.
///
/// # Returns
///
/// A boolean representing whether to hyperlink files.
fn extract_hyperlink(options: &clap::ArgMatches) -> bool {
    let hyperlink = options
        .get_one::<String>(options::HYPERLINK)
        .unwrap()
        .as_str();

    match hyperlink {
        "always" | "yes" | "force" => true,
        "auto" | "tty" | "if-tty" => std::io::stdout().is_terminal(),
        "never" | "no" | "none" => false,
        _ => unreachable!("should be handled by clap"),
    }
}

/// Match the argument given to --quoting-style or the QUOTING_STYLE env variable.
///
/// # Arguments
///
/// * `style`: the actual argument string
/// * `show_control` - A boolean value representing whether or not to show control characters.
///
/// # Returns
///
/// * An option with None if the style string is invalid, or a `QuotingStyle` wrapped in `Some`.
fn match_quoting_style_name(style: &str, show_control: bool) -> Option<QuotingStyle> {
    match style {
        "literal" => Some(QuotingStyle::Literal { show_control }),
        "shell" => Some(QuotingStyle::Shell {
            escape: false,
            always_quote: false,
            show_control,
        }),
        "shell-always" => Some(QuotingStyle::Shell {
            escape: false,
            always_quote: true,
            show_control,
        }),
        "shell-escape" => Some(QuotingStyle::Shell {
            escape: true,
            always_quote: false,
            show_control,
        }),
        "shell-escape-always" => Some(QuotingStyle::Shell {
            escape: true,
            always_quote: true,
            show_control,
        }),
        "c" => Some(QuotingStyle::C {
            quotes: quoting_style::Quotes::Double,
        }),
        "escape" => Some(QuotingStyle::C {
            quotes: quoting_style::Quotes::None,
        }),
        _ => None,
    }
}

/// Extracts the quoting style to use based on the options provided.
/// If no options are given, it looks if a default quoting style is provided
/// through the QUOTING_STYLE environment variable.
///
/// # Arguments
///
/// * `options` - A reference to a clap::ArgMatches object containing command line arguments.
/// * `show_control` - A boolean value representing whether or not to show control characters.
///
/// # Returns
///
/// A QuotingStyle variant representing the quoting style to use.
fn extract_quoting_style(options: &clap::ArgMatches, show_control: bool) -> QuotingStyle {
    let opt_quoting_style = options.get_one::<String>(options::QUOTING_STYLE);

    if let Some(style) = opt_quoting_style {
        match match_quoting_style_name(style, show_control) {
            Some(qs) => qs,
            None => unreachable!("Should have been caught by Clap"),
        }
    } else if options.get_flag(options::quoting::LITERAL) {
        QuotingStyle::Literal { show_control }
    } else if options.get_flag(options::quoting::ESCAPE) {
        QuotingStyle::C {
            quotes: quoting_style::Quotes::None,
        }
    } else if options.get_flag(options::quoting::C) {
        QuotingStyle::C {
            quotes: quoting_style::Quotes::Double,
        }
    } else if options.get_flag(options::DIRED) {
        QuotingStyle::Literal { show_control }
    } else {
        // If set, the QUOTING_STYLE environment variable specifies a default style.
        if let Ok(style) = std::env::var("QUOTING_STYLE") {
            match match_quoting_style_name(style.as_str(), show_control) {
                Some(qs) => return qs,
                None => eprintln!(
                    "{}: Ignoring invalid value of environment variable QUOTING_STYLE: '{}'",
                    std::env::args().next().unwrap_or_else(|| "ls".to_string()),
                    style
                ),
            }
        }

        // By default, `ls` uses Shell escape quoting style when writing to a terminal file
        // descriptor and Literal otherwise.
        if std::io::stdout().is_terminal() {
            QuotingStyle::Shell {
                escape: true,
                always_quote: false,
                show_control,
            }
        } else {
            QuotingStyle::Literal { show_control }
        }
    }
}

/// Extracts the indicator style to use based on the options provided.
///
/// # Returns
///
/// An IndicatorStyle variant representing the indicator style to use.
fn extract_indicator_style(options: &clap::ArgMatches) -> IndicatorStyle {
    if let Some(field) = options.get_one::<String>(options::INDICATOR_STYLE) {
        match field.as_str() {
            "none" => IndicatorStyle::None,
            "file-type" => IndicatorStyle::FileType,
            "classify" => IndicatorStyle::Classify,
            "slash" => IndicatorStyle::Slash,
            &_ => IndicatorStyle::None,
        }
    } else if let Some(field) = options.get_one::<String>(options::indicator_style::CLASSIFY) {
        match field.as_str() {
            "never" | "no" | "none" => IndicatorStyle::None,
            "always" | "yes" | "force" => IndicatorStyle::Classify,
            "auto" | "tty" | "if-tty" => {
                if std::io::stdout().is_terminal() {
                    IndicatorStyle::Classify
                } else {
                    IndicatorStyle::None
                }
            }
            &_ => IndicatorStyle::None,
        }
    } else if options.get_flag(options::indicator_style::SLASH) {
        IndicatorStyle::Slash
    } else if options.get_flag(options::indicator_style::FILE_TYPE) {
        IndicatorStyle::FileType
    } else {
        IndicatorStyle::None
    }
}

/// Parses the width value from either the command line arguments or the environment variables.
fn parse_width(width_match: Option<&String>) -> Result<u16, LsError> {
    let parse_width_from_args = |s: &str| -> Result<u16, LsError> {
        let radix = if s.starts_with('0') && s.len() > 1 {
            8
        } else {
            10
        };
        match u16::from_str_radix(s, radix) {
            Ok(x) => Ok(x),
            Err(e) => match e.kind() {
                IntErrorKind::PosOverflow => Ok(u16::MAX),
                _ => Err(LsError::InvalidLineWidth(s.into())),
            },
        }
    };

    let parse_width_from_env =
        |columns: OsString| match columns.to_str().and_then(|s| s.parse().ok()) {
            Some(columns) => columns,
            None => {
                show_error!(
                    "ignoring invalid width in environment variable COLUMNS: {}",
                    columns.quote()
                );
                DEFAULT_TERM_WIDTH
            }
        };

    let calculate_term_size = || match terminal_size::terminal_size() {
        Some((width, _)) => width.0,
        None => DEFAULT_TERM_WIDTH,
    };

    let ret = match width_match {
        Some(x) => parse_width_from_args(x)?,
        None => match std::env::var_os("COLUMNS") {
            Some(columns) => parse_width_from_env(columns),
            None => calculate_term_size(),
        },
    };

    Ok(ret)
}

impl Config {
    #[allow(clippy::cognitive_complexity)]
    pub fn from(options: &clap::ArgMatches) -> UResult<Self> {
        let context = options.get_flag(options::CONTEXT);
        let (mut format, opt) = extract_format(options);
        let files = extract_files(options);

        // The -o, -n and -g options are tricky. They cannot override with each
        // other because it's possible to combine them. For example, the option
        // -og should hide both owner and group. Furthermore, they are not
        // reset if -l or --format=long is used. So these should just show the
        // group: -gl or "-g --format=long". Finally, they are also not reset
        // when switching to a different format option in-between like this:
        // -ogCl or "-og --format=vertical --format=long".
        //
        // -1 has a similar issue: it does nothing if the format is long. This
        // actually makes it distinct from the --format=singe-column option,
        // which always applies.
        //
        // The idea here is to not let these options override with the other
        // options, but manually whether they have an index that's greater than
        // the other format options. If so, we set the appropriate format.
        if format != Format::Long {
            let idx = opt
                .and_then(|opt| options.indices_of(opt).map(|x| x.max().unwrap()))
                .unwrap_or(0);
            if [
                options::format::LONG_NO_OWNER,
                options::format::LONG_NO_GROUP,
                options::format::LONG_NUMERIC_UID_GID,
                options::FULL_TIME,
            ]
            .iter()
            .flat_map(|opt| {
                if options.value_source(opt) == Some(clap::parser::ValueSource::CommandLine) {
                    options.indices_of(opt)
                } else {
                    None
                }
            })
            .flatten()
            .any(|i| i >= idx)
            {
                format = Format::Long;
            } else if let Some(mut indices) = options.indices_of(options::format::ONE_LINE) {
                if options.value_source(options::format::ONE_LINE)
                    == Some(clap::parser::ValueSource::CommandLine)
                    && indices.any(|i| i > idx)
                {
                    format = Format::OneLine;
                }
            }
        }

        let sort = extract_sort(options);
        let time = extract_time(options);
        let mut needs_color = extract_color(options);
        let hyperlink = extract_hyperlink(options);

        let opt_block_size = options.get_one::<String>(options::size::BLOCK_SIZE);
        let opt_si = opt_block_size.is_some()
            && options
                .get_one::<String>(options::size::BLOCK_SIZE)
                .unwrap()
                .eq("si")
            || options.get_flag(options::size::SI);
        let opt_hr = (opt_block_size.is_some()
            && options
                .get_one::<String>(options::size::BLOCK_SIZE)
                .unwrap()
                .eq("human-readable"))
            || options.get_flag(options::size::HUMAN_READABLE);
        let opt_kb = options.get_flag(options::size::KIBIBYTES);

        let size_format = if opt_si {
            SizeFormat::Decimal
        } else if opt_hr {
            SizeFormat::Binary
        } else {
            SizeFormat::Bytes
        };

        let env_var_blocksize = std::env::var_os("BLOCKSIZE");
        let env_var_block_size = std::env::var_os("BLOCK_SIZE");
        let env_var_ls_block_size = std::env::var_os("LS_BLOCK_SIZE");
        let env_var_posixly_correct = std::env::var_os("POSIXLY_CORRECT");
        let mut is_env_var_blocksize = false;

        let raw_block_size = if let Some(opt_block_size) = opt_block_size {
            OsString::from(opt_block_size)
        } else if let Some(env_var_ls_block_size) = env_var_ls_block_size {
            env_var_ls_block_size
        } else if let Some(env_var_block_size) = env_var_block_size {
            env_var_block_size
        } else if let Some(env_var_blocksize) = env_var_blocksize {
            is_env_var_blocksize = true;
            env_var_blocksize
        } else {
            OsString::from("")
        };

        let (file_size_block_size, block_size) = if !opt_si && !opt_hr && !raw_block_size.is_empty()
        {
            match parse_size_u64(&raw_block_size.to_string_lossy()) {
                Ok(size) => match (is_env_var_blocksize, opt_kb) {
                    (true, true) => (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE),
                    (true, false) => (DEFAULT_FILE_SIZE_BLOCK_SIZE, size),
                    (false, true) => {
                        // --block-size overrides -k
                        if opt_block_size.is_some() {
                            (size, size)
                        } else {
                            (size, DEFAULT_BLOCK_SIZE)
                        }
                    }
                    (false, false) => (size, size),
                },
                Err(_) => {
                    // only fail if invalid block size was specified with --block-size,
                    // ignore invalid block size from env vars
                    if let Some(invalid_block_size) = opt_block_size {
                        return Err(Box::new(LsError::BlockSizeParseError(
                            invalid_block_size.clone(),
                        )));
                    }
                    if is_env_var_blocksize {
                        (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
                    } else {
                        (DEFAULT_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
                    }
                }
            }
        } else if env_var_posixly_correct.is_some() {
            if opt_kb {
                (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
            } else {
                (DEFAULT_FILE_SIZE_BLOCK_SIZE, POSIXLY_CORRECT_BLOCK_SIZE)
            }
        } else if opt_si {
            (DEFAULT_FILE_SIZE_BLOCK_SIZE, 1000)
        } else {
            (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
        };

        let long = {
            let author = options.get_flag(options::AUTHOR);
            let group = !options.get_flag(options::NO_GROUP)
                && !options.get_flag(options::format::LONG_NO_GROUP);
            let owner = !options.get_flag(options::format::LONG_NO_OWNER);
            #[cfg(unix)]
            let numeric_uid_gid = options.get_flag(options::format::LONG_NUMERIC_UID_GID);
            LongFormat {
                author,
                group,
                owner,
                #[cfg(unix)]
                numeric_uid_gid,
            }
        };
        let width = parse_width(options.get_one::<String>(options::WIDTH))?;

        #[allow(clippy::needless_bool)]
        let mut show_control = if options.get_flag(options::HIDE_CONTROL_CHARS) {
            false
        } else if options.get_flag(options::SHOW_CONTROL_CHARS) {
            true
        } else {
            !std::io::stdout().is_terminal()
        };

        let mut quoting_style = extract_quoting_style(options, show_control);
        let indicator_style = extract_indicator_style(options);
        // Only parse the value to "--time-style" if it will become relevant.
        let dired = options.get_flag(options::DIRED);
        let time_style = if format == Format::Long || dired {
            parse_time_style(options)?
        } else {
            TimeStyle::Iso
        };

        let mut ignore_patterns: Vec<Pattern> = Vec::new();

        if options.get_flag(options::IGNORE_BACKUPS) {
            ignore_patterns.push(Pattern::new("*~").unwrap());
            ignore_patterns.push(Pattern::new(".*~").unwrap());
        }

        for pattern in options
            .get_many::<String>(options::IGNORE)
            .into_iter()
            .flatten()
        {
            match parse_glob::from_str(pattern) {
                Ok(p) => {
                    ignore_patterns.push(p);
                }
                Err(_) => show_warning!("Invalid pattern for ignore: {}", pattern.quote()),
            }
        }

        if files == Files::Normal {
            for pattern in options
                .get_many::<String>(options::HIDE)
                .into_iter()
                .flatten()
            {
                match parse_glob::from_str(pattern) {
                    Ok(p) => {
                        ignore_patterns.push(p);
                    }
                    Err(_) => show_warning!("Invalid pattern for hide: {}", pattern.quote()),
                }
            }
        }

        // According to ls info page, `--zero` implies the following flags:
        //  - `--show-control-chars`
        //  - `--format=single-column`
        //  - `--color=none`
        //  - `--quoting-style=literal`
        // Current GNU ls implementation allows `--zero` Behavior to be
        // overridden by later flags.
        let zero_formats_opts = [
            options::format::ACROSS,
            options::format::COLUMNS,
            options::format::COMMAS,
            options::format::LONG,
            options::format::LONG_NO_GROUP,
            options::format::LONG_NO_OWNER,
            options::format::LONG_NUMERIC_UID_GID,
            options::format::ONE_LINE,
            options::FORMAT,
        ];
        let zero_colors_opts = [options::COLOR];
        let zero_show_control_opts = [options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS];
        let zero_quoting_style_opts = [
            options::QUOTING_STYLE,
            options::quoting::C,
            options::quoting::ESCAPE,
            options::quoting::LITERAL,
        ];
        let get_last = |flag: &str| -> usize {
            if options.value_source(flag) == Some(clap::parser::ValueSource::CommandLine) {
                options.index_of(flag).unwrap_or(0)
            } else {
                0
            }
        };
        if get_last(options::ZERO)
            > zero_formats_opts
                .into_iter()
                .map(get_last)
                .max()
                .unwrap_or(0)
        {
            format = if format == Format::Long {
                format
            } else {
                Format::OneLine
            };
        }
        if get_last(options::ZERO)
            > zero_colors_opts
                .into_iter()
                .map(get_last)
                .max()
                .unwrap_or(0)
        {
            needs_color = false;
        }
        if get_last(options::ZERO)
            > zero_show_control_opts
                .into_iter()
                .map(get_last)
                .max()
                .unwrap_or(0)
        {
            show_control = true;
        }
        if get_last(options::ZERO)
            > zero_quoting_style_opts
                .into_iter()
                .map(get_last)
                .max()
                .unwrap_or(0)
        {
            quoting_style = QuotingStyle::Literal { show_control };
        }

        let color = if needs_color {
            Some(LsColors::from_env().unwrap_or_default())
        } else {
            None
        };

        if dired || is_dired_arg_present() {
            // --dired implies --format=long
            // if we have --dired --hyperlink, we don't show dired but we still want to see the
            // long format
            format = Format::Long;
        }
        if dired && options.get_flag(options::ZERO) {
            return Err(Box::new(LsError::DiredAndZeroAreIncompatible));
        }

        let dereference = if options.get_flag(options::dereference::ALL) {
            Dereference::All
        } else if options.get_flag(options::dereference::ARGS) {
            Dereference::Args
        } else if options.get_flag(options::dereference::DIR_ARGS) {
            Dereference::DirArgs
        } else if options.get_flag(options::DIRECTORY)
            || indicator_style == IndicatorStyle::Classify
            || format == Format::Long
        {
            Dereference::None
        } else {
            Dereference::DirArgs
        };

        Ok(Self {
            format,
            files,
            sort,
            recursive: options.get_flag(options::RECURSIVE),
            reverse: options.get_flag(options::REVERSE),
            dereference,
            ignore_patterns,
            size_format,
            directory: options.get_flag(options::DIRECTORY),
            time,
            color,
            #[cfg(unix)]
            inode: options.get_flag(options::INODE),
            long,
            alloc_size: options.get_flag(options::size::ALLOCATION_SIZE),
            file_size_block_size,
            block_size,
            width,
            quoting_style,
            indicator_style,
            time_style,
            context,
            selinux_supported: {
                #[cfg(feature = "selinux")]
                {
                    selinux::kernel_support() != selinux::KernelSupport::Unsupported
                }
                #[cfg(not(feature = "selinux"))]
                {
                    false
                }
            },
            group_directories_first: options.get_flag(options::GROUP_DIRECTORIES_FIRST),
            line_ending: LineEnding::from_zero_flag(options.get_flag(options::ZERO)),
            dired,
            hyperlink,
        })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let command = uu_app();

    let matches = match command.try_get_matches_from(args) {
        // clap successfully parsed the arguments:
        Ok(matches) => matches,
        // --help, --version, etc.:
        Err(e) if e.exit_code() == 0 => {
            return Err(e.into());
        }
        // Errors in argument *values* cause exit code 1:
        Err(e) if e.kind() == clap::error::ErrorKind::InvalidValue => {
            return Err(USimpleError::new(1, e.to_string()));
        }
        // All other argument parsing errors cause exit code 2:
        Err(e) => {
            return Err(USimpleError::new(2, e.to_string()));
        }
    };

    let config = Config::from(&matches)?;

    let locs = matches
        .get_many::<OsString>(options::PATHS)
        .map(|v| v.map(Path::new).collect())
        .unwrap_or_else(|| vec![Path::new(".")]);

    list(locs, &config)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        // Format arguments
        .arg(
            Arg::new(options::FORMAT)
                .long(options::FORMAT)
                .help("Set the display format.")
                .value_parser(ShortcutValueParser::new([
                    "long",
                    "verbose",
                    "single-column",
                    "columns",
                    "vertical",
                    "across",
                    "horizontal",
                    "commas",
                ]))
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                    options::DIRED,
                ]),
        )
        .arg(
            Arg::new(options::format::COLUMNS)
                .short('C')
                .help("Display the files in columns.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG)
                .short('l')
                .long(options::format::LONG)
                .help("Display detailed information.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::ACROSS)
                .short('x')
                .help("List entries in rows instead of in columns.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            // silently ignored (see #3624)
            Arg::new(options::format::TAB_SIZE)
                .short('T')
                .long(options::format::TAB_SIZE)
                .env("TABSIZE")
                .value_name("COLS")
                .help("Assume tab stops at each COLS instead of 8 (unimplemented)"),
        )
        .arg(
            Arg::new(options::format::COMMAS)
                .short('m')
                .help("List entries separated by commas.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .overrides_with(options::ZERO)
                .help("List entries separated by ASCII NUL characters.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIRED)
                .long(options::DIRED)
                .short('D')
                .help("generate output designed for Emacs' dired (Directory Editor) mode")
                .action(ArgAction::SetTrue)
                .overrides_with(options::HYPERLINK),
        )
        .arg(
            Arg::new(options::HYPERLINK)
                .long(options::HYPERLINK)
                .help("hyperlink file names WHEN")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("always").alias("yes").alias("force"),
                    PossibleValue::new("auto").alias("tty").alias("if-tty"),
                    PossibleValue::new("never").alias("no").alias("none"),
                ]))
                .require_equals(true)
                .num_args(0..=1)
                .default_missing_value("always")
                .default_value("never")
                .value_name("WHEN")
                .overrides_with(options::DIRED),
        )
        // The next four arguments do not override with the other format
        // options, see the comment in Config::from for the reason.
        // Ideally, they would use Arg::override_with, with their own name
        // but that doesn't seem to work in all cases. Example:
        // ls -1g1
        // even though `ls -11` and `ls -1 -g -1` work.
        .arg(
            Arg::new(options::format::ONE_LINE)
                .short('1')
                .help("List one file per line.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG_NO_GROUP)
                .short('o')
                .help(
                    "Long format without group information. \
                        Identical to --format=long with --no-group.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG_NO_OWNER)
                .short('g')
                .help("Long format without owner information.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG_NUMERIC_UID_GID)
                .short('n')
                .long(options::format::LONG_NUMERIC_UID_GID)
                .help("-l with numeric UIDs and GIDs.")
                .action(ArgAction::SetTrue),
        )
        // Quoting style
        .arg(
            Arg::new(options::QUOTING_STYLE)
                .long(options::QUOTING_STYLE)
                .help("Set quoting style.")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("literal"),
                    PossibleValue::new("shell"),
                    PossibleValue::new("shell-escape"),
                    PossibleValue::new("shell-always"),
                    PossibleValue::new("shell-escape-always"),
                    PossibleValue::new("c").alias("c-maybe"),
                    PossibleValue::new("escape"),
                ]))
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ]),
        )
        .arg(
            Arg::new(options::quoting::LITERAL)
                .short('N')
                .long(options::quoting::LITERAL)
                .alias("l")
                .help("Use literal quoting style. Equivalent to `--quoting-style=literal`")
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::quoting::ESCAPE)
                .short('b')
                .long(options::quoting::ESCAPE)
                .help("Use escape quoting style. Equivalent to `--quoting-style=escape`")
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::quoting::C)
                .short('Q')
                .long(options::quoting::C)
                .help("Use C quoting style. Equivalent to `--quoting-style=c`")
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
                .action(ArgAction::SetTrue),
        )
        // Control characters
        .arg(
            Arg::new(options::HIDE_CONTROL_CHARS)
                .short('q')
                .long(options::HIDE_CONTROL_CHARS)
                .help("Replace control characters with '?' if they are not escaped.")
                .overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_CONTROL_CHARS)
                .long(options::SHOW_CONTROL_CHARS)
                .help("Show control characters 'as is' if they are not escaped.")
                .overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
                .action(ArgAction::SetTrue),
        )
        // Time arguments
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .help(
                    "Show time in <field>:\n\
                        \taccess time (-u): atime, access, use;\n\
                        \tchange time (-t): ctime, status.\n\
                        \tbirth time: birth, creation;",
                )
                .value_name("field")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("atime").alias("access").alias("use"),
                    PossibleValue::new("ctime").alias("status"),
                    PossibleValue::new("birth").alias("creation"),
                ]))
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE]),
        )
        .arg(
            Arg::new(options::time::CHANGE)
                .short('c')
                .help(
                    "If the long listing format (e.g., -l, -o) is being used, print the \
                        status change time (the 'ctime' in the inode) instead of the modification \
                        time. When explicitly sorting by time (--sort=time or -t) or when not \
                        using a long listing format, sort according to the status change time.",
                )
                .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::time::ACCESS)
                .short('u')
                .help(
                    "If the long listing format (e.g., -l, -o) is being used, print the \
                        status access time instead of the modification time. When explicitly \
                        sorting by time (--sort=time or -t) or when not using a long listing \
                        format, sort according to the access time.",
                )
                .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
                .action(ArgAction::SetTrue),
        )
        // Hide and ignore
        .arg(
            Arg::new(options::HIDE)
                .long(options::HIDE)
                .action(ArgAction::Append)
                .value_name("PATTERN")
                .help(
                    "do not list implied entries matching shell PATTERN (overridden by -a or -A)",
                ),
        )
        .arg(
            Arg::new(options::IGNORE)
                .short('I')
                .long(options::IGNORE)
                .action(ArgAction::Append)
                .value_name("PATTERN")
                .help("do not list implied entries matching shell PATTERN"),
        )
        .arg(
            Arg::new(options::IGNORE_BACKUPS)
                .short('B')
                .long(options::IGNORE_BACKUPS)
                .help("Ignore entries which end with ~.")
                .action(ArgAction::SetTrue),
        )
        // Sort arguments
        .arg(
            Arg::new(options::SORT)
                .long(options::SORT)
                .help("Sort by <field>: name, none (-U), time (-t), size (-S), extension (-X) or width")
                .value_name("field")
                .value_parser(ShortcutValueParser::new(["name", "none", "time", "size", "version", "extension", "width"]))
                .require_equals(true)
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        .arg(
            Arg::new(options::sort::SIZE)
                .short('S')
                .help("Sort by file size, largest first.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::TIME)
                .short('t')
                .help("Sort by modification time (the 'mtime' in the inode), newest first.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::VERSION)
                .short('v')
                .help("Natural sort of (version) numbers in the filenames.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::EXTENSION)
                .short('X')
                .help("Sort alphabetically by entry extension.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::NONE)
                .short('U')
                .help(
                    "Do not sort; list the files in whatever order they are stored in the \
                    directory.  This is especially useful when listing very large directories, \
                    since not doing any sorting can be noticeably faster.",
                )
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        // Dereferencing
        .arg(
            Arg::new(options::dereference::ALL)
                .short('L')
                .long(options::dereference::ALL)
                .help(
                    "When showing file information for a symbolic link, show information for the \
                    file the link references rather than the link itself.",
                )
                .overrides_with_all([
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::dereference::DIR_ARGS)
                .long(options::dereference::DIR_ARGS)
                .help(
                    "Do not follow symlinks except when they link to directories and are \
                    given as command line arguments.",
                )
                .overrides_with_all([
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::dereference::ARGS)
                .short('H')
                .long(options::dereference::ARGS)
                .help("Do not follow symlinks except when given as command line arguments.")
                .overrides_with_all([
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
                .action(ArgAction::SetTrue),
        )
        // Long format options
        .arg(
            Arg::new(options::NO_GROUP)
                .long(options::NO_GROUP)
                .short('G')
                .help("Do not show group in long format.")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(options::AUTHOR).long(options::AUTHOR).help(
            "Show author in long format. On the supported platforms, \
            the author always matches the file owner.",
        ).action(ArgAction::SetTrue))
        // Other Flags
        .arg(
            Arg::new(options::files::ALL)
                .short('a')
                .long(options::files::ALL)
                // Overrides -A (as the order matters)
                .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL])
                .help("Do not ignore hidden files (files with names that start with '.').")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::files::ALMOST_ALL)
                .short('A')
                .long(options::files::ALMOST_ALL)
                // Overrides -a (as the order matters)
                .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL])
                .help(
                    "In a directory, do not ignore all file names that start with '.', \
                    only ignore '.' and '..'.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIRECTORY)
                .short('d')
                .long(options::DIRECTORY)
                .help(
                    "Only list the names of directories, rather than listing directory contents. \
                    This will not follow symbolic links unless one of `--dereference-command-line \
                    (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is \
                    specified.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::HUMAN_READABLE)
                .short('h')
                .long(options::size::HUMAN_READABLE)
                .help("Print human readable file sizes (e.g. 1K 234M 56G).")
                .overrides_with_all([options::size::BLOCK_SIZE, options::size::SI])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::KIBIBYTES)
                .short('k')
                .long(options::size::KIBIBYTES)
                .help(
                    "default to 1024-byte blocks for file system usage; used only with -s and per \
                    directory totals",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::SI)
                .long(options::size::SI)
                .help("Print human readable file sizes using powers of 1000 instead of 1024.")
                .overrides_with_all([options::size::BLOCK_SIZE, options::size::HUMAN_READABLE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::BLOCK_SIZE)
                .long(options::size::BLOCK_SIZE)
                .require_equals(true)
                .value_name("BLOCK_SIZE")
                .help("scale sizes by BLOCK_SIZE when printing them")
                .overrides_with_all([options::size::SI, options::size::HUMAN_READABLE]),
        )
        .arg(
            Arg::new(options::INODE)
                .short('i')
                .long(options::INODE)
                .help("print the index number of each file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REVERSE)
                .short('r')
                .long(options::REVERSE)
                .help(
                    "Reverse whatever the sorting method is e.g., list files in reverse \
            alphabetical order, youngest first, smallest first, or whatever.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help("List the contents of all directories recursively.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .long(options::WIDTH)
                .short('w')
                .help("Assume that the terminal is COLS columns wide.")
                .value_name("COLS"),
        )
        .arg(
            Arg::new(options::size::ALLOCATION_SIZE)
                .short('s')
                .long(options::size::ALLOCATION_SIZE)
                .help("print the allocated size of each file, in blocks")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLOR)
                .long(options::COLOR)
                .help("Color output based on file type.")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("always").alias("yes").alias("force"),
                    PossibleValue::new("auto").alias("tty").alias("if-tty"),
                    PossibleValue::new("never").alias("no").alias("none"),
                ]))
                .require_equals(true)
                .num_args(0..=1),
        )
        .arg(
            Arg::new(options::INDICATOR_STYLE)
                .long(options::INDICATOR_STYLE)
                .help(
                    "Append indicator with style WORD to entry names: \
                none (default),  slash (-p), file-type (--file-type), classify (-F)",
                )
                .value_parser(ShortcutValueParser::new(["none", "slash", "file-type", "classify"]))
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            // The --classify flag can take an optional when argument to
            // control its behavior from version 9 of GNU coreutils.
            // There is currently an inconsistency where GNU coreutils allows only
            // the long form of the flag to take the argument while we allow it
            // for both the long and short form of the flag.
            Arg::new(options::indicator_style::CLASSIFY)
                .short('F')
                .long(options::indicator_style::CLASSIFY)
                .help(
                    "Append a character to each file name indicating the file type. Also, for \
                    regular files that are executable, append '*'. The file type indicators are \
                    '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
                    '>' for doors, and nothing for regular files. when may be omitted, or one of:\n\
                        \tnone - Do not classify. This is the default.\n\
                        \tauto - Only classify if standard output is a terminal.\n\
                        \talways - Always classify.\n\
                    Specifying --classify and no when is equivalent to --classify=always. This will \
                    not follow symbolic links listed on the command line unless the \
                    --dereference-command-line (-H), --dereference (-L), or \
                    --dereference-command-line-symlink-to-dir options are specified.",
                )
                .value_name("when")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("always").alias("yes").alias("force"),
                    PossibleValue::new("auto").alias("tty").alias("if-tty"),
                    PossibleValue::new("never").alias("no").alias("none"),
                ]))
                .default_missing_value("always")
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            Arg::new(options::indicator_style::FILE_TYPE)
                .long(options::indicator_style::FILE_TYPE)
                .help("Same as --classify, but do not append '*'")
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::indicator_style::SLASH)
                .short('p')
                .help("Append / indicator to directories.")
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            //This still needs support for posix-*
            Arg::new(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .help("time/date format with -l; see TIME_STYLE below")
                .value_name("TIME_STYLE")
                .env("TIME_STYLE")
                .value_parser(NonEmptyStringValueParser::new())
                .overrides_with_all([options::TIME_STYLE]),
        )
        .arg(
            Arg::new(options::FULL_TIME)
                .long(options::FULL_TIME)
                .overrides_with(options::FULL_TIME)
                .help("like -l --time-style=full-iso")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .short('Z')
                .long(options::CONTEXT)
                .help(CONTEXT_HELP_TEXT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::GROUP_DIRECTORIES_FIRST)
                .long(options::GROUP_DIRECTORIES_FIRST)
                .help(
                    "group directories before files; can be augmented with \
                    a --sort option, but any use of --sort=none (-U) disables grouping",
                )
                .action(ArgAction::SetTrue),
        )
        // Positional arguments
        .arg(
            Arg::new(options::PATHS)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(ValueParser::os_string()),
        )
        .after_help(AFTER_HELP)
}

/// Represents a Path along with it's associated data.
/// Any data that will be reused several times makes sense to be added to this structure.
/// Caching data here helps eliminate redundant syscalls to fetch same information.
#[derive(Debug)]
struct PathData {
    // Result<MetaData> got from symlink_metadata() or metadata() based on config
    md: OnceCell<Option<Metadata>>,
    ft: OnceCell<Option<FileType>>,
    // can be used to avoid reading the metadata. Can be also called d_type:
    // https://www.gnu.org/software/libc/manual/html_node/Directory-Entries.html
    de: Option<DirEntry>,
    // Name of the file - will be empty for . or ..
    display_name: OsString,
    // PathBuf that all above data corresponds to
    p_buf: PathBuf,
    must_dereference: bool,
    security_context: String,
    command_line: bool,
}

impl PathData {
    fn new(
        p_buf: PathBuf,
        dir_entry: Option<std::io::Result<DirEntry>>,
        file_name: Option<OsString>,
        config: &Config,
        command_line: bool,
    ) -> Self {
        // We cannot use `Path::ends_with` or `Path::Components`, because they remove occurrences of '.'
        // For '..', the filename is None
        let display_name = if let Some(name) = file_name {
            name
        } else if command_line {
            p_buf.clone().into()
        } else {
            p_buf
                .file_name()
                .unwrap_or_else(|| p_buf.iter().next_back().unwrap())
                .to_owned()
        };
        let must_dereference = match &config.dereference {
            Dereference::All => true,
            Dereference::Args => command_line,
            Dereference::DirArgs => {
                if command_line {
                    if let Ok(md) = p_buf.metadata() {
                        md.is_dir()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Dereference::None => false,
        };

        let de: Option<DirEntry> = match dir_entry {
            Some(de) => de.ok(),
            None => None,
        };

        // Why prefer to check the DirEntry file_type()?  B/c the call is
        // nearly free compared to a metadata() call on a Path
        fn get_file_type(
            de: &DirEntry,
            p_buf: &Path,
            must_dereference: bool,
        ) -> OnceCell<Option<FileType>> {
            if must_dereference {
                if let Ok(md_pb) = p_buf.metadata() {
                    return OnceCell::from(Some(md_pb.file_type()));
                }
            }
            if let Ok(ft_de) = de.file_type() {
                OnceCell::from(Some(ft_de))
            } else if let Ok(md_pb) = p_buf.symlink_metadata() {
                OnceCell::from(Some(md_pb.file_type()))
            } else {
                OnceCell::new()
            }
        }
        let ft = match de {
            Some(ref de) => get_file_type(de, &p_buf, must_dereference),
            None => OnceCell::new(),
        };

        let security_context = if config.context {
            get_security_context(config, &p_buf, must_dereference)
        } else {
            String::new()
        };

        Self {
            md: OnceCell::new(),
            ft,
            de,
            display_name,
            p_buf,
            must_dereference,
            security_context,
            command_line,
        }
    }

    fn get_metadata(&self, out: &mut BufWriter<Stdout>) -> Option<&Metadata> {
        self.md
            .get_or_init(|| {
                // check if we can use DirEntry metadata
                // it will avoid a call to stat()
                if !self.must_dereference {
                    if let Some(dir_entry) = &self.de {
                        return dir_entry.metadata().ok();
                    }
                }

                // if not, check if we can use Path metadata
                match get_metadata_with_deref_opt(self.p_buf.as_path(), self.must_dereference) {
                    Err(err) => {
                        // FIXME: A bit tricky to propagate the result here
                        out.flush().unwrap();
                        let errno = err.raw_os_error().unwrap_or(1i32);
                        // a bad fd will throw an error when dereferenced,
                        // but GNU will not throw an error until a bad fd "dir"
                        // is entered, here we match that GNU behavior, by handing
                        // back the non-dereferenced metadata upon an EBADF
                        if self.must_dereference && errno == 9i32 {
                            if let Some(dir_entry) = &self.de {
                                return dir_entry.metadata().ok();
                            }
                        }
                        show!(LsError::IOErrorContext(
                            err,
                            self.p_buf.clone(),
                            self.command_line
                        ));
                        None
                    }
                    Ok(md) => Some(md),
                }
            })
            .as_ref()
    }

    fn file_type(&self, out: &mut BufWriter<Stdout>) -> Option<&FileType> {
        self.ft
            .get_or_init(|| self.get_metadata(out).map(|md| md.file_type()))
            .as_ref()
    }
}

fn show_dir_name(path_data: &PathData, out: &mut BufWriter<Stdout>, config: &Config) {
    if config.hyperlink && !config.dired {
        let name = escape_name(path_data.p_buf.as_os_str(), &config.quoting_style);
        let hyperlink = create_hyperlink(&name, path_data);
        write!(out, "{}:", hyperlink).unwrap();
    } else {
        write!(out, "{}:", path_data.p_buf.display()).unwrap();
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn list(locs: Vec<&Path>, config: &Config) -> UResult<()> {
    let mut files = Vec::<PathData>::new();
    let mut dirs = Vec::<PathData>::new();
    let mut out = BufWriter::new(stdout());
    let mut dired = DiredOutput::default();
    let mut style_manager = config.color.as_ref().map(StyleManager::new);
    let initial_locs_len = locs.len();

    for loc in locs {
        let path_data = PathData::new(PathBuf::from(loc), None, None, config, true);

        // Getting metadata here is no big deal as it's just the CWD
        // and we really just want to know if the strings exist as files/dirs
        //
        // Proper GNU handling is don't show if dereferenced symlink DNE
        // but only for the base dir, for a child dir show, and print ?s
        // in long format
        if path_data.get_metadata(&mut out).is_none() {
            continue;
        }

        let show_dir_contents = match path_data.file_type(&mut out) {
            Some(ft) => !config.directory && ft.is_dir(),
            None => {
                set_exit_code(1);
                false
            }
        };

        if show_dir_contents {
            dirs.push(path_data);
        } else {
            files.push(path_data);
        }
    }

    sort_entries(&mut files, config, &mut out);
    sort_entries(&mut dirs, config, &mut out);

    if let Some(style_manager) = style_manager.as_mut() {
        // ls will try to write a reset before anything is written if normal
        // color is given
        if style_manager.get_normal_style().is_some() {
            let to_write = style_manager.reset(true);
            write!(out, "{}", to_write)?;
        }
    }

    display_items(&files, config, &mut out, &mut dired, &mut style_manager)?;

    for (pos, path_data) in dirs.iter().enumerate() {
        // Do read_dir call here to match GNU semantics by printing
        // read_dir errors before directory headings, names and totals
        let read_dir = match fs::read_dir(&path_data.p_buf) {
            Err(err) => {
                // flush stdout buffer before the error to preserve formatting and order
                out.flush()?;
                show!(LsError::IOErrorContext(
                    err,
                    path_data.p_buf.clone(),
                    path_data.command_line
                ));
                continue;
            }
            Ok(rd) => rd,
        };

        // Print dir heading - name... 'total' comes after error display
        if initial_locs_len > 1 || config.recursive {
            if pos.eq(&0usize) && files.is_empty() {
                if config.dired {
                    dired::indent(&mut out)?;
                }
                show_dir_name(path_data, &mut out, config);
                writeln!(out)?;
                if config.dired {
                    // First directory displayed
                    let dir_len = path_data.display_name.len();
                    // add the //SUBDIRED// coordinates
                    dired::calculate_subdired(&mut dired, dir_len);
                    // Add the padding for the dir name
                    dired::add_dir_name(&mut dired, dir_len);
                }
            } else {
                writeln!(out)?;
                show_dir_name(path_data, &mut out, config);
                writeln!(out)?;
            }
        }
        let mut listed_ancestors = HashSet::new();
        listed_ancestors.insert(FileInformation::from_path(
            &path_data.p_buf,
            path_data.must_dereference,
        )?);
        enter_directory(
            path_data,
            read_dir,
            config,
            &mut out,
            &mut listed_ancestors,
            &mut dired,
            &mut style_manager,
        )?;
    }
    if config.dired && !config.hyperlink {
        dired::print_dired_output(config, &dired, &mut out)?;
    }
    Ok(())
}

fn sort_entries(entries: &mut [PathData], config: &Config, out: &mut BufWriter<Stdout>) {
    match config.sort {
        Sort::Time => entries.sort_by_key(|k| {
            Reverse(
                k.get_metadata(out)
                    .and_then(|md| get_system_time(md, config))
                    .unwrap_or(UNIX_EPOCH),
            )
        }),
        Sort::Size => {
            entries.sort_by_key(|k| Reverse(k.get_metadata(out).map(|md| md.len()).unwrap_or(0)));
        }
        // The default sort in GNU ls is case insensitive
        Sort::Name => entries.sort_by(|a, b| a.display_name.cmp(&b.display_name)),
        Sort::Version => entries.sort_by(|a, b| {
            version_cmp(&a.p_buf.to_string_lossy(), &b.p_buf.to_string_lossy())
                .then(a.p_buf.to_string_lossy().cmp(&b.p_buf.to_string_lossy()))
        }),
        Sort::Extension => entries.sort_by(|a, b| {
            a.p_buf
                .extension()
                .cmp(&b.p_buf.extension())
                .then(a.p_buf.file_stem().cmp(&b.p_buf.file_stem()))
        }),
        Sort::Width => entries.sort_by(|a, b| {
            a.display_name
                .len()
                .cmp(&b.display_name.len())
                .then(a.display_name.cmp(&b.display_name))
        }),
        Sort::None => {}
    }

    if config.reverse {
        entries.reverse();
    }

    if config.group_directories_first && config.sort != Sort::None {
        entries.sort_by_key(|p| {
            let md = {
                // We will always try to deref symlinks to group directories, so PathData.md
                // is not always useful.
                if p.must_dereference {
                    p.md.get()
                } else {
                    None
                }
            };

            !match md {
                None | Some(None) => {
                    // If it metadata cannot be determined, treat as a file.
                    get_metadata_with_deref_opt(p.p_buf.as_path(), true)
                        .map_or_else(|_| false, |m| m.is_dir())
                }
                Some(Some(m)) => m.is_dir(),
            }
        });
    }
}

fn is_hidden(file_path: &DirEntry) -> bool {
    #[cfg(windows)]
    {
        let metadata = file_path.metadata().unwrap();
        let attr = metadata.file_attributes();
        (attr & 0x2) > 0
    }
    #[cfg(not(windows))]
    {
        file_path
            .file_name()
            .to_str()
            .map(|res| res.starts_with('.'))
            .unwrap_or(false)
    }
}

fn should_display(entry: &DirEntry, config: &Config) -> bool {
    // check if hidden
    if config.files == Files::Normal && is_hidden(entry) {
        return false;
    }

    // check if it is among ignore_patterns
    let options = MatchOptions {
        // setting require_literal_leading_dot to match behavior in GNU ls
        require_literal_leading_dot: true,
        require_literal_separator: false,
        case_sensitive: true,
    };
    let file_name = entry.file_name();
    // If the decoding fails, still show an incorrect rendering
    let file_name = match file_name.to_str() {
        Some(s) => s.to_string(),
        None => file_name.to_string_lossy().into_owned(),
    };
    !config
        .ignore_patterns
        .iter()
        .any(|p| p.matches_with(&file_name, options))
}

#[allow(clippy::cognitive_complexity)]
fn enter_directory(
    path_data: &PathData,
    read_dir: ReadDir,
    config: &Config,
    out: &mut BufWriter<Stdout>,
    listed_ancestors: &mut HashSet<FileInformation>,
    dired: &mut DiredOutput,
    style_manager: &mut Option<StyleManager>,
) -> UResult<()> {
    // Create vec of entries with initial dot files
    let mut entries: Vec<PathData> = if config.files == Files::All {
        vec![
            PathData::new(
                path_data.p_buf.clone(),
                None,
                Some(".".into()),
                config,
                false,
            ),
            PathData::new(
                path_data.p_buf.join(".."),
                None,
                Some("..".into()),
                config,
                false,
            ),
        ]
    } else {
        vec![]
    };

    // Convert those entries to the PathData struct
    for raw_entry in read_dir {
        let dir_entry = match raw_entry {
            Ok(path) => path,
            Err(err) => {
                out.flush()?;
                show!(LsError::IOError(err));
                continue;
            }
        };

        if should_display(&dir_entry, config) {
            let entry_path_data =
                PathData::new(dir_entry.path(), Some(Ok(dir_entry)), None, config, false);
            entries.push(entry_path_data);
        };
    }

    sort_entries(&mut entries, config, out);

    // Print total after any error display
    if config.format == Format::Long || config.alloc_size {
        let total = return_total(&entries, config, out)?;
        write!(out, "{}", total.as_str())?;
        if config.dired {
            dired::add_total(dired, total.len());
        }
    }

    display_items(&entries, config, out, dired, style_manager)?;

    if config.recursive {
        for e in entries
            .iter()
            .skip(if config.files == Files::All { 2 } else { 0 })
            .filter(|p| p.ft.get().is_some())
            .filter(|p| p.ft.get().unwrap().is_some())
            .filter(|p| p.ft.get().unwrap().unwrap().is_dir())
        {
            match fs::read_dir(&e.p_buf) {
                Err(err) => {
                    out.flush()?;
                    show!(LsError::IOErrorContext(
                        err,
                        e.p_buf.clone(),
                        e.command_line
                    ));
                    continue;
                }
                Ok(rd) => {
                    if listed_ancestors
                        .insert(FileInformation::from_path(&e.p_buf, e.must_dereference)?)
                    {
                        // when listing several directories in recursive mode, we show
                        // "dirname:" at the beginning of the file list
                        writeln!(out)?;
                        if config.dired {
                            // We already injected the first dir
                            // Continue with the others
                            // 2 = \n + \n
                            dired.padding = 2;
                            dired::indent(out)?;
                            let dir_name_size = e.p_buf.to_string_lossy().len();
                            dired::calculate_subdired(dired, dir_name_size);
                            // inject dir name
                            dired::add_dir_name(dired, dir_name_size);
                        }

                        show_dir_name(e, out, config);
                        writeln!(out)?;
                        enter_directory(
                            e,
                            rd,
                            config,
                            out,
                            listed_ancestors,
                            dired,
                            style_manager,
                        )?;
                        listed_ancestors
                            .remove(&FileInformation::from_path(&e.p_buf, e.must_dereference)?);
                    } else {
                        out.flush()?;
                        show!(LsError::AlreadyListedError(e.p_buf.clone()));
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_metadata_with_deref_opt(p_buf: &Path, dereference: bool) -> std::io::Result<Metadata> {
    if dereference {
        p_buf.metadata()
    } else {
        p_buf.symlink_metadata()
    }
}

fn display_dir_entry_size(
    entry: &PathData,
    config: &Config,
    out: &mut BufWriter<std::io::Stdout>,
) -> (usize, usize, usize, usize, usize, usize) {
    // TODO: Cache/memorize the display_* results so we don't have to recalculate them.
    if let Some(md) = entry.get_metadata(out) {
        let (size_len, major_len, minor_len) = match display_len_or_rdev(md, config) {
            SizeOrDeviceId::Device(major, minor) => (
                (major.len() + minor.len() + 2usize),
                major.len(),
                minor.len(),
            ),
            SizeOrDeviceId::Size(size) => (size.len(), 0usize, 0usize),
        };
        (
            display_symlink_count(md).len(),
            display_uname(md, config).len(),
            display_group(md, config).len(),
            size_len,
            major_len,
            minor_len,
        )
    } else {
        (0, 0, 0, 0, 0, 0)
    }
}

fn pad_left(string: &str, count: usize) -> String {
    format!("{string:>count$}")
}

fn pad_right(string: &str, count: usize) -> String {
    format!("{string:<count$}")
}

fn return_total(
    items: &[PathData],
    config: &Config,
    out: &mut BufWriter<Stdout>,
) -> UResult<String> {
    let mut total_size = 0;
    for item in items {
        total_size += item
            .get_metadata(out)
            .as_ref()
            .map_or(0, |md| get_block_size(md, config));
    }
    if config.dired {
        dired::indent(out)?;
    }
    Ok(format!(
        "total {}{}",
        display_size(total_size, config),
        config.line_ending
    ))
}

fn display_additional_leading_info(
    item: &PathData,
    padding: &PaddingCollection,
    config: &Config,
    out: &mut BufWriter<Stdout>,
) -> UResult<String> {
    let mut result = String::new();
    #[cfg(unix)]
    {
        if config.inode {
            let i = if let Some(md) = item.get_metadata(out) {
                get_inode(md)
            } else {
                "?".to_owned()
            };
            write!(result, "{} ", pad_left(&i, padding.inode)).unwrap();
        }
    }

    if config.alloc_size {
        let s = if let Some(md) = item.get_metadata(out) {
            display_size(get_block_size(md, config), config)
        } else {
            "?".to_owned()
        };
        // extra space is insert to align the sizes, as needed for all formats, except for the comma format.
        if config.format == Format::Commas {
            write!(result, "{s} ").unwrap();
        } else {
            write!(result, "{} ", pad_left(&s, padding.block_size)).unwrap();
        };
    }
    Ok(result)
}

#[allow(clippy::cognitive_complexity)]
fn display_items(
    items: &[PathData],
    config: &Config,
    out: &mut BufWriter<Stdout>,
    dired: &mut DiredOutput,
    style_manager: &mut Option<StyleManager>,
) -> UResult<()> {
    // `-Z`, `--context`:
    // Display the SELinux security context or '?' if none is found. When used with the `-l`
    // option, print the security context to the left of the size column.

    let quoted = items.iter().any(|item| {
        let name = escape_name(&item.display_name, &config.quoting_style);
        name.starts_with('\'')
    });

    if config.format == Format::Long {
        let padding_collection = calculate_padding_collection(items, config, out);

        for item in items {
            #[cfg(unix)]
            if config.inode || config.alloc_size {
                let more_info =
                    display_additional_leading_info(item, &padding_collection, config, out)?;

                write!(out, "{more_info}")?;
            }
            #[cfg(not(unix))]
            if config.alloc_size {
                let more_info =
                    display_additional_leading_info(item, &padding_collection, config, out)?;
                write!(out, "{more_info}")?;
            }
            display_item_long(
                item,
                &padding_collection,
                config,
                out,
                dired,
                style_manager,
                quoted,
            )?;
        }
    } else {
        let mut longest_context_len = 1;
        let prefix_context = if config.context {
            for item in items {
                let context_len = item.security_context.len();
                longest_context_len = context_len.max(longest_context_len);
            }
            Some(longest_context_len)
        } else {
            None
        };

        let padding = calculate_padding_collection(items, config, out);

        // we need to apply normal color to non filename output
        if let Some(style_manager) = style_manager {
            write!(out, "{}", style_manager.apply_normal())?;
        }

        let mut names_vec = Vec::new();
        for i in items {
            let more_info = display_additional_leading_info(i, &padding, config, out)?;
            // it's okay to set current column to zero which is used to decide
            // whether text will wrap or not, because when format is grid or
            // column ls will try to place the item name in a new line if it
            // wraps.
            let cell =
                display_item_name(i, config, prefix_context, more_info, out, style_manager, 0);

            names_vec.push(cell);
        }

        let mut names = names_vec.into_iter();

        match config.format {
            Format::Columns => {
                display_grid(names, config.width, Direction::TopToBottom, out, quoted)?;
            }
            Format::Across => {
                display_grid(names, config.width, Direction::LeftToRight, out, quoted)?;
            }
            Format::Commas => {
                let mut current_col = 0;
                if let Some(name) = names.next() {
                    write!(out, "{}", name)?;
                    current_col = ansi_width(&name) as u16 + 2;
                }
                for name in names {
                    let name_width = ansi_width(&name) as u16;
                    // If the width is 0 we print one single line
                    if config.width != 0 && current_col + name_width + 1 > config.width {
                        current_col = name_width + 2;
                        write!(out, ",\n{}", name)?;
                    } else {
                        current_col += name_width + 2;
                        write!(out, ", {}", name)?;
                    }
                }
                // Current col is never zero again if names have been printed.
                // So we print a newline.
                if current_col > 0 {
                    write!(out, "{}", config.line_ending)?;
                }
            }
            _ => {
                for name in names {
                    write!(out, "{}{}", name, config.line_ending)?;
                }
            }
        };
    }

    Ok(())
}

#[allow(unused_variables)]
fn get_block_size(md: &Metadata, config: &Config) -> u64 {
    /* GNU ls will display sizes in terms of block size
       md.len() will differ from this value when the file has some holes
    */
    #[cfg(unix)]
    {
        let raw_blocks = if md.file_type().is_char_device() || md.file_type().is_block_device() {
            0u64
        } else {
            md.blocks() * 512
        };
        match config.size_format {
            SizeFormat::Binary | SizeFormat::Decimal => raw_blocks,
            SizeFormat::Bytes => raw_blocks / config.block_size,
        }
    }
    #[cfg(not(unix))]
    {
        // no way to get block size for windows, fall-back to file size
        md.len()
    }
}

fn display_grid(
    names: impl Iterator<Item = String>,
    width: u16,
    direction: Direction,
    out: &mut BufWriter<Stdout>,
    quoted: bool,
) -> UResult<()> {
    if width == 0 {
        // If the width is 0 we print one single line
        let mut printed_something = false;
        for name in names {
            if printed_something {
                write!(out, "  ")?;
            }
            printed_something = true;
            write!(out, "{name}")?;
        }
        if printed_something {
            writeln!(out)?;
        }
    } else {
        let names: Vec<String> = if quoted {
            // In case some names are quoted, GNU adds a space before each
            // entry that does not start with a quote to make it prettier
            // on multiline.
            //
            // Example:
            // ```
            // $ ls
            // 'a\nb'   bar
            //  foo     baz
            // ^       ^
            // These spaces is added
            // ```
            names
                .map(|n| {
                    if n.starts_with('\'') || n.starts_with('"') {
                        n
                    } else {
                        format!(" {n}")
                    }
                })
                .collect()
        } else {
            names.collect()
        };

        // Determine whether to use tabs for separation based on whether any entry ends with '/'.
        // If any entry ends with '/', it indicates that the -F flag is likely used to classify directories.
        let use_tabs = names.iter().any(|name| name.ends_with('/'));

        let filling = if use_tabs {
            Filling::Text("\t".to_string())
        } else {
            Filling::Spaces(2)
        };

        let grid = Grid::new(
            names,
            GridOptions {
                filling,
                direction,
                width: width as usize,
            },
        );
        write!(out, "{grid}")?;
    }
    Ok(())
}

/// This writes to the BufWriter out a single string of the output of `ls -l`.
///
/// It writes the following keys, in order:
/// * `inode` ([`get_inode`], config-optional)
/// * `permissions` ([`display_permissions`])
/// * `symlink_count` ([`display_symlink_count`])
/// * `owner` ([`display_uname`], config-optional)
/// * `group` ([`display_group`], config-optional)
/// * `author` ([`display_uname`], config-optional)
/// * `size / rdev` ([`display_len_or_rdev`])
/// * `system_time` ([`get_system_time`])
/// * `item_name` ([`display_item_name`])
///
/// This function needs to display information in columns:
/// * permissions and system_time are already guaranteed to be pre-formatted in fixed length.
/// * item_name is the last column and is left-aligned.
/// * Everything else needs to be padded using [`pad_left`].
///
/// That's why we have the parameters:
/// ```txt
///    longest_link_count_len: usize,
///    longest_uname_len: usize,
///    longest_group_len: usize,
///    longest_context_len: usize,
///    longest_size_len: usize,
/// ```
/// that decide the maximum possible character count of each field.
#[allow(clippy::write_literal)]
#[allow(clippy::cognitive_complexity)]
fn display_item_long(
    item: &PathData,
    padding: &PaddingCollection,
    config: &Config,
    out: &mut BufWriter<Stdout>,
    dired: &mut DiredOutput,
    style_manager: &mut Option<StyleManager>,
    quoted: bool,
) -> UResult<()> {
    let mut output_display: String = String::new();

    // apply normal color to non filename outputs
    if let Some(style_manager) = style_manager {
        write!(output_display, "{}", style_manager.apply_normal()).unwrap();
    }
    if config.dired {
        output_display += "  ";
    }
    if let Some(md) = item.get_metadata(out) {
        #[cfg(any(not(unix), target_os = "android", target_os = "macos"))]
        // TODO: See how Mac should work here
        let is_acl_set = false;
        #[cfg(all(unix, not(any(target_os = "android", target_os = "macos"))))]
        let is_acl_set = has_acl(item.display_name.as_os_str());
        write!(
            output_display,
            "{}{}{} {}",
            display_permissions(md, true),
            if item.security_context.len() > 1 {
                // GNU `ls` uses a "." character to indicate a file with a security context,
                // but not other alternate access method.
                "."
            } else {
                ""
            },
            if is_acl_set {
                // if acl has been set, we display a "+" at the end of the file permissions
                "+"
            } else {
                ""
            },
            pad_left(&display_symlink_count(md), padding.link_count)
        )
        .unwrap();

        if config.long.owner {
            write!(
                output_display,
                " {}",
                pad_right(&display_uname(md, config), padding.uname)
            )
            .unwrap();
        }

        if config.long.group {
            write!(
                output_display,
                " {}",
                pad_right(&display_group(md, config), padding.group)
            )
            .unwrap();
        }

        if config.context {
            write!(
                output_display,
                " {}",
                pad_right(&item.security_context, padding.context)
            )
            .unwrap();
        }

        // Author is only different from owner on GNU/Hurd, so we reuse
        // the owner, since GNU/Hurd is not currently supported by Rust.
        if config.long.author {
            write!(
                output_display,
                " {}",
                pad_right(&display_uname(md, config), padding.uname)
            )
            .unwrap();
        }

        match display_len_or_rdev(md, config) {
            SizeOrDeviceId::Size(size) => {
                write!(output_display, " {}", pad_left(&size, padding.size)).unwrap();
            }
            SizeOrDeviceId::Device(major, minor) => {
                write!(
                    output_display,
                    " {}, {}",
                    pad_left(
                        &major,
                        #[cfg(not(unix))]
                        0usize,
                        #[cfg(unix)]
                        padding.major.max(
                            padding
                                .size
                                .saturating_sub(padding.minor.saturating_add(2usize))
                        ),
                    ),
                    pad_left(
                        &minor,
                        #[cfg(not(unix))]
                        0usize,
                        #[cfg(unix)]
                        padding.minor,
                    ),
                )
                .unwrap();
            }
        };

        write!(output_display, " {} ", display_date(md, config)).unwrap();

        let item_name = display_item_name(
            item,
            config,
            None,
            String::new(),
            out,
            style_manager,
            ansi_width(&output_display),
        );

        let displayed_item = if quoted && !item_name.starts_with('\'') {
            format!(" {}", item_name)
        } else {
            item_name
        };

        if config.dired {
            let (start, end) = dired::calculate_dired(
                &dired.dired_positions,
                output_display.len(),
                displayed_item.len(),
            );
            dired::update_positions(dired, start, end);
        }
        write!(output_display, "{}{}", displayed_item, config.line_ending).unwrap();
    } else {
        #[cfg(unix)]
        let leading_char = {
            if let Some(Some(ft)) = item.ft.get() {
                if ft.is_char_device() {
                    "c"
                } else if ft.is_block_device() {
                    "b"
                } else if ft.is_symlink() {
                    "l"
                } else if ft.is_dir() {
                    "d"
                } else {
                    "-"
                }
            } else {
                "-"
            }
        };
        #[cfg(not(unix))]
        let leading_char = {
            if let Some(Some(ft)) = item.ft.get() {
                if ft.is_symlink() {
                    "l"
                } else if ft.is_dir() {
                    "d"
                } else {
                    "-"
                }
            } else {
                "-"
            }
        };

        write!(
            output_display,
            "{}{} {}",
            format_args!("{leading_char}?????????"),
            if item.security_context.len() > 1 {
                // GNU `ls` uses a "." character to indicate a file with a security context,
                // but not other alternate access method.
                "."
            } else {
                ""
            },
            pad_left("?", padding.link_count)
        )
        .unwrap();

        if config.long.owner {
            write!(output_display, " {}", pad_right("?", padding.uname)).unwrap();
        }

        if config.long.group {
            write!(output_display, " {}", pad_right("?", padding.group)).unwrap();
        }

        if config.context {
            write!(
                output_display,
                " {}",
                pad_right(&item.security_context, padding.context)
            )
            .unwrap();
        }

        // Author is only different from owner on GNU/Hurd, so we reuse
        // the owner, since GNU/Hurd is not currently supported by Rust.
        if config.long.author {
            write!(output_display, " {}", pad_right("?", padding.uname)).unwrap();
        }

        let displayed_item = display_item_name(
            item,
            config,
            None,
            String::new(),
            out,
            style_manager,
            ansi_width(&output_display),
        );
        let date_len = 12;

        write!(
            output_display,
            " {} {} ",
            pad_left("?", padding.size),
            pad_left("?", date_len),
        )
        .unwrap();

        if config.dired {
            dired::calculate_and_update_positions(
                dired,
                output_display.len(),
                displayed_item.trim().len(),
            );
        }
        write!(output_display, "{}{}", displayed_item, config.line_ending).unwrap();
    }
    write!(out, "{}", output_display)?;

    Ok(())
}

#[cfg(unix)]
fn get_inode(metadata: &Metadata) -> String {
    format!("{}", metadata.ino())
}

// Currently getpwuid is `linux` target only. If it's broken out into
// a posix-compliant attribute this can be updated...
#[cfg(unix)]
use once_cell::sync::Lazy;
#[cfg(unix)]
use std::sync::Mutex;
#[cfg(unix)]
use uucore::entries;
use uucore::fs::FileInformation;
use uucore::quoting_style;

#[cfg(unix)]
fn cached_uid2usr(uid: u32) -> String {
    static UID_CACHE: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

    let mut uid_cache = UID_CACHE.lock().unwrap();
    uid_cache
        .entry(uid)
        .or_insert_with(|| entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()))
        .clone()
}

#[cfg(unix)]
fn display_uname(metadata: &Metadata, config: &Config) -> String {
    if config.long.numeric_uid_gid {
        metadata.uid().to_string()
    } else {
        cached_uid2usr(metadata.uid())
    }
}

#[cfg(all(unix, not(target_os = "redox")))]
fn cached_gid2grp(gid: u32) -> String {
    static GID_CACHE: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

    let mut gid_cache = GID_CACHE.lock().unwrap();
    gid_cache
        .entry(gid)
        .or_insert_with(|| entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()))
        .clone()
}

#[cfg(all(unix, not(target_os = "redox")))]
fn display_group(metadata: &Metadata, config: &Config) -> String {
    if config.long.numeric_uid_gid {
        metadata.gid().to_string()
    } else {
        cached_gid2grp(metadata.gid())
    }
}

#[cfg(target_os = "redox")]
fn display_group(metadata: &Metadata, _config: &Config) -> String {
    metadata.gid().to_string()
}

#[cfg(not(unix))]
fn display_uname(_metadata: &Metadata, _config: &Config) -> String {
    "somebody".to_string()
}

#[cfg(not(unix))]
fn display_group(_metadata: &Metadata, _config: &Config) -> String {
    "somegroup".to_string()
}

// The implementations for get_time are separated because some options, such
// as ctime will not be available
#[cfg(unix)]
fn get_system_time(md: &Metadata, config: &Config) -> Option<SystemTime> {
    match config.time {
        Time::Change => Some(UNIX_EPOCH + Duration::new(md.ctime() as u64, md.ctime_nsec() as u32)),
        Time::Modification => md.modified().ok(),
        Time::Access => md.accessed().ok(),
        Time::Birth => md.created().ok(),
    }
}

#[cfg(not(unix))]
fn get_system_time(md: &Metadata, config: &Config) -> Option<SystemTime> {
    match config.time {
        Time::Modification => md.modified().ok(),
        Time::Access => md.accessed().ok(),
        Time::Birth => md.created().ok(),
        _ => None,
    }
}

fn get_time(md: &Metadata, config: &Config) -> Option<chrono::DateTime<chrono::Local>> {
    let time = get_system_time(md, config)?;
    Some(time.into())
}

fn display_date(metadata: &Metadata, config: &Config) -> String {
    match get_time(metadata, config) {
        Some(time) => {
            //Date is recent if from past 6 months
            //According to GNU a Gregorian year has 365.2425 * 24 * 60 * 60 == 31556952 seconds on the average.
            let recent = time + chrono::TimeDelta::try_seconds(31_556_952 / 2).unwrap()
                > chrono::Local::now();

            match &config.time_style {
                TimeStyle::FullIso => time.format("%Y-%m-%d %H:%M:%S.%f %z"),
                TimeStyle::LongIso => time.format("%Y-%m-%d %H:%M"),
                TimeStyle::Iso => time.format(if recent { "%m-%d %H:%M" } else { "%Y-%m-%d " }),
                TimeStyle::Locale => {
                    let fmt = if recent { "%b %e %H:%M" } else { "%b %e  %Y" };

                    // spell-checker:ignore (word) datetime
                    //In this version of chrono translating can be done
                    //The function is chrono::datetime::DateTime::format_localized
                    //However it's currently still hard to get the current pure-rust-locale
                    //So it's not yet implemented

                    time.format(fmt)
                }
                TimeStyle::Format(e) => time.format(e),
            }
            .to_string()
        }
        None => "???".into(),
    }
}

#[allow(dead_code)]
enum SizeOrDeviceId {
    Size(String),
    Device(String, String),
}

fn display_len_or_rdev(metadata: &Metadata, config: &Config) -> SizeOrDeviceId {
    #[cfg(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "android",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    {
        let ft = metadata.file_type();
        if ft.is_char_device() || ft.is_block_device() {
            // A type cast is needed here as the `dev_t` type varies across OSes.
            let dev = metadata.rdev() as dev_t;
            let major = unsafe { major(dev) };
            let minor = unsafe { minor(dev) };
            return SizeOrDeviceId::Device(major.to_string(), minor.to_string());
        }
    }
    let len_adjusted = {
        let d = metadata.len() / config.file_size_block_size;
        let r = metadata.len() % config.file_size_block_size;
        if r == 0 {
            d
        } else {
            d + 1
        }
    };
    SizeOrDeviceId::Size(display_size(len_adjusted, config))
}

fn display_size(size: u64, config: &Config) -> String {
    human_readable(size, config.size_format)
}

#[cfg(unix)]
fn file_is_executable(md: &Metadata) -> bool {
    // Mode always returns u32, but the flags might not be, based on the platform
    // e.g. linux has u32, mac has u16.
    // S_IXUSR -> user has execute permission
    // S_IXGRP -> group has execute permission
    // S_IXOTH -> other users have execute permission
    #[allow(clippy::unnecessary_cast)]
    return md.mode() & ((S_IXUSR | S_IXGRP | S_IXOTH) as u32) != 0;
}

fn classify_file(path: &PathData, out: &mut BufWriter<Stdout>) -> Option<char> {
    let file_type = path.file_type(out)?;

    if file_type.is_dir() {
        Some('/')
    } else if file_type.is_symlink() {
        Some('@')
    } else {
        #[cfg(unix)]
        {
            if file_type.is_socket() {
                Some('=')
            } else if file_type.is_fifo() {
                Some('|')
            } else if file_type.is_file()
                // Safe unwrapping if the file was removed between listing and display
                // See https://github.com/uutils/coreutils/issues/5371
                && path.get_metadata(out).map(file_is_executable).unwrap_or_default()
            {
                Some('*')
            } else {
                None
            }
        }
        #[cfg(not(unix))]
        None
    }
}

/// Takes a [`PathData`] struct and returns a cell with a name ready for displaying.
///
/// This function relies on the following parameters in the provided `&Config`:
/// * `config.quoting_style` to decide how we will escape `name` using [`escape_name`].
/// * `config.inode` decides whether to display inode numbers beside names using [`get_inode`].
/// * `config.color` decides whether it's going to color `name` using [`color_name`].
/// * `config.indicator_style` to append specific characters to `name` using [`classify_file`].
/// * `config.format` to display symlink targets if `Format::Long`. This function is also
///   responsible for coloring symlink target names if `config.color` is specified.
/// * `config.context` to prepend security context to `name` if compiled with `feat_selinux`.
/// * `config.hyperlink` decides whether to hyperlink the item
///
/// Note that non-unicode sequences in symlink targets are dealt with using
/// [`std::path::Path::to_string_lossy`].
#[allow(clippy::cognitive_complexity)]
fn display_item_name(
    path: &PathData,
    config: &Config,
    prefix_context: Option<usize>,
    more_info: String,
    out: &mut BufWriter<Stdout>,
    style_manager: &mut Option<StyleManager>,
    current_column: usize,
) -> String {
    // This is our return value. We start by `&path.display_name` and modify it along the way.
    let mut name = escape_name(&path.display_name, &config.quoting_style);

    let is_wrap =
        |namelen: usize| config.width != 0 && current_column + namelen > config.width.into();

    if config.hyperlink {
        name = create_hyperlink(&name, path);
    }

    if let Some(style_manager) = style_manager {
        name = color_name(&name, path, style_manager, out, None, is_wrap(name.len()));
    }

    if config.format != Format::Long && !more_info.is_empty() {
        name = more_info + &name;
    }

    if config.indicator_style != IndicatorStyle::None {
        let sym = classify_file(path, out);

        let char_opt = match config.indicator_style {
            IndicatorStyle::Classify => sym,
            IndicatorStyle::FileType => {
                // Don't append an asterisk.
                match sym {
                    Some('*') => None,
                    _ => sym,
                }
            }
            IndicatorStyle::Slash => {
                // Append only a slash.
                match sym {
                    Some('/') => Some('/'),
                    _ => None,
                }
            }
            IndicatorStyle::None => None,
        };

        if let Some(c) = char_opt {
            name.push(c);
        }
    }

    if config.format == Format::Long
        && path.file_type(out).is_some()
        && path.file_type(out).unwrap().is_symlink()
        && !path.must_dereference
    {
        match path.p_buf.read_link() {
            Ok(target) => {
                name.push_str(" -> ");

                // We might as well color the symlink output after the arrow.
                // This makes extra system calls, but provides important information that
                // people run `ls -l --color` are very interested in.
                if let Some(style_manager) = style_manager {
                    // We get the absolute path to be able to construct PathData with valid Metadata.
                    // This is because relative symlinks will fail to get_metadata.
                    let mut absolute_target = target.clone();
                    if target.is_relative() {
                        if let Some(parent) = path.p_buf.parent() {
                            absolute_target = parent.join(absolute_target);
                        }
                    }

                    let target_data = PathData::new(absolute_target, None, None, config, false);

                    // If we have a symlink to a valid file, we use the metadata of said file.
                    // Because we use an absolute path, we can assume this is guaranteed to exist.
                    // Otherwise, we use path.md(), which will guarantee we color to the same
                    // color of non-existent symlinks according to style_for_path_with_metadata.
                    if path.get_metadata(out).is_none()
                        && get_metadata_with_deref_opt(
                            target_data.p_buf.as_path(),
                            target_data.must_dereference,
                        )
                        .is_err()
                    {
                        name.push_str(&path.p_buf.read_link().unwrap().to_string_lossy());
                    } else {
                        name.push_str(&color_name(
                            &escape_name(target.as_os_str(), &config.quoting_style),
                            path,
                            style_manager,
                            out,
                            Some(&target_data),
                            is_wrap(name.len()),
                        ));
                    }
                } else {
                    // If no coloring is required, we just use target as is.
                    // Apply the right quoting
                    name.push_str(&escape_name(target.as_os_str(), &config.quoting_style));
                }
            }
            Err(err) => {
                show!(LsError::IOErrorContext(err, path.p_buf.clone(), false));
            }
        }
    }

    // Prepend the security context to the `name` and adjust `width` in order
    // to get correct alignment from later calls to`display_grid()`.
    if config.context {
        if let Some(pad_count) = prefix_context {
            let security_context = if matches!(config.format, Format::Commas) {
                path.security_context.clone()
            } else {
                pad_left(&path.security_context, pad_count)
            };
            name = format!("{security_context} {name}");
        }
    }

    name
}

fn create_hyperlink(name: &str, path: &PathData) -> String {
    let hostname = hostname::get().unwrap_or_else(|_| OsString::from(""));
    let hostname = hostname.to_string_lossy();

    let absolute_path = fs::canonicalize(&path.p_buf).unwrap_or_default();
    let absolute_path = absolute_path.to_string_lossy();

    #[cfg(not(target_os = "windows"))]
    let unencoded_chars = "_-.:~/";
    #[cfg(target_os = "windows")]
    let unencoded_chars = "_-.:~/\\";

    // percentage encoding of path
    let absolute_path: String = absolute_path
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || unencoded_chars.contains(c) {
                c.to_string()
            } else {
                format!("%{:02x}", c as u8)
            }
        })
        .collect();

    // \x1b = ESC, \x07 = BEL
    format!("\x1b]8;;file://{hostname}{absolute_path}\x07{name}\x1b]8;;\x07")
}

#[cfg(not(unix))]
fn display_symlink_count(_metadata: &Metadata) -> String {
    // Currently not sure of how to get this on Windows, so I'm punting.
    // Git Bash looks like it may do the same thing.
    String::from("1")
}

#[cfg(unix)]
fn display_symlink_count(metadata: &Metadata) -> String {
    metadata.nlink().to_string()
}

#[cfg(unix)]
fn display_inode(metadata: &Metadata) -> String {
    get_inode(metadata)
}

// This returns the SELinux security context as UTF8 `String`.
// In the long term this should be changed to `OsStr`, see discussions at #2621/#2656
fn get_security_context(config: &Config, p_buf: &Path, must_dereference: bool) -> String {
    let substitute_string = "?".to_string();
    // If we must dereference, ensure that the symlink is actually valid even if the system
    // does not support SELinux.
    // Conforms to the GNU coreutils where a dangling symlink results in exit code 1.
    if must_dereference {
        match get_metadata_with_deref_opt(p_buf, must_dereference) {
            Err(err) => {
                // The Path couldn't be dereferenced, so return early and set exit code 1
                // to indicate a minor error
                show!(LsError::IOErrorContext(err, p_buf.to_path_buf(), false));
                return substitute_string;
            }
            Ok(_md) => (),
        }
    }
    if config.selinux_supported {
        #[cfg(feature = "selinux")]
        {
            match selinux::SecurityContext::of_path(p_buf, must_dereference.to_owned(), false) {
                Err(_r) => {
                    // TODO: show the actual reason why it failed
                    show_warning!("failed to get security context of: {}", p_buf.quote());
                    substitute_string
                }
                Ok(None) => substitute_string,
                Ok(Some(context)) => {
                    let context = context.as_bytes();

                    let context = context.strip_suffix(&[0]).unwrap_or(context);
                    String::from_utf8(context.to_vec()).unwrap_or_else(|e| {
                        show_warning!(
                            "getting security context of: {}: {}",
                            p_buf.quote(),
                            e.to_string()
                        );
                        String::from_utf8_lossy(context).into_owned()
                    })
                }
            }
        }
        #[cfg(not(feature = "selinux"))]
        {
            substitute_string
        }
    } else {
        substitute_string
    }
}

#[cfg(unix)]
fn calculate_padding_collection(
    items: &[PathData],
    config: &Config,
    out: &mut BufWriter<Stdout>,
) -> PaddingCollection {
    let mut padding_collections = PaddingCollection {
        inode: 1,
        link_count: 1,
        uname: 1,
        group: 1,
        context: 1,
        size: 1,
        major: 1,
        minor: 1,
        block_size: 1,
    };

    for item in items {
        #[cfg(unix)]
        if config.inode {
            let inode_len = if let Some(md) = item.get_metadata(out) {
                display_inode(md).len()
            } else {
                continue;
            };
            padding_collections.inode = inode_len.max(padding_collections.inode);
        }

        if config.alloc_size {
            if let Some(md) = item.get_metadata(out) {
                let block_size_len = display_size(get_block_size(md, config), config).len();
                padding_collections.block_size = block_size_len.max(padding_collections.block_size);
            }
        }

        if config.format == Format::Long {
            let context_len = item.security_context.len();
            let (link_count_len, uname_len, group_len, size_len, major_len, minor_len) =
                display_dir_entry_size(item, config, out);
            padding_collections.link_count = link_count_len.max(padding_collections.link_count);
            padding_collections.uname = uname_len.max(padding_collections.uname);
            padding_collections.group = group_len.max(padding_collections.group);
            if config.context {
                padding_collections.context = context_len.max(padding_collections.context);
            }
            if items.len() == 1usize {
                padding_collections.size = 0usize;
                padding_collections.major = 0usize;
                padding_collections.minor = 0usize;
            } else {
                padding_collections.major = major_len.max(padding_collections.major);
                padding_collections.minor = minor_len.max(padding_collections.minor);
                padding_collections.size = size_len
                    .max(padding_collections.size)
                    .max(padding_collections.major);
            }
        }
    }

    padding_collections
}

#[cfg(not(unix))]
fn calculate_padding_collection(
    items: &[PathData],
    config: &Config,
    out: &mut BufWriter<Stdout>,
) -> PaddingCollection {
    let mut padding_collections = PaddingCollection {
        link_count: 1,
        uname: 1,
        group: 1,
        context: 1,
        size: 1,
        block_size: 1,
    };

    for item in items {
        if config.alloc_size {
            if let Some(md) = item.get_metadata(out) {
                let block_size_len = display_size(get_block_size(md, config), config).len();
                padding_collections.block_size = block_size_len.max(padding_collections.block_size);
            }
        }

        let context_len = item.security_context.len();
        let (link_count_len, uname_len, group_len, size_len, _major_len, _minor_len) =
            display_dir_entry_size(item, config, out);
        padding_collections.link_count = link_count_len.max(padding_collections.link_count);
        padding_collections.uname = uname_len.max(padding_collections.uname);
        padding_collections.group = group_len.max(padding_collections.group);
        if config.context {
            padding_collections.context = context_len.max(padding_collections.context);
        }
        padding_collections.size = size_len.max(padding_collections.size);
    }

    padding_collections
}
