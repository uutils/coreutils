// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) somegroup nlink tabsize dired subdired dtype colorterm stringly
// spell-checker:ignore nohash strtime clocale

use clap::{
    Arg, ArgAction, Command,
    builder::{NonEmptyStringValueParser, PossibleValue, ValueParser},
};
use lscolors::Colorable;
#[cfg(unix)]
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::borrow::Cow;
use std::cell::RefCell;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::{
    cell::OnceCell,
    cmp::Reverse,
    ffi::{OsStr, OsString},
    fs::{self, DirEntry, FileType, Metadata, ReadDir},
    io::{BufWriter, ErrorKind, Stdout, Write, stdout},
    ops::RangeInclusive,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
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

pub mod output;
pub use config::{Config, options};
pub use display::Format;
pub use output::{EntryInfo, LsOutput, StreamMode, StreamingOutput};

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
		ErrorKind::NotADirectory => translate!("ls-error-not-directory", "path" => .0.quote()),
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

/// Represents the possible values of [`PathData::display_name`]. The reason this is a
/// separate enum is to avoid a self-referential struct, as it is moved in hot loops.
#[derive(Debug, Clone)]
enum PathDataDisplayName<'a> {
    SelfReferential,
    Custom(Cow<'a, OsStr>),
}

/// Represents a Path along with it's associated data.
/// Any data that will be reused several times makes sense to be added to this structure.
/// Caching data here helps eliminate redundant syscalls to fetch same information.
#[derive(Debug)]
/// Internal representation of file/directory entry data.
///
/// This struct is used internally for file enumeration. It can be converted
/// to [`EntryInfo`] for programmatic access via the [`LsOutput`] trait.
pub struct PathData<'a> {
    // Result<MetaData> got from symlink_metadata() or metadata() based on config
    md: OnceCell<Option<Metadata>>,
    ft: OnceCell<Option<FileType>>,
    // can be used to avoid reading the filetype. Can be also called d_type:
    // https://www.gnu.org/software/libc/manual/html_node/Directory-Entries.html
    de: RefCell<Option<DirEntry>>,
    security_context: OnceCell<Box<str>>,
    // Name of the file - will be empty for . or ..
    display_name: PathDataDisplayName<'a>,
    // PathBuf that all above data corresponds to
    p_buf: Cow<'a, Path>,
    must_dereference: bool,
    command_line: bool,
}

impl<'a> PathData<'a> {
    /// Convert this PathData to an EntryInfo for programmatic access
    pub fn to_entry_info(&self, config: &Config) -> EntryInfo {
        EntryInfo {
            path: self.p_buf.clone().into_owned(),
            display_name: self.display_name().to_os_string(),
            file_type: self.file_type().copied(),
            metadata: self.metadata().cloned(),
            security_context: self.security_context(config).to_string(),
            command_line: self.command_line,
            must_dereference: self.must_dereference,
        }
    }

    fn new(
        p_buf: Cow<'a, Path>,
        dir_entry: Option<DirEntry>,
        file_name: Option<Cow<'a, OsStr>>,
        config: &Config,
        command_line: bool,
    ) -> Self {
        // We cannot use `Path::ends_with` or `Path::Components`, because they remove occurrences of '.'
        // For '..', the filename is None
        let display_name = if let Some(name) = file_name {
            PathDataDisplayName::Custom(name)
        } else if command_line {
            PathDataDisplayName::SelfReferential
        } else {
            PathDataDisplayName::Custom(
                dir_entry
                    .as_ref()
                    .map(DirEntry::file_name)
                    .unwrap_or_default()
                    .into(),
            )
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

        // Why prefer to check the DirEntry file_type()?  B/c the call is
        // nearly free compared to a metadata() call on a Path
        let ft: OnceCell<Option<FileType>> = OnceCell::new();
        let md: OnceCell<Option<Metadata>> = OnceCell::new();
        let security_context: OnceCell<Box<str>> = OnceCell::new();

        let de: RefCell<Option<DirEntry>> = if let Some(de) = dir_entry {
            if must_dereference {
                if let Ok(md_pb) = p_buf.metadata() {
                    ft.get_or_init(|| Some(md_pb.file_type()));
                    md.get_or_init(|| Some(md_pb));
                }
            }

            if let Ok(ft_de) = de.file_type() {
                ft.get_or_init(|| Some(ft_de));
            }

            RefCell::new(Some(de))
        } else {
            RefCell::new(None)
        };

        Self {
            md,
            ft,
            de,
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
                if !self.must_dereference {
                    if let Some(dir_entry) = RefCell::take(&self.de) {
                        return dir_entry.metadata().ok();
                    }
                }

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
        match self.display_name {
            PathDataDisplayName::SelfReferential => self.p_buf.as_os_str(),
            PathDataDisplayName::Custom(ref cow) => cow,
        }
    }
}

impl Colorable for PathData<'_> {
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

// A struct to encapsulate state that is passed around from `list` functions.
#[cfg_attr(not(unix), allow(dead_code))]
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
    #[cfg(not(unix))]
    uid_cache: (),
    #[cfg(not(unix))]
    gid_cache: (),
    recent_time_range: RangeInclusive<SystemTime>,
    display_buf: Vec<u8>,
}

/// Text output implementation that formats entries for terminal display.
///
/// This is the default output sink used by [`list`] for standard ls behavior.
/// It handles all text formatting including colors, columns, long format, etc.
pub struct TextOutput<'a> {
    state: ListState<'a>,
    dired: DiredOutput,
}

