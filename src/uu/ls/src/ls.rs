// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) somegroup nlink tabsize dired subdired dtype colorterm stringly
// spell-checker:ignore nohash strtime clocale

#[cfg(unix)]
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::{borrow::Cow, fs::DirEntry};
use std::{
    cell::OnceCell,
    cmp::Reverse,
    ffi::{OsStr, OsString},
    fmt::Write as _,
    fs::{self, FileType, Metadata, ReadDir},
    io::{BufWriter, ErrorKind, IsTerminal, Stdout, Write, stdout},
    iter,
    num::IntErrorKind,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::{
    Arg, ArgAction, Command,
    builder::{NonEmptyStringValueParser, PossibleValue, ValueParser},
};
use lscolors::Colorable;
use thiserror::Error;

#[cfg(unix)]
use uucore::libc::{S_IXGRP, S_IXOTH, S_IXUSR};
use uucore::{
    display::Quotable,
    error::{UError, UResult, set_exit_code},
    format_usage,
    fs::FileInformation,
    fsext::metadata_get_time,
    os_str_as_bytes_lossy,
    parser::shortcut_value_parser::ShortcutValueParser,
    show, translate,
    version_cmp::version_cmp,
};

mod colors;
mod config;
mod dired;
mod display;

pub use config::{Config, options};
pub use display::Format;

use colors::StyleManager;
use config::options::QUOTING_STYLE;
use config::{Dereference, Files, Sort};
use dired::DiredOutput;
use display::{display_items, display_size, should_display, show_dir_name};

#[derive(Error, Debug)]
enum LsError {
    #[error("{}", translate!("ls-error-invalid-line-width", "width" => format!("'{_0}'")))]
    InvalidLineWidth(String),

    #[error("{}", translate!("ls-error-general-io", "error" => _0))]
    IOError(#[from] std::io::Error),

    #[error("{}", match .1.kind() {
        ErrorKind::NotFound => translate!("ls-error-cannot-access-no-such-file", "path" => .0.quote()),
        ErrorKind::PermissionDenied => match .1.raw_os_error().unwrap_or(1) {
            1 => translate!("ls-error-cannot-access-operation-not-permitted", "path" => .0.quote()),
            _ => if .0.is_dir() {
                translate!("ls-error-cannot-open-directory-permission-denied", "path" => .0.quote())
            } else {
                translate!("ls-error-cannot-open-file-permission-denied", "path" => .0.quote())
            },
        },
        _ => if 9 == .1.raw_os_error().unwrap_or(1) {
            translate!("ls-error-cannot-open-directory-bad-descriptor", "path" => .0.quote())
        } else {
            translate!("ls-error-unknown-io-error", "path" => .0.quote(), "error" => format!("{:?}", .1))
        },
    })]
    IOErrorContext(PathBuf, std::io::Error, bool),

    #[error("{}", translate!("ls-error-invalid-block-size", "size" => format!("'{_0}'")))]
    BlockSizeParseError(String),

    #[error("{}", translate!("ls-error-dired-and-zero-incompatible"))]
    DiredAndZeroAreIncompatible,

    #[error("{}", translate!("ls-error-not-listing-already-listed", "path" => .0.maybe_quote()))]
    AlreadyListedError(PathBuf),

    #[error("{}", translate!("ls-error-invalid-time-style", "style" => .0.quote()))]
    TimeStyleParseError(String),
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
            Self::TimeStyleParseError(_) => 2,
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 2)?;

    let config = Config::from(&matches)?;

    let locs = matches
        .get_many::<OsString>(options::PATHS)
        .map_or_else(|| vec![Path::new(".")], |v| v.map(Path::new).collect());

    list(locs, &config)
}