impl<'a> TextOutput<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            state: ListState {
                out: BufWriter::new(stdout()),
                style_manager: config.color.as_ref().map(StyleManager::new),
                #[cfg(unix)]
                uid_cache: FxHashMap::default(),
                #[cfg(unix)]
                gid_cache: FxHashMap::default(),
                #[cfg(not(unix))]
                uid_cache: (),
                #[cfg(not(unix))]
                gid_cache: (),
                // Use "recent" format for files modified within the last ~0.5 years (31556952s).
                // According to GNU a Gregorian year has 365.2425 * 24 * 60 * 60 == 31556952 seconds on the average.
                recent_time_range: (SystemTime::now() - Duration::new(31_556_952 / 2, 0))
                    ..=SystemTime::now(),
                display_buf: Vec::with_capacity(if config.format == Format::Long {
                    128
                } else {
                    0
                }),
            },
            dired: DiredOutput::default(),
        }
    }
}

impl LsOutput for TextOutput<'_> {
    fn write_entries(&mut self, entries: &[PathData], config: &Config) -> UResult<()> {
        display_items(entries, config, &mut self.state, &mut self.dired)
    }

    fn write_dir_header(
        &mut self,
        path_data: &PathData,
        config: &Config,
        is_first: bool,
    ) -> UResult<()> {
        if is_first {
            if config.dired {
                dired::indent(&mut self.state.out)?;
            }
            show_dir_name(path_data, &mut self.state.out, config)?;
            writeln!(self.state.out)?;
            if config.dired {
                let dir_len = path_data.path().as_os_str().len();
                dired::calculate_subdired(&mut self.dired, dir_len);
                dired::add_dir_name(&mut self.dired, dir_len);
            }
        } else {
            writeln!(self.state.out)?;
            if config.dired {
                self.dired.line_offset += 1; // account for the blank line before recursive directory headings
                self.dired.padding = 0;
                dired::indent(&mut self.state.out)?;
                let dir_name_size = path_data.path().as_os_str().len();
                dired::calculate_subdired(&mut self.dired, dir_name_size);
                dired::add_dir_name(&mut self.dired, dir_name_size);
            }
            show_dir_name(path_data, &mut self.state.out, config)?;
            writeln!(self.state.out)?;
        }
        Ok(())
    }

    fn write_total(&mut self, total_size: u64, config: &Config) -> UResult<()> {
        if config.dired {
            dired::indent(&mut self.state.out)?;
        }
        let total = translate!("ls-total", "size" => display_size(total_size, config));
        let total_len = total.len() + 1;
        self.state.out.write_all(total.as_bytes())?;
        self.state.out.write_all(&[config.line_ending as u8])?;
        if config.dired {
            dired::add_total(&mut self.dired, total_len);
        }
        Ok(())
    }

    fn flush(&mut self) -> UResult<()> {
        self.state.out.flush()?;
        Ok(())
    }

    fn finalize(&mut self, config: &Config) -> UResult<()> {
        if config.dired && !config.hyperlink {
            dired::print_dired_output(config, &self.dired, &mut self.state.out)?;
        }
        Ok(())
    }

    fn initialize(&mut self, _config: &Config) -> UResult<()> {
        if let Some(style_manager) = self.state.style_manager.as_mut() {
            if style_manager.get_normal_style().is_some() {
                let to_write = style_manager.reset(true);
                write!(self.state.out, "{to_write}")?;
            }
        }
        Ok(())
    }
}

/// Lists files and directories, sending structured output to a custom sink.
///
/// This function provides programmatic access to ls functionality without
/// requiring text parsing. It enumerates files and directories according
/// to the provided configuration and sends each entry to the output sink.
///
/// # Arguments
///
/// * `locs` - Paths to list
/// * `config` - Configuration controlling listing behavior
/// * `output` - A sink implementing [`LsOutput`] to receive entries
///
/// # Example
///
/// ```ignore
/// use uu_ls::{Config, list_with_output, StreamingOutput};
/// use std::path::Path;
///
/// let config = Config::from(&matches)?;
/// let mut output = StreamingOutput::new();
/// list_with_output(vec![Path::new(".")], &config, &mut output)?;
///
/// for entry in output.entries() {
///     println!("{}: {:?}", entry.display_name.to_string_lossy(), entry.file_type);
/// }
/// ```
pub fn list_with_output<O: LsOutput>(
    locs: Vec<&Path>,
    config: &Config,
    output: &mut O,
) -> UResult<()> {
    let mut files = Vec::with_capacity(locs.len());
    let mut dirs = Vec::with_capacity(locs.len());
    let initial_locs_len = locs.len();

    for loc in locs {
        let path_data = PathData::new(loc.into(), None, None, config, true);

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

    output.initialize(config)?;

    // Write file entries.
    if matches!(output.stream_mode(), StreamMode::Streaming) {
        for file_entry in &files {
            output.write_entry(&file_entry.to_entry_info(config))?;
        }
    } else {
        output.write_entries(&files, config)?;
    }

    let mut entries = Vec::<PathData>::with_capacity(2);

    for (pos, path_data) in dirs.iter().enumerate() {
        // Do read_dir call here to match GNU semantics by printing
        // read_dir errors before directory headings, names and totals
        let read_dir = match fs::read_dir(path_data.path()) {
            Err(err) => {
                // flush stdout buffer before the error to preserve formatting and order
                output.flush()?;
                show!(LsError::IOErrorContext(
                    path_data.path().to_path_buf(),
                    err,
                    path_data.command_line
                ));
                continue;
            }
            Ok(rd) => rd,
        };

        // Write dir heading for multiple arguments or recursive mode
        if initial_locs_len > 1 || config.recursive {
            let is_first = pos == 0 && files.is_empty();
            output.write_dir_header(path_data, config, is_first)?;
        }

        let mut listed_ancestors = FxHashSet::default();
        listed_ancestors.insert(FileInformation::from_path(
            path_data.path(),
            path_data.must_dereference,
        )?);
        enter_directory(
            path_data,
            read_dir,
            config,
            &mut listed_ancestors,
            output,
            &mut entries,
        )?;
    }

    output.finalize(config)?;
    Ok(())
}

fn collect_directory_entries<O: LsOutput>(
    entries: &mut Vec<PathData>,
    path_data: &PathData,
    config: &Config,
    output: &mut O,
    read_dir: &mut ReadDir,
) -> UResult<()> {
    entries.clear();

    if config.files == Files::All {
        entries.push(PathData::new(
            path_data.path().to_path_buf().into(),
            None,
            Some(OsStr::new(".").into()),
            config,
            false,
        ));
        entries.push(PathData::new(
            path_data.path().join("..").into(),
            None,
            Some(OsStr::new("..").into()),
            config,
            false,
        ));
    }

    for raw_entry in read_dir.by_ref() {
        let dir_entry = match raw_entry {
            Ok(path) => path,
            Err(err) => {
                output.flush()?;
                show!(LsError::IOError(err));
                continue;
            }
        };

        if should_display(&dir_entry, config) {
            entries.push(PathData::new(
                dir_entry.path().into(),
                Some(dir_entry),
                None,
                config,
                false,
            ));
        }
    }

    sort_entries(entries, config);
    entries.shrink_to_fit();

    Ok(())
}

fn write_directory_entries<O: LsOutput>(
    entries: &[PathData],
    config: &Config,
    output: &mut O,
) -> UResult<()> {
    if config.format == Format::Long || config.alloc_size {
        let total_size: u64 = entries
            .iter()
            .map(|item| {
                item.metadata()
                    .as_ref()
                    .map_or(0, |md| get_block_size(md, config))
            })
            .sum();
        output.write_total(total_size, config)?;
    }

    if matches!(output.stream_mode(), StreamMode::Streaming) {
        for entry in entries {
            output.write_entry(&entry.to_entry_info(config))?;
        }
        Ok(())
    } else {
        output.write_entries(entries, config)
    }
}

/// Recursively traverse directories using an explicit stack.
///
/// This avoids deep recursive call chains while preserving GNU-style
/// directory traversal order and ancestor detection.
fn enter_directory<O: LsOutput>(
    path_data: &PathData,
    read_dir: ReadDir,
    config: &Config,
    listed_ancestors: &mut FxHashSet<FileInformation>,
    output: &mut O,
    entries: &mut Vec<PathData>,
) -> UResult<()> {
    struct StackEntry {
        path: PathBuf,
        command_line: bool,
        is_first: bool,
    }

    let mut stack = Vec::new();
    let mut current = Some(StackEntry {
        path: path_data.path().to_path_buf(),
        command_line: path_data.command_line,
        is_first: true,
    });
    let mut initial_read_dir = Some(read_dir);

    while let Some(entry) = current.take().or_else(|| stack.pop()) {
        let path_data = PathData::new(
            entry.path.as_path().into(),
            None,
            None,
            config,
            entry.command_line,
        );

        if !entry.is_first {
            output.write_dir_header(&path_data, config, false)?;
        }

        let mut current_read_dir = if entry.is_first {
            initial_read_dir
                .take()
                .expect("initial read_dir is present for first entry")
        } else {
            match fs::read_dir(&entry.path) {
                Err(err) => {
                    output.flush()?;
                    show!(LsError::IOErrorContext(
                        entry.path.clone(),
                        err,
                        entry.command_line,
                    ));
                    continue;
                }
                Ok(rd) => rd,
            }
        };

        collect_directory_entries(entries, &path_data, config, output, &mut current_read_dir)?;
        write_directory_entries(entries, config, output)?;

        if config.recursive {
            let start = if config.files == Files::All { 2 } else { 0 };

            for child in entries
                .iter()
                .skip(start)
                .filter(|p| p.file_type().is_some_and(FileType::is_dir))
                .rev()
            {
                let child_path = child.path().to_path_buf();
                let child_must_dereference = child.must_dereference;
                let child_command_line = child.command_line;

                match fs::read_dir(&child_path) {
                    Err(err) => {
                        output.flush()?;
                        show!(LsError::IOErrorContext(
                            child_path.clone(),
                            err,
                            child_command_line,
                        ));
                    }
                    Ok(_) => {
                        if listed_ancestors.insert(FileInformation::from_path(
                            &child_path,
                            child_must_dereference,
                        )?) {
                            stack.push(StackEntry {
                                path: child_path,
                                command_line: child_command_line,
                                is_first: false,
                            });
                        } else {
                            output.flush()?;
                            show!(LsError::AlreadyListedError(child_path));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Lists files and directories with text output to stdout.
///
/// This is the standard ls entry point that formats output as text.
/// It uses [`list_with_output`] internally with a text formatter.
pub fn list(locs: Vec<&Path>, config: &Config) -> UResult<()> {
    let mut output = TextOutput::new(config);
    list_with_output(locs, config, &mut output)
}

fn sort_entries(entries: &mut [PathData], config: &Config) {
    match config.sort {
        Sort::Time => entries.sort_unstable_by_key(|k| {
            Reverse(
                k.metadata()
                    .and_then(|md| metadata_get_time(md, config.time))
                    .unwrap_or(UNIX_EPOCH),
            )
        }),
        Sort::Size => {
            entries.sort_unstable_by_key(|k| Reverse(k.metadata().map_or(0, Metadata::len)));
        }
        // The default sort in GNU ls is case insensitive
        Sort::Name => entries.sort_unstable_by(|a, b| a.display_name().cmp(b.display_name())),
        Sort::Version => entries.sort_unstable_by(|a, b| {
            version_cmp(
                os_str_as_bytes_lossy(a.path().as_os_str()).as_ref(),
                os_str_as_bytes_lossy(b.path().as_os_str()).as_ref(),
            )
            .then(a.path().cmp(b.path()))
        }),
        Sort::Extension => entries.sort_unstable_by(|a, b| {
            a.path()
                .extension()
                .cmp(&b.path().extension())
                .then(a.path().file_stem().cmp(&b.path().file_stem()))
        }),
        Sort::Width => entries.sort_unstable_by(|a, b| {
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
        entries.sort_unstable_by_key(|p| {
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
                    get_metadata_with_deref_opt(&p.p_buf, true)
                        .map_or_else(|_| false, |m| m.is_dir())
                }
                Some(ft) => ft.is_dir(),
            }
        });
    }
}

fn get_metadata_with_deref_opt(p_buf: &Path, dereference: bool) -> std::io::Result<Metadata> {
    if dereference {
        p_buf.metadata()
    } else {
        p_buf.symlink_metadata()
    }
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
                        "path" => path.quote()
                    )
                );
                return Cow::Borrowed(SUBSTITUTE_STRING);
            }
            Ok(None) => return Cow::Borrowed(SUBSTITUTE_STRING),
            Ok(Some(context)) => {
                let context = context.as_bytes();

                let context = context.strip_suffix(&[0]).unwrap_or(context);

                let res: String = match str::from_utf8(context) {
                    Ok(s) => s.to_string(),
                    Err(e) => {
                        show_warning!(
                            "{}",
                            translate!(
                                "ls-warning-getting-security-context",
                                "path" => path.quote(),
                                "error" => e
                            )
                        );
                        String::from_utf8_lossy(context).into_owned()
                    }
                };

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