pub fn uu_app() -> Command {
    uucore::clap_localization::configure_localized_command(
        Command::new("ls")
            .version(uucore::crate_version!())
            .override_usage(format_usage(&translate!("ls-usage")))
            .about(translate!("ls-about")),
    )
    .infer_long_args(true)
    .disable_help_flag(true)
    .args_override_self(true)
    .arg(
        Arg::new(options::HELP)
            .long(options::HELP)
            .help(translate!("ls-help-print-help"))
            .action(ArgAction::Help),
    )
    // Format arguments
    .arg(
        Arg::new(options::FORMAT)
            .long(options::FORMAT)
            .help(translate!("ls-help-set-display-format"))
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
            .help(translate!("ls-help-display-files-columns"))
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
            .help(translate!("ls-help-display-detailed-info"))
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
            .help(translate!("ls-help-list-entries-rows"))
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
        Arg::new(options::format::TAB_SIZE)
            .short('T')
            .long(options::format::TAB_SIZE)
            .env("TABSIZE")
            .value_name("COLS")
            .help(translate!("ls-help-assume-tab-stops")),
    )
    .arg(
        Arg::new(options::format::COMMAS)
            .short('m')
            .help(translate!("ls-help-list-entries-commas"))
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
            .help(translate!("ls-help-list-entries-nul"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::DIRED)
            .long(options::DIRED)
            .short('D')
            .help(translate!("ls-help-generate-dired-output"))
            .action(ArgAction::SetTrue)
            .overrides_with(options::HYPERLINK),
    )
    .arg(
        Arg::new(options::HYPERLINK)
            .long(options::HYPERLINK)
            .help(translate!("ls-help-hyperlink-filenames"))
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
            .help(translate!("ls-help-list-one-file-per-line"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::format::LONG_NO_GROUP)
            .short('o')
            .help(translate!("ls-help-long-format-no-group"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::format::LONG_NO_OWNER)
            .short('g')
            .help(translate!("ls-help-long-no-owner"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::format::LONG_NUMERIC_UID_GID)
            .short('n')
            .long(options::format::LONG_NUMERIC_UID_GID)
            .help(translate!("ls-help-long-numeric-uid-gid"))
            .action(ArgAction::SetTrue),
    )
    // Quoting style
    .arg(
        Arg::new(QUOTING_STYLE)
            .long(QUOTING_STYLE)
            .help(translate!("ls-help-set-quoting-style"))
            .value_parser(ShortcutValueParser::new([
                PossibleValue::new("literal"),
                PossibleValue::new("locale"),
                PossibleValue::new("shell"),
                PossibleValue::new("shell-escape"),
                PossibleValue::new("shell-always"),
                PossibleValue::new("shell-escape-always"),
                PossibleValue::new("clocale"),
                PossibleValue::new("c").alias("c-maybe"),
                PossibleValue::new("escape"),
            ]))
            .overrides_with_all([
                QUOTING_STYLE,
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
            .help(translate!("ls-help-literal-quoting-style"))
            .overrides_with_all([
                QUOTING_STYLE,
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
            .help(translate!("ls-help-escape-quoting-style"))
            .overrides_with_all([
                QUOTING_STYLE,
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
            .help(translate!("ls-help-c-quoting-style"))
            .overrides_with_all([
                QUOTING_STYLE,
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
            .help(translate!("ls-help-replace-control-chars"))
            .overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::SHOW_CONTROL_CHARS)
            .long(options::SHOW_CONTROL_CHARS)
            .help(translate!("ls-help-show-control-chars"))
            .overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
            .action(ArgAction::SetTrue),
    )
    // Time arguments
    .arg(
        Arg::new(options::TIME)
            .long(options::TIME)
            .help(translate!("ls-help-show-time-field"))
            .value_name("field")
            .value_parser(ShortcutValueParser::new([
                PossibleValue::new("atime").alias("access").alias("use"),
                PossibleValue::new("ctime").alias("status"),
                PossibleValue::new("mtime").alias("modification"),
                PossibleValue::new("birth").alias("creation"),
            ]))
            .hide_possible_values(true)
            .require_equals(true)
            .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE]),
    )
    .arg(
        Arg::new(options::time::CHANGE)
            .short('c')
            .help(translate!("ls-help-time-change"))
            .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::time::ACCESS)
            .short('u')
            .help(translate!("ls-help-time-access"))
            .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
            .action(ArgAction::SetTrue),
    )
    // Hide and ignore
    .arg(
        Arg::new(options::HIDE)
            .long(options::HIDE)
            .action(ArgAction::Append)
            .value_name("PATTERN")
            .help(translate!("ls-help-hide-pattern")),
    )
    .arg(
        Arg::new(options::IGNORE)
            .short('I')
            .long(options::IGNORE)
            .action(ArgAction::Append)
            .value_name("PATTERN")
            .help(translate!("ls-help-ignore-pattern")),
    )
    .arg(
        Arg::new(options::IGNORE_BACKUPS)
            .short('B')
            .long(options::IGNORE_BACKUPS)
            .help(translate!("ls-help-ignore-backups"))
            .action(ArgAction::SetTrue),
    )
    // Sort arguments
    .arg(
        Arg::new(options::SORT)
            .long(options::SORT)
            .help(translate!("ls-help-sort-by-field"))
            .value_name("field")
            .value_parser(ShortcutValueParser::new([
                "name",
                "none",
                "time",
                "size",
                "version",
                "extension",
                "width",
            ]))
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
            .help(translate!("ls-help-sort-by-size"))
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
            .help(translate!("ls-help-sort-by-time"))
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
            .help(translate!("ls-help-sort-by-version"))
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
            .help(translate!("ls-help-sort-by-extension"))
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
            .help(translate!("ls-help-sort-none"))
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
            .help(translate!("ls-help-dereference-all"))
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
            .help(translate!("ls-help-dereference-dir-args"))
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
            .help(translate!("ls-help-dereference-args"))
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
            .help(translate!("ls-help-no-group"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::AUTHOR)
            .long(options::AUTHOR)
            .help(translate!("ls-help-author"))
            .action(ArgAction::SetTrue),
    )
    // Other Flags
    .arg(
        Arg::new(options::files::ALL)
            .short('a')
            .long(options::files::ALL)
            // Overrides -A (as the order matters)
            .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL])
            .help(translate!("ls-help-all-files"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::files::ALMOST_ALL)
            .short('A')
            .long(options::files::ALMOST_ALL)
            // Overrides -a (as the order matters)
            .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL])
            .help(translate!("ls-help-almost-all"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::files::UNSORTED_ALL)
            .short('f')
            .help(translate!("ls-help-unsorted-all"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::DIRECTORY)
            .short('d')
            .long(options::DIRECTORY)
            .help(translate!("ls-help-directory"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::size::HUMAN_READABLE)
            .short('h')
            .long(options::size::HUMAN_READABLE)
            .help(translate!("ls-help-human-readable"))
            .overrides_with_all([options::size::BLOCK_SIZE, options::size::SI])
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::size::KIBIBYTES)
            .short('k')
            .long(options::size::KIBIBYTES)
            .help(translate!("ls-help-kibibytes"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::size::SI)
            .long(options::size::SI)
            .help(translate!("ls-help-si"))
            .overrides_with_all([options::size::BLOCK_SIZE, options::size::HUMAN_READABLE])
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::size::BLOCK_SIZE)
            .long(options::size::BLOCK_SIZE)
            .require_equals(true)
            .value_name("BLOCK_SIZE")
            .help(translate!("ls-help-block-size"))
            .overrides_with_all([options::size::SI, options::size::HUMAN_READABLE]),
    )
    .arg(
        Arg::new(options::INODE)
            .short('i')
            .long(options::INODE)
            .help(translate!("ls-help-print-inode"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::REVERSE)
            .short('r')
            .long(options::REVERSE)
            .help(translate!("ls-help-reverse-sort"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::RECURSIVE)
            .short('R')
            .long(options::RECURSIVE)
            .help(translate!("ls-help-recursive"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::WIDTH)
            .long(options::WIDTH)
            .short('w')
            .help(translate!("ls-help-terminal-width"))
            .value_name("COLS"),
    )
    .arg(
        Arg::new(options::size::ALLOCATION_SIZE)
            .short('s')
            .long(options::size::ALLOCATION_SIZE)
            .help(translate!("ls-help-allocation-size"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::COLOR)
            .long(options::COLOR)
            .help(translate!("ls-help-color-output"))
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
            .help(translate!("ls-help-indicator-style"))
            .value_parser(ShortcutValueParser::new([
                "none",
                "slash",
                "file-type",
                "classify",
            ]))
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
            .help(translate!("ls-help-classify"))
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
            .help(translate!("ls-help-file-type"))
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
            .help(translate!("ls-help-slash-directories"))
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
            .help(translate!("ls-help-time-style"))
            .value_name("TIME_STYLE")
            .env("TIME_STYLE")
            .value_parser(NonEmptyStringValueParser::new())
            .overrides_with_all([options::TIME_STYLE]),
    )
    .arg(
        Arg::new(options::FULL_TIME)
            .long(options::FULL_TIME)
            .overrides_with(options::FULL_TIME)
            .help(translate!("ls-help-full-time"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::CONTEXT)
            .short('Z')
            .long(options::CONTEXT)
            .help(translate!("ls-help-context"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::GROUP_DIRECTORIES_FIRST)
            .long(options::GROUP_DIRECTORIES_FIRST)
            .help(translate!("ls-help-group-directories-first"))
            .action(ArgAction::SetTrue),
    )
    // Positional arguments
    .arg(
        Arg::new(options::PATHS)
            .action(ArgAction::Append)
            .value_hint(clap::ValueHint::AnyPath)
            .value_parser(ValueParser::os_string()),
    )
    .after_help(translate!("ls-after-help"))
}

/// Represents a Path along with it's associated data.
/// Any data that will be reused several times makes sense to be added to this structure.
/// Caching data here helps eliminate redundant syscalls to fetch same information.
#[derive(Debug)]
struct PathData {
    // Result<MetaData> got from symlink_metadata() or metadata() based on config
    md: OnceCell<Option<Metadata>>,
    ft: OnceCell<Option<FileType>>,
    security_context: OnceCell<Box<str>>,
    // Name of the file - will be empty for . or ..
    display_name: OsString,
    // PathBuf that all above data corresponds to
    p_buf: PathBuf,
    must_dereference: bool,
    command_line: bool,
}

impl PathData {
    fn new(
        p_buf: PathBuf,
        opt_dir_entry: Option<DirEntry>,
        opt_file_name: Option<OsString>,
        config: &Config,
        command_line: bool,
    ) -> Self {
        // We cannot use `Path::ends_with` or `Path::Components`, because they remove occurrences of '.'
        // For '..', the filename is None
        let display_name = if let Some(name) = opt_file_name {
            name
        } else if command_line {
            p_buf.as_os_str().to_os_string()
        } else {
            p_buf.file_name().unwrap_or_default().to_os_string()
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

        let ft: OnceCell<Option<FileType>> = OnceCell::new();
        let md: OnceCell<Option<Metadata>> = OnceCell::new();
        let security_context: OnceCell<Box<str>> = OnceCell::new();

        if !must_dereference {
            opt_dir_entry.map(|de| ft.get_or_init(|| de.file_type().ok()));
        }

        Self {
            md,
            ft,
            security_context,
            display_name,
            p_buf,
            must_dereference,
            command_line,
        }
    }

    fn metadata(&self) -> Option<&Metadata> {
        self.md
            .get_or_init(|| {
                match get_metadata_with_deref_opt(self.path(), self.must_dereference) {
                    Err(err) => {
                        // FIXME: A bit tricky to propagate the result here
                        let mut out: std::io::StdoutLock<'static> = stdout().lock();
                        let _ = out.flush();
                        let errno = err.raw_os_error().unwrap_or(1i32);
                        // a bad fd will throw an error when dereferenced,
                        // but GNU will not throw an error until a bad fd "dir"
                        // is entered, here we match that GNU behavior, by handing
                        // back the non-dereferenced metadata upon an EBADF
                        if self.must_dereference && errno == 9i32 {
                            if let Ok(file) = self.path().read_link() {
                                return file.symlink_metadata().ok();
                            }
                        }
                        show!(LsError::IOErrorContext(
                            self.path().to_path_buf(),
                            err,
                            self.command_line
                        ));
                        None
                    }
                    Ok(md) => Some(md),
                }
            })
            .as_ref()
    }

    fn file_type(&self) -> Option<&FileType> {
        self.ft
            .get_or_init(|| self.metadata().map(Metadata::file_type))
            .as_ref()
    }

    fn is_dangling_link(&self) -> bool {
        // deref enabled, self is real dir entry, self has metadata associated with link, but not with target
        self.must_dereference && self.file_type().is_none() && self.metadata().is_none()
    }

    #[cfg(unix)]
    fn is_executable_file(&self) -> bool {
        self.file_type().is_some_and(FileType::is_file)
            && self.metadata().is_some_and(file_is_executable)
    }

    fn security_context(&self, config: &Config) -> &str {
        self.security_context
            .get_or_init(|| get_security_context(&self.p_buf, self.must_dereference, config).into())
    }

    fn path(&self) -> &Path {
        &self.p_buf
    }

    fn display_name(&self) -> &OsStr {
        &self.display_name
    }
}

impl Colorable for PathData {
    fn file_name(&self) -> OsString {
        self.display_name().to_os_string()
    }
    fn file_type(&self) -> Option<FileType> {
        self.file_type().copied()
    }
    fn metadata(&self) -> Option<Metadata> {
        self.metadata().cloned()
    }
    fn path(&self) -> PathBuf {
        self.path().to_path_buf()
    }
}

type DirData = (PathBuf, bool);

// A struct to encapsulate state that is passed around from `list` functions.
struct ListState<'a> {
    out: BufWriter<Stdout>,
    style_manager: Option<StyleManager<'a>>,
    // TODO: More benchmarking with different use cases is required here.
    // From experiments, BTreeMap may be faster than HashMap, especially as the
    // number of users/groups is very limited. It seems like nohash::IntMap
    // performance was equivalent to BTreeMap.
    // It's possible a simple vector linear(binary?) search implementation would be even faster.
    #[cfg(unix)]
    uid_cache: FxHashMap<u32, String>,
    #[cfg(unix)]
    gid_cache: FxHashMap<u32, String>,
    recent_time_range: RangeInclusive<SystemTime>,
    stack: Vec<DirData>,
    listed_ancestors: FxHashSet<FileInformation>,
    initial_locs_len: usize,
}

#[allow(clippy::cognitive_complexity)]
pub fn list(locs: Vec<&Path>, config: &Config) -> UResult<()> {
    let mut files = Vec::<PathData>::new();
    let mut dirs = Vec::<PathData>::new();
    let mut dired = DiredOutput::default();
    let initial_locs_len = locs.len();

    let mut state = ListState {
        out: BufWriter::new(stdout()),
        style_manager: config.color.as_ref().map(StyleManager::new),
        #[cfg(unix)]
        uid_cache: FxHashMap::default(),
        #[cfg(unix)]
        gid_cache: FxHashMap::default(),
        // Time range for which to use the "recent" format. Anything from 0.5 year in the past to now
        // (files with modification time in the future use "old" format).
        // According to GNU a Gregorian year has 365.2425 * 24 * 60 * 60 == 31556952 seconds on the average.
        recent_time_range: (SystemTime::now() - Duration::new(31_556_952 / 2, 0))
            ..=SystemTime::now(),
        stack: Vec::new(),
        listed_ancestors: FxHashSet::default(),
        initial_locs_len,
    };

    for loc in locs {
        let path_data = PathData::new(PathBuf::from(loc), None, None, config, true);

        // Getting metadata here is no big deal as it's just the CWD
        // and we really just want to know if the strings exist as files/dirs
        //
        // Proper GNU handling is don't show if dereferenced symlink DNE
        // but only for the base dir, for a child dir show, and print ?s
        // in long format
        if path_data.metadata().is_none() {
            continue;
        }

        let show_dir_contents = if let Some(ft) = path_data.file_type() {
            !config.directory && ft.is_dir()
        } else {
            set_exit_code(1);
            false
        };

        if show_dir_contents {
            dirs.push(path_data);
        } else {
            files.push(path_data);
        }
    }

    sort_entries(&mut files, config);
    sort_entries(&mut dirs, config);

    if let Some(style_manager) = state.style_manager.as_mut() {
        // ls will try to write a reset before anything is written if normal
        // color is given
        if style_manager.get_normal_style().is_some() {
            let to_write = style_manager.reset(true);
            write!(state.out, "{to_write}")?;
        }
    }

    display_items(&files, config, &mut state, &mut dired)?;

    for (pos, path_data) in dirs.iter().enumerate() {
        let needs_blank_line = pos != 0 || !files.is_empty();
        // Do read_dir call here to match GNU semantics by printing
        // read_dir errors before directory headings, names and totals
        let mut read_dir = match fs::read_dir(path_data.path()) {
            Err(err) => {
                // flush stdout buffer before the error to preserve formatting and order
                state.out.flush()?;
                show!(LsError::IOErrorContext(
                    path_data.path().to_path_buf(),
                    err,
                    path_data.command_line
                ));
                continue;
            }
            Ok(rd) => rd,
        };

        state.listed_ancestors.insert(FileInformation::from_path(
            path_data.path(),
            path_data.must_dereference,
        )?);

        // List each of the arguments to ls first.
        depth_first_list(
            (path_data.path().to_path_buf(), needs_blank_line),
            &mut read_dir,
            config,
            &mut state,
            &mut dired,
            true,
        )?;

        // Only runs if it must list recursively.
        while let Some(dir_data) = state.stack.pop() {
            let mut read_dir = match fs::read_dir(&dir_data.0) {
                Err(err) => {
                    // flush stdout buffer before the error to preserve formatting and order
                    state.out.flush()?;
                    show!(LsError::IOErrorContext(
                        path_data.path().to_path_buf(),
                        err,
                        path_data.command_line
                    ));
                    continue;
                }
                Ok(rd) => rd,
            };

            depth_first_list(
                dir_data,
                &mut read_dir,
                config,
                &mut state,
                &mut dired,
                false,
            )?;

            // Heuristic to ensure stack does not keep its capacity forever if there is
            // combinatorial explosion; we decrease it logarithmically here.
            let (cap, len) = (state.stack.capacity(), state.stack.len());
            if cap > (len + 4) * 2 {
                state.stack.shrink_to(len + (cap - len) / 2);
            }
        }

        // No need to clear state.buf since [`enter_directory`] drains it.
        state.listed_ancestors.clear();
    }
    if config.dired && !config.hyperlink {
        dired::print_dired_output(config, &dired, &mut state.out)?;
    }
    Ok(())
}

fn sort_entries(entries: &mut [PathData], config: &Config) {
    match config.sort {
        Sort::Time => entries.sort_by_key(|k| {
            Reverse(
                k.metadata()
                    .and_then(|md| metadata_get_time(md, config.time))
                    .unwrap_or(UNIX_EPOCH),
            )
        }),
        Sort::Size => {
            entries.sort_by_key(|k| Reverse(k.metadata().map_or(0, Metadata::len)));
        }
        // The default sort in GNU ls is case insensitive
        Sort::Name => entries.sort_by(|a, b| a.display_name().cmp(b.display_name())),
        Sort::Version => entries.sort_by(|a, b| {
            version_cmp(
                os_str_as_bytes_lossy(a.path().as_os_str()).as_ref(),
                os_str_as_bytes_lossy(b.path().as_os_str()).as_ref(),
            )
            .then(a.path().to_string_lossy().cmp(&b.path().to_string_lossy()))
        }),
        Sort::Extension => entries.sort_by(|a, b| {
            a.path()
                .extension()
                .cmp(&b.path().extension())
                .then(a.path().file_stem().cmp(&b.path().file_stem()))
        }),
        Sort::Width => entries.sort_by(|a, b| {
            a.display_name()
                .len()
                .cmp(&b.display_name().len())
                .then(a.display_name().cmp(b.display_name()))
        }),
        Sort::None => {}
    }

    if config.reverse {
        entries.reverse();
    }

    if config.group_directories_first && config.sort != Sort::None {
        entries.sort_by_key(|p| {
            let ft = {
                // We will always try to deref symlinks to group directories, so PathData.md
                // is not always useful.
                if p.must_dereference {
                    p.file_type()
                } else {
                    None
                }
            };

            !match ft {
                None => {
                    // If it metadata cannot be determined, treat as a file.
                    get_metadata_with_deref_opt(p.p_buf.as_path(), true)
                        .map_or_else(|_| false, |m| m.is_dir())
                }
                Some(ft) => ft.is_dir(),
            }
        });
    }
}

fn is_hidden(path_data: &PathData) -> bool {
    #[cfg(windows)]
    {
        let metadata = path_data.metadata().unwrap();
        let attr = metadata.file_attributes();
        (attr & 0x2) > 0
    }
    #[cfg(not(windows))]
    {
        path_data.file_name().as_encoded_bytes().starts_with(b".")
    }
}

fn should_display(path_data: &PathData, config: &Config) -> bool {
    // check if hidden
    if config.files == Files::Normal && is_hidden(path_data) {
        return false;
    }

    // check if it is among ignore_patterns
    let options = MatchOptions {
        // setting require_literal_leading_dot to match behavior in GNU ls
        require_literal_leading_dot: true,
        require_literal_separator: false,
        case_sensitive: true,
    };

    let file_name = path_data.file_name();
    // If the decoding fails, still match best we can
    // FIXME: use OsStrings or Paths once we have a glob crate that supports it:
    // https://github.com/rust-lang/glob/issues/23
    // https://github.com/rust-lang/glob/issues/78
    // https://github.com/BurntSushi/ripgrep/issues/1250

    let file_name_as_cow = file_name.to_string_lossy();

    !config
        .ignore_patterns
        .iter()
        .any(|p| p.matches_with(&file_name_as_cow, options))
}

fn depth_first_list(
    (dir_path, needs_blank_line): DirData,
    read_dir: &mut ReadDir,
    config: &Config,
    state: &mut ListState,
    dired: &mut DiredOutput,
    is_top_level: bool,
) -> UResult<()> {
    let path_data = PathData::new(dir_path, None, None, config, false);

    // Print dir heading - name... 'total' comes after error display
    if state.initial_locs_len > 1 || config.recursive {
        if is_top_level {
            if needs_blank_line {
                writeln!(state.out)?;
                if config.dired {
                    dired.padding += 1;
                }
            }
            if config.dired {
                dired::indent(&mut state.out)?;
            }
            show_dir_name(&path_data, &mut state.out, config)?;
            writeln!(state.out)?;
            if config.dired {
                let dir_len = path_data.path().as_os_str().len();
                // add the //SUBDIRED// coordinates
                dired::calculate_subdired(dired, dir_len);
                // Add the padding for the dir name
                dired::add_dir_name(dired, dir_len);
            }
        } else {
            writeln!(state.out)?;
            if config.dired {
                dired.padding += 1;
                dired::indent(&mut state.out)?;
                let dir_name_size = path_data.path().as_os_str().len();
                dired::calculate_subdired(dired, dir_name_size);
                dired::add_dir_name(dired, dir_name_size);
            }
            show_dir_name(&path_data, &mut state.out, config)?;
            writeln!(state.out)?;
        }
    }

    // Append entries with initial dot files and record their existence
    let (ref mut buf, trim) = if config.files == Files::All {
        const DOT_DIRECTORIES: usize = 2;
        let v = vec![
            PathData::new(
                path_data.path().to_path_buf(),
                None,
                Some(".".into()),
                config,
                false,
            ),
            PathData::new(
                path_data.path().join(".."),
                None,
                Some("..".into()),
                config,
                false,
            ),
        ];
        (v, DOT_DIRECTORIES)
    } else {
        (Vec::new(), 0)
    };

    // Convert those entries to the PathData struct
    for raw_entry in read_dir {
        match raw_entry {
            Ok(dir_entry) => {
                let path_data =
                    PathData::new(dir_entry.path(), Some(dir_entry), None, config, false);
                if should_display(&path_data, config) {
                    buf.push(path_data);
                }
            }
            Err(err) => {
                state.out.flush()?;
                show!(LsError::IOError(err));
            }
        }
    }
    // Relinquish unused space since we won't need it anymore.
    buf.shrink_to_fit();

    sort_entries(buf, config);

    if config.format == Format::Long || config.alloc_size {
        let total = return_total(buf, config, &mut state.out)?;
        write!(state.out, "{}", total.as_str())?;
        if config.dired {
            dired::add_total(dired, total.len());
        }
    }

    display_items(buf, config, state, dired)?;

    if config.recursive {
        for e in buf
            .iter()
            .skip(trim)
            .filter(|p| p.file_type().is_some_and(FileType::is_dir))
            .rev()
        {
            // Try to open only to report any errors in order to match GNU semantics.
            if let Err(err) = fs::read_dir(e.path()) {
                state.out.flush()?;
                show!(LsError::IOErrorContext(
                    e.path().to_path_buf(),
                    err,
                    e.command_line
                ));
            } else {
                let fi = FileInformation::from_path(e.path(), e.must_dereference)?;
                if state.listed_ancestors.insert(fi) {
                    // Push to stack, but with a less aggressive growth curve.
                    let (cap, len) = (state.stack.capacity(), state.stack.len());
                    if cap == len {
                        state.stack.reserve_exact(len / 4 + 4);
                    }
                    state.stack.push((e.path().to_path_buf(), true));
                } else {
                    state.out.flush()?;
                    show!(LsError::AlreadyListedError(e.path().to_path_buf()));
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
    state: &mut ListState,
) -> (usize, usize, usize, usize, usize, usize) {
    // TODO: Cache/memorize the display_* results so we don't have to recalculate them.
    if let Some(md) = entry.metadata() {
        let (size_len, major_len, minor_len) = match display_len_or_rdev(md, config) {
            SizeOrDeviceId::Device(major, minor) => {
                (major.len() + minor.len() + 2usize, major.len(), minor.len())
            }
            SizeOrDeviceId::Size(size) => (size.len(), 0usize, 0usize),
        };
        (
            display_symlink_count(md).len(),
            display_uname(md, config, state).len(),
            display_group(md, config, state).len(),
            size_len,
            major_len,
            minor_len,
        )
    } else {
        (0, 0, 0, 0, 0, 0)
    }
}

// A simple, performant, ExtendPad trait to add a string to a Vec<u8>, padding with spaces
// on the left or right, without making additional copies, or using formatting functions.
trait ExtendPad {
    fn extend_pad_left(&mut self, string: &str, count: usize);
    fn extend_pad_right(&mut self, string: &str, count: usize);
}

impl ExtendPad for Vec<u8> {
    fn extend_pad_left(&mut self, string: &str, count: usize) {
        if string.len() < count {
            self.extend(iter::repeat_n(b' ', count - string.len()));
        }
        self.extend(string.as_bytes());
    }

    fn extend_pad_right(&mut self, string: &str, count: usize) {
        self.extend(string.as_bytes());
        if string.len() < count {
            self.extend(iter::repeat_n(b' ', count - string.len()));
        }
    }
}

// TODO: Consider converting callers to use ExtendPad instead, as it avoids
// additional copies.
fn pad_left(string: &str, count: usize) -> String {
    format!("{string:>count$}")
}

fn return_total(
    items: &[PathData],
    config: &Config,
    out: &mut BufWriter<Stdout>,
) -> UResult<String> {
    let mut total_size = 0;
    for item in items {
        total_size += item
            .metadata()
            .as_ref()
            .map_or(0, |md| get_block_size(md, config));
    }
    if config.dired {
        dired::indent(out)?;
    }
    Ok(format!(
        "{}{}",
        translate!("ls-total", "size" => display_size(total_size, config)),
        config.line_ending
    ))
}

fn display_additional_leading_info(
    item: &PathData,
    padding: &PaddingCollection,
    config: &Config,
) -> String {
    let mut result = String::new();
    #[cfg(unix)]
    {
        if config.inode {
            let i = if let Some(md) = item.metadata() {
                get_inode(md)
            } else {
                "?".to_owned()
            };
            write!(result, "{} ", pad_left(&i, padding.inode)).unwrap();
        }
    }

    if config.alloc_size {
        let s = if let Some(md) = item.metadata() {
            display_size(get_block_size(md, config), config)
        } else {
            "?".to_owned()
        };
        // extra space is insert to align the sizes, as needed for all formats, except for the comma format.
        if config.format == Format::Commas {
            write!(result, "{s} ").unwrap();
        } else {
            write!(result, "{} ", pad_left(&s, padding.block_size)).unwrap();
        }
    }

    result
}

#[allow(clippy::cognitive_complexity)]
fn display_items(
    items: &[PathData],
    config: &Config,
    state: &mut ListState,
    dired: &mut DiredOutput,
) -> UResult<()> {
    // `-Z`, `--context`:
    // Display the SELinux security context or '?' if none is found. When used with the `-l`
    // option, print the security context to the left of the size column.

    let quoted = items.iter().any(|item| {
        let name = escape_name_with_locale(item.display_name(), config);
        os_str_starts_with(&name, b"'")
    });

    if config.format == Format::Long {
        let padding_collection = calculate_padding_collection(items, config, state);

        for item in items {
            #[cfg(unix)]
            let should_display_leading_info = config.inode || config.alloc_size;
            #[cfg(not(unix))]
            let should_display_leading_info = config.alloc_size;

            if should_display_leading_info {
                let more_info = display_additional_leading_info(item, &padding_collection, config);

                write!(state.out, "{more_info}")?;
            }

            display_item_long(item, &padding_collection, config, state, dired, quoted)?;
        }
    } else {
        let mut longest_context_len = 1;
        let prefix_context = if config.context {
            for item in items {
                let context_len = item.security_context(config).len();
                longest_context_len = context_len.max(longest_context_len);
            }
            Some(longest_context_len)
        } else {
            None
        };

        let padding = calculate_padding_collection(items, config, state);

        // we need to apply normal color to non filename output
        if let Some(style_manager) = &mut state.style_manager {
            write!(state.out, "{}", style_manager.apply_normal())?;
        }

        let mut names_vec = Vec::new();

        #[cfg(unix)]
        let should_display_leading_info = config.inode || config.alloc_size;
        #[cfg(not(unix))]
        let should_display_leading_info = config.alloc_size;

        for i in items {
            let more_info = if should_display_leading_info {
                Some(display_additional_leading_info(i, &padding, config))
            } else {
                None
            };
            // it's okay to set current column to zero which is used to decide
            // whether text will wrap or not, because when format is grid or
            // column ls will try to place the item name in a new line if it
            // wraps.
            let cell = display_item_name(
                i,
                config,
                prefix_context,
                more_info,
                state,
                LazyCell::new(Box::new(|| 0)),
            );

            names_vec.push(cell.displayed);
        }

        let mut names = names_vec.into_iter();

        match config.format {
            Format::Columns => {
                display_grid(
                    names,
                    config.width,
                    Direction::TopToBottom,
                    &mut state.out,
                    quoted,
                    config.tab_size,
                )?;
            }
            Format::Across => {
                display_grid(
                    names,
                    config.width,
                    Direction::LeftToRight,
                    &mut state.out,
                    quoted,
                    config.tab_size,
                )?;
            }
            Format::Commas => {
                let mut current_col = 0;
                if let Some(name) = names.next() {
                    write_os_str(&mut state.out, &name)?;
                    current_col = ansi_width(&name.to_string_lossy()) as u16 + 2;
                }
                for name in names {
                    let name_width = ansi_width(&name.to_string_lossy()) as u16;
                    // If the width is 0 we print one single line
                    if config.width != 0 && current_col + name_width + 1 > config.width {
                        current_col = name_width + 2;
                        writeln!(state.out, ",")?;
                    } else {
                        current_col += name_width + 2;
                        write!(state.out, ", ")?;
                    }
                    write_os_str(&mut state.out, &name)?;
                }
                // Current col is never zero again if names have been printed.
                // So we print a newline.
                if current_col > 0 {
                    write!(state.out, "{}", config.line_ending)?;
                }
            }
            _ => {
                for name in names {
                    write_os_str(&mut state.out, &name)?;
                    write!(state.out, "{}", config.line_ending)?;
                }
            }
        }
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
    names: impl Iterator<Item = OsString>,
    width: u16,
    direction: Direction,
    out: &mut BufWriter<Stdout>,
    quoted: bool,
    tab_size: usize,
) -> UResult<()> {
    if width == 0 {
        // If the width is 0 we print one single line
        let mut printed_something = false;
        for name in names {
            if printed_something {
                write!(out, "  ")?;
            }
            printed_something = true;
            write_os_str(out, &name)?;
        }
        if printed_something {
            writeln!(out)?;
        }
    } else {
        let names: Vec<_> = if quoted {
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
                    if os_str_starts_with(&n, b"'") || os_str_starts_with(&n, b"\"") {
                        n
                    } else {
                        let mut ret: OsString = " ".into();
                        ret.push(n);
                        ret
                    }
                })
                .collect()
        } else {
            names.collect()
        };

        // FIXME: the Grid crate only supports &str, so can't display raw bytes
        let names: Vec<_> = names
            .into_iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();

        // Since tab_size=0 means no \t, use Spaces separator for optimization.
        let filling = match tab_size {
            0 => Filling::Spaces(DEFAULT_SEPARATOR_SIZE),
            _ => Filling::Tabs {
                spaces: DEFAULT_SEPARATOR_SIZE,
                tab_size,
            },
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

fn calculate_line_len(output_len: usize, item_len: usize, line_ending: LineEnding) -> usize {
    output_len + item_len + line_ending.to_string().len()
}

fn update_dired_for_item(
    dired: &mut DiredOutput,
    output_display_len: usize,
    displayed_len: usize,
    dired_name_len: usize,
    line_ending: LineEnding,
) {
    let line_len = calculate_line_len(output_display_len, displayed_len, line_ending);
    dired::calculate_and_update_positions(dired, output_display_len, dired_name_len, line_len);
}

/// This writes to the [`BufWriter`] `state.out` a single string of the output of `ls -l`.
///
/// It writes the following keys, in order:
/// * `inode` ([`get_inode`], config-optional)
/// * `permissions` ([`display_permissions`])
/// * `symlink_count` ([`display_symlink_count`])
/// * `owner` ([`display_uname`], config-optional)
/// * `group` ([`display_group`], config-optional)
/// * `author` ([`display_uname`], config-optional)
/// * `size / rdev` ([`display_len_or_rdev`])
/// * `system_time` ([`display_date`])
/// * `item_name` ([`display_item_name`])
///
/// This function needs to display information in columns:
/// * permissions and `system_time` are already guaranteed to be pre-formatted in fixed length.
/// * `item_name` is the last column and is left-aligned.
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
    state: &mut ListState,
    dired: &mut DiredOutput,
    quoted: bool,
) -> UResult<()> {
    let mut output_display: Vec<u8> = Vec::with_capacity(128);

    // apply normal color to non filename outputs
    if let Some(style_manager) = &mut state.style_manager {
        output_display.extend(style_manager.apply_normal().as_bytes());
    }
    if config.dired {
        output_display.extend(b"  ");
    }
    if let Some(md) = item.metadata() {
        #[cfg(any(not(unix), target_os = "android", target_os = "macos"))]
        // TODO: See how Mac should work here
        let is_acl_set = false;
        #[cfg(all(unix, not(any(target_os = "android", target_os = "macos"))))]
        let is_acl_set = has_acl(item.path());
        output_display.extend(display_permissions(md, true).as_bytes());
        if item.security_context(config).len() > 1 {
            // GNU `ls` uses a "." character to indicate a file with a security context,
            // but not other alternate access method.
            output_display.extend(b".");
        } else if is_acl_set {
            output_display.extend(b"+");
        } else {
            output_display.extend(b" ");
        }

        output_display.extend_pad_left(&display_symlink_count(md), padding.link_count);

        if config.long.owner {
            output_display.extend(b" ");
            output_display.extend_pad_right(display_uname(md, config, state), padding.uname);
        }

        if config.long.group {
            output_display.extend(b" ");
            output_display.extend_pad_right(display_group(md, config, state), padding.group);
        }

        if config.context {
            output_display.extend(b" ");
            output_display.extend_pad_right(item.security_context(config), padding.context);
        }

        // Author is only different from owner on GNU/Hurd, so we reuse
        // the owner, since GNU/Hurd is not currently supported by Rust.
        if config.long.author {
            output_display.extend(b" ");
            output_display.extend_pad_right(display_uname(md, config, state), padding.uname);
        }

        match display_len_or_rdev(md, config) {
            SizeOrDeviceId::Size(size) => {
                output_display.extend(b" ");
                output_display.extend_pad_left(&size, padding.size);
            }
            SizeOrDeviceId::Device(major, minor) => {
                output_display.extend(b" ");
                output_display.extend_pad_left(
                    &major,
                    #[cfg(not(unix))]
                    0usize,
                    #[cfg(unix)]
                    padding.major.max(
                        padding
                            .size
                            .saturating_sub(padding.minor.saturating_add(2usize)),
                    ),
                );
                output_display.extend(b", ");
                output_display.extend_pad_left(
                    &minor,
                    #[cfg(not(unix))]
                    0usize,
                    #[cfg(unix)]
                    padding.minor,
                );
            }
        }

        output_display.extend(b" ");
        display_date(md, config, state, &mut output_display)?;
        output_display.extend(b" ");

        let item_display = display_item_name(
            item,
            config,
            None,
            None,
            state,
            LazyCell::new(Box::new(|| {
                ansi_width(&String::from_utf8_lossy(&output_display))
            })),
        );

        let needs_space = quoted && !os_str_starts_with(&item_display.displayed, b"'");

        if config.dired {
            let mut dired_name_len = item_display.dired_name_len;
            if needs_space {
                dired_name_len += 1;
            }
            let displayed_len = item_display.displayed.len() + usize::from(needs_space);
            update_dired_for_item(
                dired,
                output_display.len(),
                displayed_len,
                dired_name_len,
                config.line_ending,
            );
        }

        let item_name = item_display.displayed;
        let displayed_item = if needs_space {
            let mut ret: OsString = " ".into();
            ret.push(&item_name);
            ret
        } else {
            item_name
        };

        write_os_str(&mut output_display, &displayed_item)?;
        output_display.extend(config.line_ending.to_string().as_bytes());
    } else {
        #[cfg(unix)]
        let leading_char = {
            if let Some(ft) = item.file_type() {
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
            } else if item.is_dangling_link() {
                "l"
            } else {
                "-"
            }
        };
        #[cfg(not(unix))]
        let leading_char = {
            if let Some(ft) = item.file_type() {
                if ft.is_symlink() {
                    "l"
                } else if ft.is_dir() {
                    "d"
                } else {
                    "-"
                }
            } else if item.is_dangling_link() {
                "l"
            } else {
                "-"
            }
        };

        output_display.extend(leading_char.as_bytes());
        output_display.extend(b"?????????");
        if item.security_context(config).len() > 1 {
            // GNU `ls` uses a "." character to indicate a file with a security context,
            // but not other alternate access method.
            output_display.extend(b".");
        }
        output_display.extend(b" ");
        output_display.extend_pad_left("?", padding.link_count);

        if config.long.owner {
            output_display.extend(b" ");
            output_display.extend_pad_right("?", padding.uname);
        }

        if config.long.group {
            output_display.extend(b" ");
            output_display.extend_pad_right("?", padding.group);
        }

        if config.context {
            output_display.extend(b" ");
            output_display.extend_pad_right(item.security_context(config), padding.context);
        }

        // Author is only different from owner on GNU/Hurd, so we reuse
        // the owner, since GNU/Hurd is not currently supported by Rust.
        if config.long.author {
            output_display.extend(b" ");
            output_display.extend_pad_right("?", padding.uname);
        }

        let displayed_item = display_item_name(
            item,
            config,
            None,
            None,
            state,
            LazyCell::new(Box::new(|| {
                ansi_width(&String::from_utf8_lossy(&output_display))
            })),
        );
        let date_len = 12;

        output_display.extend(b" ");
        output_display.extend_pad_left("?", padding.size);
        output_display.extend(b" ");
        output_display.extend_pad_left("?", date_len);
        output_display.extend(b" ");

        if config.dired {
            update_dired_for_item(
                dired,
                output_display.len(),
                displayed_item.displayed.len(),
                displayed_item.dired_name_len,
                config.line_ending,
            );
        }
        let displayed_item = displayed_item.displayed;
        write_os_str(&mut output_display, &displayed_item)?;
        output_display.extend(config.line_ending.to_string().as_bytes());
    }
    state.out.write_all(&output_display)?;

    Ok(())
}

#[cfg(unix)]
fn get_inode(metadata: &Metadata) -> String {
    format!("{}", metadata.ino())
}

// Currently getpwuid is `linux` target only. If it's broken state.out into
// a posix-compliant attribute this can be updated...
#[cfg(unix)]
fn display_uname<'a>(metadata: &Metadata, config: &Config, state: &'a mut ListState) -> &'a String {
    let uid = metadata.uid();

    state.uid_cache.entry(uid).or_insert_with(|| {
        if config.long.numeric_uid_gid {
            uid.to_string()
        } else {
            entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string())
        }
    })
}

#[cfg(unix)]
fn display_group<'a>(metadata: &Metadata, config: &Config, state: &'a mut ListState) -> &'a String {
    let gid = metadata.gid();
    state.gid_cache.entry(gid).or_insert_with(|| {
        if config.long.numeric_uid_gid {
            gid.to_string()
        } else {
            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string())
        }
    })
}

#[cfg(not(unix))]
fn display_uname(_metadata: &Metadata, _config: &Config, _state: &mut ListState) -> &'static str {
    "somebody"
}

#[cfg(not(unix))]
fn display_group(_metadata: &Metadata, _config: &Config, _state: &mut ListState) -> &'static str {
    "somegroup"
}

fn display_date(
    metadata: &Metadata,
    config: &Config,
    state: &mut ListState,
    out: &mut Vec<u8>,
) -> UResult<()> {
    let Some(time) = metadata_get_time(metadata, config.time) else {
        out.extend(b"???");
        return Ok(());
    };

    // Use "recent" format if the given date is considered recent (i.e., in the last 6 months),
    // or if no "older" format is available.
    let fmt = match &config.time_format_older {
        Some(time_format_older) if !state.recent_time_range.contains(&time) => time_format_older,
        _ => &config.time_format_recent,
    };

    format_system_time(out, time, fmt, FormatSystemTimeFallback::Integer)
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
            let major = major(dev);
            let minor = minor(dev);
            return SizeOrDeviceId::Device(major.to_string(), minor.to_string());
        }
    }
    let len_adjusted = {
        let d = metadata.len() / config.file_size_block_size;
        let r = metadata.len() % config.file_size_block_size;
        if r == 0 { d } else { d + 1 }
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

fn classify_file(path: &PathData) -> Option<char> {
    let file_type = path.file_type()?;

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
                // Safe unwrapping if the file was removed between listing and display
                // See https://github.com/uutils/coreutils/issues/5371
            } else if path.is_executable_file() {
                Some('*')
            } else {
                None
            }
        }
        #[cfg(not(unix))]
        None
    }
}

fn depth_first_list(
    (dir_path, needs_blank_line): DirData,
    mut read_dir: ReadDir,
    config: &Config,
    state: &mut ListState,
    dired: &mut DiredOutput,
    is_top_level: bool,
) -> UResult<()> {
    let path_data = PathData::new(dir_path, None, None, config, false);

    // Print dir heading - name... 'total' comes after error display
    if state.initial_locs_len > 1 || config.recursive {
        if is_top_level {
            if needs_blank_line {
                writeln!(state.out)?;
                if config.dired {
                    dired.padding += 1;
                }
            }
            if config.dired {
                dired::indent(&mut state.out)?;
            }
            show_dir_name(&path_data, &mut state.out, config)?;
            writeln!(state.out)?;
            if config.dired {
                let dir_len = path_data.path().as_os_str().len();
                // add the //SUBDIRED// coordinates
                dired::calculate_subdired(dired, dir_len);
                // Add the padding for the dir name
                dired::add_dir_name(dired, dir_len);
            }
        } else {
            writeln!(state.out)?;
            if config.dired {
                dired.padding += 1;
                dired::indent(&mut state.out)?;
                let dir_name_size = path_data.path().as_os_str().len();
                dired::calculate_subdired(dired, dir_name_size);
                dired::add_dir_name(dired, dir_name_size);
            }
            show_dir_name(&path_data, &mut state.out, config)?;
            writeln!(state.out)?;
        }
    }

    // Append entries with initial dot files and record their existence
    let (ref mut buf, trim) = if config.files == Files::All {
        const DOT_DIRECTORIES: usize = 2;
        let v = vec![
            PathData::new(
                path_data.path().to_path_buf(),
                None,
                Some(".".into()),
                config,
                false,
            ),
            PathData::new(
                path_data.path().join(".."),
                None,
                Some("..".into()),
                config,
                false,
            ),
        ];
        (v, DOT_DIRECTORIES)
    } else {
        (Vec::new(), 0)
    };

                    match fs::canonicalize(&absolute_target) {
                        Ok(resolved_target) if config.color.is_some() => {
                            let target_data = PathData::new(
                                resolved_target,
                                None,
                                target_path.file_name().map(OsStr::to_os_string),
                                config,
                                false,
                            );

                            name.push(color_name(
                                escaped_target,
                                &target_data,
                                style_manager,
                                None,
                                is_wrap(name.len()),
                            ));
                        }
                        _ => {
                            name.push(
                                style_manager.apply_missing_target_style(
                                    escaped_target,
                                    is_wrap(name.len()),
                                ),
                            );
                        }
                    }
                } else {
                    // If no coloring is required, we just use target as is.
                    // Apply the right quoting
                    name.push(escape_name_with_locale(target_path.as_os_str(), config));
                }
            }
            Err(err) => {
                state.out.flush()?;
                show!(LsError::IOError(err));
            }
        }
    }
    // Relinquish unused space since we won't need it anymore.
    buf.shrink_to_fit();

    sort_entries(buf, config);

    if config.format == Format::Long || config.alloc_size {
        let total = return_total(buf, config, &mut state.out)?;
        write!(state.out, "{}", total.as_str())?;
        if config.dired {
            dired::add_total(dired, total.len());
        }
    }

    display_items(buf, config, state, dired)?;

    if config.recursive {
        for e in buf
            .iter()
            .skip(trim)
            .filter(|p| p.file_type().is_some_and(FileType::is_dir))
            .rev()
        {
            // Try to open only to report any errors in order to match GNU semantics.
            if let Err(err) = fs::read_dir(e.path()) {
                state.out.flush()?;
                show!(LsError::IOErrorContext(
                    e.path().to_path_buf(),
                    err,
                    e.command_line
                ));
            } else {
                let fi = FileInformation::from_path(e.path(), e.must_dereference)?;
                if state.listed_ancestors.insert(fi) {
                    // Push to stack, but with a less aggressive growth curve.
                    let (cap, len) = (state.stack.capacity(), state.stack.len());
                    if cap == len {
                        state.stack.reserve_exact(len / 4 + 4);
                    }
                    state.stack.push((e.path().to_path_buf(), true));
                } else {
                    state.out.flush()?;
                    show!(LsError::AlreadyListedError(e.path().to_path_buf()));
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

fn return_total(
    items: &[PathData],
    config: &Config,
    out: &mut BufWriter<Stdout>,
) -> UResult<String> {
    let mut total_size = 0;
    for item in items {
        total_size += item
            .metadata()
            .as_ref()
            .map_or(0, |md| get_block_size(md, config));
    }
    if config.dired {
        dired::indent(out)?;
    }
    Ok(format!(
        "{}{}",
        translate!("ls-total", "size" => display_size(total_size, config)),
        config.line_ending
    ))
}

#[allow(unused_variables)]
fn get_block_size(md: &Metadata, config: &Config) -> u64 {
    /* GNU ls will display sizes in terms of block size
       md.len() will differ from this value when the file has some holes
    */
    #[cfg(unix)]
    {
        use uucore::format::human::SizeFormat;

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

/// This returns the `SELinux` security context as UTF8 `String`.
/// In the long term this should be changed to [`OsStr`], see discussions at #2621/#2656
fn get_security_context<'a>(
    path: &'a Path,
    must_dereference: bool,
    config: &'a Config,
) -> Cow<'a, str> {
    static SUBSTITUTE_STRING: &str = "?";

    // If we must dereference, ensure that the symlink is actually valid even if the system
    // does not support SELinux.
    // Conforms to the GNU coreutils where a dangling symlink results in exit code 1.
    if must_dereference {
        if let Err(err) = get_metadata_with_deref_opt(path, must_dereference) {
            // The Path couldn't be dereferenced, so return early and set exit code 1
            // to indicate a minor error
            // Only show error when context display is requested to avoid duplicate messages
            if config.context {
                show!(LsError::IOErrorContext(path.to_path_buf(), err, false));
            }
            return Cow::Borrowed(SUBSTITUTE_STRING);
        }
    }

    #[cfg(all(feature = "selinux", any(target_os = "linux", target_os = "android")))]
    if config.selinux_supported {
        use uucore::show_warning;

        match selinux::SecurityContext::of_path(path, must_dereference, false) {
            Err(_r) => {
                // TODO: show the actual reason why it failed
                show_warning!(
                    "{}",
                    translate!(
                        "ls-warning-failed-to-get-security-context",
                        "path" => path.quote().to_string()
                    )
                );
                return Cow::Borrowed(SUBSTITUTE_STRING);
            }
            Ok(None) => return Cow::Borrowed(SUBSTITUTE_STRING),
            Ok(Some(context)) => {
                let context = context.as_bytes();

                let context = context.strip_suffix(&[0]).unwrap_or(context);

                let res: String = String::from_utf8(context.to_vec()).unwrap_or_else(|e| {
                    show_warning!(
                        "{}",
                        translate!(
                            "ls-warning-getting-security-context",
                            "path" => path.quote().to_string(),
                            "error" => e.to_string()
                        )
                    );

                    String::from_utf8_lossy(context).to_string()
                });

                return Cow::Owned(res);
            }
        }
    }

    #[cfg(all(feature = "smack", target_os = "linux"))]
    if config.smack_supported {
        // For SMACK, use the path to get the label
        // If must_dereference is true, we follow the symlink
        let target_path = if must_dereference {
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        } else {
            path.to_path_buf()
        };

        return uucore::smack::get_smack_label_for_path(&target_path)
            .map_or(Cow::Borrowed(SUBSTITUTE_STRING), Cow::Owned);
    }

    Cow::Borrowed(SUBSTITUTE_STRING)
}
