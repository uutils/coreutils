// This file is part of the uutils coreutils package.
//
// (c) Jeremiah Peschka <jeremiah.peschka@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) cpio svgz webm somegroup nlink rmvb xspf

// clippy bug https://github.com/rust-lang/rust-clippy/issues/7422
#![allow(clippy::nonstandard_macro_braces)]

#[macro_use]
extern crate uucore;
#[cfg(unix)]
#[macro_use]
extern crate lazy_static;

mod quoting_style;

use clap::{crate_version, App, Arg};
use globset::{self, Glob, GlobSet, GlobSetBuilder};
use lscolors::LsColors;
use number_prefix::NumberPrefix;
use once_cell::unsync::OnceCell;
use quoting_style::{escape_name, QuotingStyle};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::{
    cmp::Reverse,
    error::Error,
    fmt::Display,
    fs::{self, DirEntry, FileType, Metadata},
    io::{stdout, BufWriter, Stdout, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(unix)]
use std::{
    collections::HashMap,
    os::unix::fs::{FileTypeExt, MetadataExt},
    time::Duration,
};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use uucore::error::{set_exit_code, FromIo, UCustomError, UResult};

use unicode_width::UnicodeWidthStr;
#[cfg(unix)]
use uucore::libc::{S_IXGRP, S_IXOTH, S_IXUSR};
use uucore::{fs::display_permissions, version_cmp::version_cmp};

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

pub mod options {

    pub mod format {
        pub static ONE_LINE: &str = "1";
        pub static LONG: &str = "long";
        pub static COLUMNS: &str = "C";
        pub static ACROSS: &str = "x";
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
        pub static HUMAN_READABLE: &str = "human-readable";
        pub static SI: &str = "si";
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
    pub static BLOCKS: &str = "blocks";
    pub static REVERSE: &str = "reverse";
    pub static RECURSIVE: &str = "recursive";
    pub static COLOR: &str = "color";
    pub static PATHS: &str = "paths";
    pub static INDICATOR_STYLE: &str = "indicator-style";
    pub static TIME_STYLE: &str = "time-style";
    pub static FULL_TIME: &str = "full-time";
    pub static HIDE: &str = "hide";
    pub static IGNORE: &str = "ignore";
}

#[derive(Debug)]
enum LsError {
    InvalidLineWidth(String),
    NoMetadata(PathBuf),
}

impl UCustomError for LsError {
    fn code(&self) -> i32 {
        match self {
            LsError::InvalidLineWidth(_) => 2,
            LsError::NoMetadata(_) => 1,
        }
    }
}

impl Error for LsError {}

impl Display for LsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LsError::InvalidLineWidth(s) => write!(f, "invalid line width: '{}'", s),
            LsError::NoMetadata(p) => write!(f, "could not open file: '{}'", p.display()),
        }
    }
}

#[derive(PartialEq, Eq)]
enum Format {
    Columns,
    Long,
    OneLine,
    Across,
    Commas,
}

enum Sort {
    None,
    Name,
    Size,
    Time,
    Version,
    Extension,
}

enum SizeFormat {
    Bytes,
    Binary,  // Powers of 1024, --human-readable, -h
    Decimal, // Powers of 1000, --si
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

struct Config {
    format: Format,
    files: Files,
    sort: Sort,
    recursive: bool,
    reverse: bool,
    dereference: Dereference,
    ignore_patterns: GlobSet,
    size_format: SizeFormat,
    directory: bool,
    time: Time,
    #[cfg(unix)]
    inode: bool,
    blocks: bool,
    color: Option<LsColors>,
    long: LongFormat,
    width: Option<u16>,
    quoting_style: QuotingStyle,
    indicator_style: IndicatorStyle,
    time_style: TimeStyle,
}

// Fields that can be removed or added to the long format
struct LongFormat {
    author: bool,
    group: bool,
    owner: bool,
    #[cfg(unix)]
    numeric_uid_gid: bool,
}

impl Config {
    #[allow(clippy::cognitive_complexity)]
    fn from(options: clap::ArgMatches) -> UResult<Config> {
        let (mut format, opt) = if let Some(format_) = options.value_of(options::FORMAT) {
            (
                match format_ {
                    "long" | "verbose" => Format::Long,
                    "single-column" => Format::OneLine,
                    "columns" | "vertical" => Format::Columns,
                    "across" | "horizontal" => Format::Across,
                    "commas" => Format::Commas,
                    // below should never happen as clap already restricts the values.
                    _ => unreachable!("Invalid field for --format"),
                },
                options::FORMAT,
            )
        } else if options.is_present(options::format::LONG) {
            (Format::Long, options::format::LONG)
        } else if options.is_present(options::format::ACROSS) {
            (Format::Across, options::format::ACROSS)
        } else if options.is_present(options::format::COMMAS) {
            (Format::Commas, options::format::COMMAS)
        } else {
            (Format::Columns, options::format::COLUMNS)
        };

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
            let idx = options
                .indices_of(opt)
                .map(|x| x.max().unwrap())
                .unwrap_or(0);
            if [
                options::format::LONG_NO_OWNER,
                options::format::LONG_NO_GROUP,
                options::format::LONG_NUMERIC_UID_GID,
                options::FULL_TIME,
            ]
            .iter()
            .flat_map(|opt| options.indices_of(opt))
            .flatten()
            .any(|i| i >= idx)
            {
                format = Format::Long;
            } else if let Some(mut indices) = options.indices_of(options::format::ONE_LINE) {
                if indices.any(|i| i > idx) {
                    format = Format::OneLine;
                }
            }
        }

        let files = if options.is_present(options::files::ALL) {
            Files::All
        } else if options.is_present(options::files::ALMOST_ALL) {
            Files::AlmostAll
        } else {
            Files::Normal
        };

        let sort = if let Some(field) = options.value_of(options::SORT) {
            match field {
                "none" => Sort::None,
                "name" => Sort::Name,
                "time" => Sort::Time,
                "size" => Sort::Size,
                "version" => Sort::Version,
                "extension" => Sort::Extension,
                // below should never happen as clap already restricts the values.
                _ => unreachable!("Invalid field for --sort"),
            }
        } else if options.is_present(options::sort::TIME) {
            Sort::Time
        } else if options.is_present(options::sort::SIZE) {
            Sort::Size
        } else if options.is_present(options::sort::NONE) {
            Sort::None
        } else if options.is_present(options::sort::VERSION) {
            Sort::Version
        } else if options.is_present(options::sort::EXTENSION) {
            Sort::Extension
        } else {
            Sort::Name
        };

        let time = if let Some(field) = options.value_of(options::TIME) {
            match field {
                "ctime" | "status" => Time::Change,
                "access" | "atime" | "use" => Time::Access,
                "birth" | "creation" => Time::Birth,
                // below should never happen as clap already restricts the values.
                _ => unreachable!("Invalid field for --time"),
            }
        } else if options.is_present(options::time::ACCESS) {
            Time::Access
        } else if options.is_present(options::time::CHANGE) {
            Time::Change
        } else {
            Time::Modification
        };

        let needs_color = match options.value_of(options::COLOR) {
            None => options.is_present(options::COLOR),
            Some(val) => match val {
                "" | "always" | "yes" | "force" => true,
                "auto" | "tty" | "if-tty" => atty::is(atty::Stream::Stdout),
                /* "never" | "no" | "none" | */ _ => false,
            },
        };

        let color = if needs_color {
            Some(LsColors::from_env().unwrap_or_default())
        } else {
            None
        };

        let size_format = if options.is_present(options::size::HUMAN_READABLE) {
            SizeFormat::Binary
        } else if options.is_present(options::size::SI) {
            SizeFormat::Decimal
        } else {
            SizeFormat::Bytes
        };

        let long = {
            let author = options.is_present(options::AUTHOR);
            let group = !options.is_present(options::NO_GROUP)
                && !options.is_present(options::format::LONG_NO_GROUP);
            let owner = !options.is_present(options::format::LONG_NO_OWNER);
            #[cfg(unix)]
            let numeric_uid_gid = options.is_present(options::format::LONG_NUMERIC_UID_GID);
            LongFormat {
                author,
                group,
                owner,
                #[cfg(unix)]
                numeric_uid_gid,
            }
        };

        let width = match options.value_of(options::WIDTH) {
            Some(x) => match x.parse::<u16>() {
                Ok(u) => Some(u),
                Err(_) => return Err(LsError::InvalidLineWidth(x.into()).into()),
            },
            None => termsize::get().map(|s| s.cols),
        };

        #[allow(clippy::needless_bool)]
        let show_control = if options.is_present(options::HIDE_CONTROL_CHARS) {
            false
        } else if options.is_present(options::SHOW_CONTROL_CHARS) || atty::is(atty::Stream::Stdout)
        {
            true
        } else {
            false
        };

        let quoting_style = if let Some(style) = options.value_of(options::QUOTING_STYLE) {
            match style {
                "literal" => QuotingStyle::Literal { show_control },
                "shell" => QuotingStyle::Shell {
                    escape: false,
                    always_quote: false,
                    show_control,
                },
                "shell-always" => QuotingStyle::Shell {
                    escape: false,
                    always_quote: true,
                    show_control,
                },
                "shell-escape" => QuotingStyle::Shell {
                    escape: true,
                    always_quote: false,
                    show_control,
                },
                "shell-escape-always" => QuotingStyle::Shell {
                    escape: true,
                    always_quote: true,
                    show_control,
                },
                "c" => QuotingStyle::C {
                    quotes: quoting_style::Quotes::Double,
                },
                "escape" => QuotingStyle::C {
                    quotes: quoting_style::Quotes::None,
                },
                _ => unreachable!("Should have been caught by Clap"),
            }
        } else if options.is_present(options::quoting::LITERAL) {
            QuotingStyle::Literal { show_control }
        } else if options.is_present(options::quoting::ESCAPE) {
            QuotingStyle::C {
                quotes: quoting_style::Quotes::None,
            }
        } else if options.is_present(options::quoting::C) {
            QuotingStyle::C {
                quotes: quoting_style::Quotes::Double,
            }
        } else {
            // TODO: use environment variable if available
            QuotingStyle::Shell {
                escape: true,
                always_quote: false,
                show_control,
            }
        };

        let indicator_style = if let Some(field) = options.value_of(options::INDICATOR_STYLE) {
            match field {
                "none" => IndicatorStyle::None,
                "file-type" => IndicatorStyle::FileType,
                "classify" => IndicatorStyle::Classify,
                "slash" => IndicatorStyle::Slash,
                &_ => IndicatorStyle::None,
            }
        } else if options.is_present(options::indicator_style::CLASSIFY) {
            IndicatorStyle::Classify
        } else if options.is_present(options::indicator_style::SLASH) {
            IndicatorStyle::Slash
        } else if options.is_present(options::indicator_style::FILE_TYPE) {
            IndicatorStyle::FileType
        } else {
            IndicatorStyle::None
        };

        let time_style = if let Some(field) = options.value_of(options::TIME_STYLE) {
            //If both FULL_TIME and TIME_STYLE are present
            //The one added last is dominant
            if options.is_present(options::FULL_TIME)
                && options.indices_of(options::FULL_TIME).unwrap().last()
                    > options.indices_of(options::TIME_STYLE).unwrap().last()
            {
                TimeStyle::FullIso
            } else {
                //Clap handles the env variable "TIME_STYLE"
                match field {
                    "full-iso" => TimeStyle::FullIso,
                    "long-iso" => TimeStyle::LongIso,
                    "iso" => TimeStyle::Iso,
                    "locale" => TimeStyle::Locale,
                    // below should never happen as clap already restricts the values.
                    _ => unreachable!("Invalid field for --time-style"),
                }
            }
        } else if options.is_present(options::FULL_TIME) {
            TimeStyle::FullIso
        } else {
            TimeStyle::Locale
        };
        let mut ignore_patterns = GlobSetBuilder::new();
        if options.is_present(options::IGNORE_BACKUPS) {
            ignore_patterns.add(Glob::new("*~").unwrap());
            ignore_patterns.add(Glob::new(".*~").unwrap());
        }

        for pattern in options.values_of(options::IGNORE).into_iter().flatten() {
            match Glob::new(pattern) {
                Ok(p) => {
                    ignore_patterns.add(p);
                }
                Err(_) => show_warning!("Invalid pattern for ignore: '{}'", pattern),
            }
        }

        if files == Files::Normal {
            for pattern in options.values_of(options::HIDE).into_iter().flatten() {
                match Glob::new(pattern) {
                    Ok(p) => {
                        ignore_patterns.add(p);
                    }
                    Err(_) => show_warning!("Invalid pattern for hide: '{}'", pattern),
                }
            }
        }

        if files == Files::Normal {
            ignore_patterns.add(Glob::new(".*").unwrap());
        }

        let ignore_patterns = ignore_patterns.build().unwrap();

        let dereference = if options.is_present(options::dereference::ALL) {
            Dereference::All
        } else if options.is_present(options::dereference::ARGS) {
            Dereference::Args
        } else if options.is_present(options::dereference::DIR_ARGS) {
            Dereference::DirArgs
        } else if options.is_present(options::DIRECTORY)
            || indicator_style == IndicatorStyle::Classify
            || format == Format::Long
        {
            Dereference::None
        } else {
            Dereference::DirArgs
        };

        Ok(Config {
            format,
            files,
            sort,
            recursive: options.is_present(options::RECURSIVE),
            reverse: options.is_present(options::REVERSE),
            dereference,
            ignore_patterns,
            size_format,
            directory: options.is_present(options::DIRECTORY),
            time,
            color,
            #[cfg(unix)]
            inode: options.is_present(options::INODE),
            blocks: options.is_present(options::BLOCKS),
            long,
            width,
            quoting_style,
            indicator_style,
            time_style,
        })
    }
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let usage = get_usage();

    let app = uu_app().usage(&usage[..]);

    let matches = app.get_matches_from(args);

    let locs = matches
        .values_of(options::PATHS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_else(|| vec![String::from(".")]);

    list(locs, Config::from(matches)?)
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(crate_version!())
        .about(
            "By default, ls will list the files and contents of any directories on \
            the command line, expect that it will ignore files and directories \
            whose names start with '.'.",
        )
        // Format arguments
        .arg(
            Arg::with_name(options::FORMAT)
                .long(options::FORMAT)
                .help("Set the display format.")
                .takes_value(true)
                .possible_values(&[
                    "long",
                    "verbose",
                    "single-column",
                    "columns",
                    "vertical",
                    "across",
                    "horizontal",
                    "commas",
                ])
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::COLUMNS)
                .short(options::format::COLUMNS)
                .help("Display the files in columns.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::LONG)
                .short("l")
                .long(options::format::LONG)
                .help("Display detailed information.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::ACROSS)
                .short(options::format::ACROSS)
                .help("List entries in rows instead of in columns.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::COMMAS)
                .short(options::format::COMMAS)
                .help("List entries separated by commas.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        // The next four arguments do not override with the other format
        // options, see the comment in Config::from for the reason.
        // Ideally, they would use Arg::override_with, with their own name
        // but that doesn't seem to work in all cases. Example:
        // ls -1g1
        // even though `ls -11` and `ls -1 -g -1` work.
        .arg(
            Arg::with_name(options::format::ONE_LINE)
                .short(options::format::ONE_LINE)
                .help("List one file per line.")
                .multiple(true),
        )
        .arg(
            Arg::with_name(options::format::LONG_NO_GROUP)
                .short(options::format::LONG_NO_GROUP)
                .help(
                    "Long format without group information. \
                    Identical to --format=long with --no-group.",
                )
                .multiple(true),
        )
        .arg(
            Arg::with_name(options::format::LONG_NO_OWNER)
                .short(options::format::LONG_NO_OWNER)
                .help("Long format without owner information.")
                .multiple(true),
        )
        .arg(
            Arg::with_name(options::format::LONG_NUMERIC_UID_GID)
                .short("n")
                .long(options::format::LONG_NUMERIC_UID_GID)
                .help("-l with numeric UIDs and GIDs.")
                .multiple(true),
        )
        // Quoting style
        .arg(
            Arg::with_name(options::QUOTING_STYLE)
                .long(options::QUOTING_STYLE)
                .takes_value(true)
                .help("Set quoting style.")
                .possible_values(&[
                    "literal",
                    "shell",
                    "shell-always",
                    "shell-escape",
                    "shell-escape-always",
                    "c",
                    "escape",
                ])
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ]),
        )
        .arg(
            Arg::with_name(options::quoting::LITERAL)
                .short("N")
                .long(options::quoting::LITERAL)
                .help("Use literal quoting style. Equivalent to `--quoting-style=literal`")
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ]),
        )
        .arg(
            Arg::with_name(options::quoting::ESCAPE)
                .short("b")
                .long(options::quoting::ESCAPE)
                .help("Use escape quoting style. Equivalent to `--quoting-style=escape`")
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ]),
        )
        .arg(
            Arg::with_name(options::quoting::C)
                .short("Q")
                .long(options::quoting::C)
                .help("Use C quoting style. Equivalent to `--quoting-style=c`")
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ]),
        )
        // Control characters
        .arg(
            Arg::with_name(options::HIDE_CONTROL_CHARS)
                .short("q")
                .long(options::HIDE_CONTROL_CHARS)
                .help("Replace control characters with '?' if they are not escaped.")
                .overrides_with_all(&[options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS]),
        )
        .arg(
            Arg::with_name(options::SHOW_CONTROL_CHARS)
                .long(options::SHOW_CONTROL_CHARS)
                .help("Show control characters 'as is' if they are not escaped.")
                .overrides_with_all(&[options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS]),
        )
        // Time arguments
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .help(
                    "Show time in <field>:\n\
                    \taccess time (-u): atime, access, use;\n\
                    \tchange time (-t): ctime, status.\n\
                    \tbirth time: birth, creation;",
                )
                .value_name("field")
                .takes_value(true)
                .possible_values(&[
                    "atime", "access", "use", "ctime", "status", "birth", "creation",
                ])
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all(&[options::TIME, options::time::ACCESS, options::time::CHANGE]),
        )
        .arg(
            Arg::with_name(options::time::CHANGE)
                .short(options::time::CHANGE)
                .help(
                    "If the long listing format (e.g., -l, -o) is being used, print the status \
                change time (the 'ctime' in the inode) instead of the modification time. When \
                explicitly sorting by time (--sort=time or -t) or when not using a long listing \
                format, sort according to the status change time.",
                )
                .overrides_with_all(&[options::TIME, options::time::ACCESS, options::time::CHANGE]),
        )
        .arg(
            Arg::with_name(options::time::ACCESS)
                .short(options::time::ACCESS)
                .help(
                    "If the long listing format (e.g., -l, -o) is being used, print the status \
                access time instead of the modification time. When explicitly sorting by time \
                (--sort=time or -t) or when not using a long listing format, sort according to the \
                access time.",
                )
                .overrides_with_all(&[options::TIME, options::time::ACCESS, options::time::CHANGE]),
        )
        // Hide and ignore
        .arg(
            Arg::with_name(options::HIDE)
                .long(options::HIDE)
                .takes_value(true)
                .multiple(true)
                .value_name("PATTERN")
                .help(
                    "do not list implied entries matching shell PATTERN (overridden by -a or -A)",
                ),
        )
        .arg(
            Arg::with_name(options::IGNORE)
                .short("I")
                .long(options::IGNORE)
                .takes_value(true)
                .multiple(true)
                .value_name("PATTERN")
                .help("do not list implied entries matching shell PATTERN"),
        )
        .arg(
            Arg::with_name(options::IGNORE_BACKUPS)
                .short("B")
                .long(options::IGNORE_BACKUPS)
                .help("Ignore entries which end with ~."),
        )
        // Sort arguments
        .arg(
            Arg::with_name(options::SORT)
                .long(options::SORT)
                .help("Sort by <field>: name, none (-U), time (-t), size (-S) or extension (-X)")
                .value_name("field")
                .takes_value(true)
                .possible_values(&["name", "none", "time", "size", "version", "extension"])
                .require_equals(true)
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        .arg(
            Arg::with_name(options::sort::SIZE)
                .short(options::sort::SIZE)
                .help("Sort by file size, largest first.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        .arg(
            Arg::with_name(options::sort::TIME)
                .short(options::sort::TIME)
                .help("Sort by modification time (the 'mtime' in the inode), newest first.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        .arg(
            Arg::with_name(options::sort::VERSION)
                .short(options::sort::VERSION)
                .help("Natural sort of (version) numbers in the filenames.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        .arg(
            Arg::with_name(options::sort::EXTENSION)
                .short(options::sort::EXTENSION)
                .help("Sort alphabetically by entry extension.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        .arg(
            Arg::with_name(options::sort::NONE)
                .short(options::sort::NONE)
                .help(
                    "Do not sort; list the files in whatever order they are stored in the \
    directory.  This is especially useful when listing very large directories, \
    since not doing any sorting can be noticeably faster.",
                )
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        // Dereferencing
        .arg(
            Arg::with_name(options::dereference::ALL)
                .short("L")
                .long(options::dereference::ALL)
                .help(
                    "When showing file information for a symbolic link, show information for the \
                file the link references rather than the link itself.",
                )
                .overrides_with_all(&[
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ]),
        )
        .arg(
            Arg::with_name(options::dereference::DIR_ARGS)
                .long(options::dereference::DIR_ARGS)
                .help(
                    "Do not dereference symlinks except when they link to directories and are \
                given as command line arguments.",
                )
                .overrides_with_all(&[
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ]),
        )
        .arg(
            Arg::with_name(options::dereference::ARGS)
                .short("H")
                .long(options::dereference::ARGS)
                .help("Do not dereference symlinks except when given as command line arguments.")
                .overrides_with_all(&[
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ]),
        )
        // Long format options
        .arg(
            Arg::with_name(options::NO_GROUP)
                .long(options::NO_GROUP)
                .short("-G")
                .help("Do not show group in long format."),
        )
        .arg(Arg::with_name(options::AUTHOR).long(options::AUTHOR).help(
            "Show author in long format. \
            On the supported platforms, the author always matches the file owner.",
        ))
        // Other Flags
        .arg(
            Arg::with_name(options::files::ALL)
                .short("a")
                .long(options::files::ALL)
                .help("Do not ignore hidden files (files with names that start with '.')."),
        )
        .arg(
            Arg::with_name(options::files::ALMOST_ALL)
                .short("A")
                .long(options::files::ALMOST_ALL)
                .help(
                    "In a directory, do not ignore all file names that start with '.', \
only ignore '.' and '..'.",
                ),
        )
        .arg(
            Arg::with_name(options::DIRECTORY)
                .short("d")
                .long(options::DIRECTORY)
                .help(
                    "Only list the names of directories, rather than listing directory contents. \
                This will not follow symbolic links unless one of `--dereference-command-line \
                (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is \
                specified.",
                ),
        )
        .arg(
            Arg::with_name(options::size::HUMAN_READABLE)
                .short("h")
                .long(options::size::HUMAN_READABLE)
                .help("Print human readable file sizes (e.g. 1K 234M 56G).")
                .overrides_with(options::size::SI),
        )
        .arg(
            Arg::with_name(options::size::SI)
                .long(options::size::SI)
                .help("Print human readable file sizes using powers of 1000 instead of 1024."),
        )
        .arg(
            Arg::with_name(options::INODE)
                .short("i")
                .long(options::INODE)
                .help("print the index number of each file"),
        )
        .arg(
            Arg::with_name(options::BLOCKS)
                .short("s")
                .long(options::BLOCKS)
                .help("print the number of file system blocks of each file"),
        )
        .arg(
            Arg::with_name(options::REVERSE)
                .short("r")
                .long(options::REVERSE)
                .help(
                    "Reverse whatever the sorting method is e.g., list files in reverse \
                alphabetical order, youngest first, smallest first, or whatever.",
                ),
        )
        .arg(
            Arg::with_name(options::RECURSIVE)
                .short("R")
                .long(options::RECURSIVE)
                .help("List the contents of all directories recursively."),
        )
        .arg(
            Arg::with_name(options::WIDTH)
                .long(options::WIDTH)
                .short("w")
                .help("Assume that the terminal is COLS columns wide.")
                .value_name("COLS")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::COLOR)
                .long(options::COLOR)
                .help("Color output based on file type.")
                .takes_value(true)
                .require_equals(true)
                .min_values(0),
        )
        .arg(
            Arg::with_name(options::INDICATOR_STYLE)
                .long(options::INDICATOR_STYLE)
                .help(
                    "Append indicator with style WORD to entry names: \
                    none (default),  slash (-p), file-type (--file-type), classify (-F)",
                )
                .takes_value(true)
                .possible_values(&["none", "slash", "file-type", "classify"])
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            Arg::with_name(options::indicator_style::CLASSIFY)
                .short("F")
                .long(options::indicator_style::CLASSIFY)
                .help(
                    "Append a character to each file name indicating the file type. Also, for \
                regular files that are executable, append '*'. The file type indicators are \
                '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
                '>' for doors, and nothing for regular files.",
                )
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            Arg::with_name(options::indicator_style::FILE_TYPE)
                .long(options::indicator_style::FILE_TYPE)
                .help("Same as --classify, but do not append '*'")
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            Arg::with_name(options::indicator_style::SLASH)
                .short(options::indicator_style::SLASH)
                .help("Append / indicator to directories.")
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            //This still needs support for posix-*, +FORMAT
            Arg::with_name(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .help("time/date format with -l; see TIME_STYLE below")
                .value_name("TIME_STYLE")
                .env("TIME_STYLE")
                .possible_values(&["full-iso", "long-iso", "iso", "locale"])
                .overrides_with_all(&[options::TIME_STYLE]),
        )
        .arg(
            Arg::with_name(options::FULL_TIME)
                .long(options::FULL_TIME)
                .overrides_with(options::FULL_TIME)
                .help("like -l --time-style=full-iso"),
        )
        // Positional arguments
        .arg(
            Arg::with_name(options::PATHS)
                .multiple(true)
                .takes_value(true),
        )
        .after_help(
            "The TIME_STYLE argument can be full-iso, long-iso, iso. \
            Also the TIME_STYLE environment variable sets the default style to use.",
        )
}

/// Represents a Path along with it's associated data
/// Any data that will be reused several times makes sense to be added to this structure
/// Caching data here helps eliminate redundant syscalls to fetch same information
struct PathData {
    // Result<MetaData> got from symlink_metadata() or metadata() based on config
    md: OnceCell<Option<Metadata>>,
    ft: OnceCell<Option<FileType>>,
    // Name of the file - will be empty for . or ..
    display_name: String,
    // PathBuf that all above data corresponds to
    p_buf: PathBuf,
    must_dereference: bool,
}

impl PathData {
    fn new(
        p_buf: PathBuf,
        file_type: Option<std::io::Result<FileType>>,
        file_name: Option<String>,
        config: &Config,
        command_line: bool,
    ) -> Self {
        // We cannot use `Path::ends_with` or `Path::Components`, because they remove occurrences of '.'
        // For '..', the filename is None
        let display_name = if let Some(name) = file_name {
            name
        } else {
            let display_os_str = if command_line {
                p_buf.as_os_str()
            } else {
                p_buf
                    .file_name()
                    .unwrap_or_else(|| p_buf.iter().next_back().unwrap())
            };

            display_os_str.to_string_lossy().into_owned()
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
        let ft = match file_type {
            Some(ft) => OnceCell::from(ft.ok()),
            None => OnceCell::new(),
        };

        Self {
            md: OnceCell::new(),
            ft,
            display_name,
            p_buf,
            must_dereference,
        }
    }

    fn md(&self) -> Option<&Metadata> {
        self.md
            .get_or_init(|| get_metadata(&self.p_buf, self.must_dereference).ok())
            .as_ref()
    }

    fn file_type(&self) -> Option<&FileType> {
        self.ft
            .get_or_init(|| self.md().map(|md| md.file_type()))
            .as_ref()
    }
}

fn list(locs: Vec<String>, config: Config) -> UResult<()> {
    let mut files = Vec::<PathData>::new();
    let mut dirs = Vec::<PathData>::new();

    let mut out = BufWriter::new(stdout());

    for loc in &locs {
        let p = PathBuf::from(&loc);
        let path_data = PathData::new(p, None, None, &config, true);

        if path_data.md().is_none() {
            show!(std::io::ErrorKind::NotFound
                .map_err_context(|| format!("cannot access '{}'", path_data.p_buf.display())));
            // We found an error, no need to continue the execution
            continue;
        }

        let show_dir_contents = match path_data.file_type() {
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
    sort_entries(&mut files, &config);
    display_items(&files, &config, &mut out);

    sort_entries(&mut dirs, &config);
    for dir in dirs {
        if locs.len() > 1 || config.recursive {
            let _ = writeln!(out, "\n{}:", dir.p_buf.display());
        }
        enter_directory(&dir, &config, &mut out);
    }

    Ok(())
}

fn sort_entries(entries: &mut Vec<PathData>, config: &Config) {
    match config.sort {
        Sort::Time => entries.sort_by_key(|k| {
            Reverse(
                k.md()
                    .and_then(|md| get_system_time(md, config))
                    .unwrap_or(UNIX_EPOCH),
            )
        }),
        Sort::Size => {
            entries.sort_by_key(|k| Reverse(k.md().as_ref().map(|md| md.len()).unwrap_or(0)))
        }
        // The default sort in GNU ls is case insensitive
        Sort::Name => entries.sort_by(|a, b| a.display_name.cmp(&b.display_name)),
        Sort::Version => entries
            .sort_by(|a, b| version_cmp(&a.p_buf.to_string_lossy(), &b.p_buf.to_string_lossy())),
        Sort::Extension => entries.sort_by(|a, b| {
            a.p_buf
                .extension()
                .cmp(&b.p_buf.extension())
                .then(a.p_buf.file_stem().cmp(&b.p_buf.file_stem()))
        }),
        Sort::None => {}
    }

    if config.reverse {
        entries.reverse();
    }
}

#[cfg(windows)]
fn is_hidden(file_path: &DirEntry) -> bool {
    let path = file_path.path();
    let metadata = fs::metadata(&path).unwrap_or_else(|_| fs::symlink_metadata(&path).unwrap());
    let attr = metadata.file_attributes();
    (attr & 0x2) > 0
}

fn should_display(entry: &DirEntry, config: &Config) -> bool {
    let ffi_name = entry.file_name();

    // For unix, the hidden files are already included in the ignore pattern
    #[cfg(windows)]
    {
        if config.files == Files::Normal && is_hidden(entry) {
            return false;
        }
    }

    !config.ignore_patterns.is_match(&ffi_name)
}

fn enter_directory(dir: &PathData, config: &Config, out: &mut BufWriter<Stdout>) {
    let mut entries: Vec<_> = if config.files == Files::All {
        vec![
            PathData::new(
                dir.p_buf.clone(),
                Some(Ok(*dir.file_type().unwrap())),
                Some(".".into()),
                config,
                false,
            ),
            PathData::new(dir.p_buf.join(".."), None, Some("..".into()), config, false),
        ]
    } else {
        vec![]
    };

    let mut temp: Vec<_> = safe_unwrap!(fs::read_dir(&dir.p_buf))
        .map(|res| safe_unwrap!(res))
        .filter(|e| should_display(e, config))
        .map(|e| PathData::new(DirEntry::path(&e), Some(e.file_type()), None, config, false))
        .collect();

    sort_entries(&mut temp, config);

    entries.append(&mut temp);

    display_items(&entries, config, out);

    if config.recursive {
        for e in entries
            .iter()
            .skip(if config.files == Files::All { 2 } else { 0 })
            .filter(|p| p.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        {
            let _ = writeln!(out, "\n{}:", e.p_buf.display());
            enter_directory(e, config, out);
        }
    }
}

fn get_metadata(entry: &Path, dereference: bool) -> std::io::Result<Metadata> {
    if dereference {
        entry.metadata()
    } else {
        entry.symlink_metadata()
    }
}

fn get_display_size(md: &Metadata) -> u64 {
    #[cfg(windows)] {
        if md.is_dir() {
            return 0
        }
        return md.len() as u64;
    }

    #[cfg(unix)] {
        return md.blocks() as u64;
    }
}


// Get maximum string sizes for:
// - number of links of each items
// - size
// - blocks
fn get_max_sizes (items: &[PathData], config: &Config) -> (usize, usize, usize, u64) {
    let (mut max_links, mut max_width, mut max_blocks) = (1, 1, 1);
        let mut total_size = 0;

    for item in items {
        if let Some(md) = item.md() {
            max_links = display_symlink_count(md).len().max(max_links);
            max_width = display_size_or_rdev(md, config).len().max(max_width);
            max_blocks = get_display_size(md).to_string().len().max(max_blocks);
            total_size += get_block_size(md, config);
        }
    }

    return (max_links, max_width, max_blocks, total_size);
}

fn pad_left(string: String, count: usize) -> String {
    format!("{:>width$}", string, width = count)
}

fn display_items(items: &[PathData], config: &Config, out: &mut BufWriter<Stdout>) {
    let (max_links, max_width, max_blocks, total_size) = get_max_sizes(items, config);
    if config.format == Format::Long {

        if total_size > 0 {
            let _ = writeln!(out, "total {}", display_size(total_size, config));
        }

        for item in items {
            display_item_long(item, max_links, max_width, max_blocks, config, out);
        }
    } else {
        let names = items.iter().filter_map(|i| display_file_name(i, config, max_blocks));

        match (&config.format, config.width) {
            (Format::Columns, Some(width)) => {
                display_grid(names, width, Direction::TopToBottom, out)
            }
            (Format::Across, Some(width)) => {
                display_grid(names, width, Direction::LeftToRight, out)
            }
            (Format::Commas, width_opt) => {
                let term_width = width_opt.unwrap_or(1);
                let mut current_col = 0;
                let mut names = names;
                if let Some(name) = names.next() {
                    let _ = write!(out, "{}", name.contents);
                    current_col = name.width as u16 + 2;
                }
                for name in names {
                    let name_width = name.width as u16;
                    if current_col + name_width + 1 > term_width {
                        current_col = name_width + 2;
                        let _ = write!(out, ",\n{}", name.contents);
                    } else {
                        current_col += name_width + 2;
                        let _ = write!(out, ", {}", name.contents);
                    }
                }
                // Current col is never zero again if names have been printed.
                // So we print a newline.
                if current_col > 0 {
                    let _ = writeln!(out,);
                }
            }
            _ => {
                for name in names {
                    let _ = writeln!(out, "{}", name.contents);
                }
            }
        }
    }
}

fn get_block_size(md: &Metadata, config: &Config) -> u64 {
    /* GNU ls will display sizes in terms of block size
       md.len() will differ from this value when the file has some holes
    */
    #[cfg(unix)]
    {
        // hard-coded for now - enabling setting this remains a TODO
        let ls_block_size = 1024;
        match config.size_format {
            SizeFormat::Binary => md.blocks() * 512,
            SizeFormat::Decimal => md.blocks() * 512,
            SizeFormat::Bytes => md.blocks() * 512 / ls_block_size,
        }
    }

    #[cfg(not(unix))]
    {
        let _ = config;
        // no way to get block size for windows, fall-back to file size
        md.len()
    }
}

fn display_grid(
    names: impl Iterator<Item = Cell>,
    width: u16,
    direction: Direction,
    out: &mut BufWriter<Stdout>,
) {
    let mut grid = Grid::new(GridOptions {
        filling: Filling::Spaces(2),
        direction,
    });

    for name in names {
        grid.add(name);
    }

    match grid.fit_into_width(width as usize) {
        Some(output) => {
            let _ = write!(out, "{}", output);
        }
        // Width is too small for the grid, so we fit it in one column
        None => {
            let _ = write!(out, "{}", grid.fit_into_columns(1));
        }
    }
}

fn display_item_long(
    item: &PathData,
    max_links: usize,
    max_size: usize,
    max_blocks: usize,
    config: &Config,
    out: &mut BufWriter<Stdout>,
) {
    let md = match item.md() {
        None => {
            show!(LsError::NoMetadata(item.p_buf.clone()));
            return;
        }
        Some(md) => md,
    };

    #[cfg(unix)]
    {
        if config.inode {
            let _ = write!(out, "{} ", get_inode(md));
        }
    }

    if config.blocks {
        let _ = write!(
            out,
            "{} ",
            pad_left(get_display_size(md).to_string(), max_blocks)
        );
    }

    let _ = write!(
        out,
        "{} {}",
        display_permissions(md, true),
        pad_left(display_symlink_count(md), max_links),
    );

    if config.long.owner {
        let _ = write!(out, " {}", display_uname(md, config));
    }

    if config.long.group {
        let _ = write!(out, " {}", display_group(md, config));
    }

    // Author is only different from owner on GNU/Hurd, so we reuse
    // the owner, since GNU/Hurd is not currently supported by Rust.
    if config.long.author {
        let _ = write!(out, " {}", display_uname(md, config));
    }

    let _ = writeln!(
        out,
        " {} {} {}",
        pad_left(display_size_or_rdev(md, config), max_size),
        display_date(md, config),
        // unwrap is fine because it fails when metadata is not available
        // but we already know that it is because it's checked at the
        // start of the function.
        display_file_name(item, config, 1).unwrap().contents,
    );
}

#[cfg(unix)]
fn get_inode(metadata: &Metadata) -> String {
    format!("{:8}", metadata.ino())
}


// Currently getpwuid is `linux` target only. If it's broken out into
// a posix-compliant attribute this can be updated...
#[cfg(unix)]
use std::sync::Mutex;
#[cfg(unix)]
use uucore::entries;
use uucore::InvalidEncodingHandling;

#[cfg(unix)]
fn cached_uid2usr(uid: u32) -> String {
    lazy_static! {
        static ref UID_CACHE: Mutex<HashMap<u32, String>> = Mutex::new(HashMap::new());
    }

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

#[cfg(unix)]
fn cached_gid2grp(gid: u32) -> String {
    lazy_static! {
        static ref GID_CACHE: Mutex<HashMap<u32, String>> = Mutex::new(HashMap::new());
    }

    let mut gid_cache = GID_CACHE.lock().unwrap();
    gid_cache
        .entry(gid)
        .or_insert_with(|| entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()))
        .clone()
}

#[cfg(unix)]
fn display_group(metadata: &Metadata, config: &Config) -> String {
    if config.long.numeric_uid_gid {
        metadata.gid().to_string()
    } else {
        cached_gid2grp(metadata.gid())
    }
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
            let recent = time + chrono::Duration::seconds(31_556_952 / 2) > chrono::Local::now();

            match config.time_style {
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
            }
            .to_string()
        }
        None => "???".into(),
    }
}

// There are a few peculiarities to how GNU formats the sizes:
// 1. One decimal place is given if and only if the size is smaller than 10
// 2. It rounds sizes up.
// 3. The human-readable format uses powers for 1024, but does not display the "i"
//    that is commonly used to denote Kibi, Mebi, etc.
// 4. Kibi and Kilo are denoted differently ("k" and "K", respectively)
fn format_prefixed(prefixed: NumberPrefix<f64>) -> String {
    match prefixed {
        NumberPrefix::Standalone(bytes) => bytes.to_string(),
        NumberPrefix::Prefixed(prefix, bytes) => {
            // Remove the "i" from "Ki", "Mi", etc. if present
            let prefix_str = prefix.symbol().trim_end_matches('i');

            // Check whether we get more than 10 if we round up to the first decimal
            // because we want do display 9.81 as "9.9", not as "10".
            if (10.0 * bytes).ceil() >= 100.0 {
                format!("{:.0}{}", bytes.ceil(), prefix_str)
            } else {
                format!("{:.1}{}", (10.0 * bytes).ceil() / 10.0, prefix_str)
            }
        }
    }
}

fn display_size_or_rdev(metadata: &Metadata, config: &Config) -> String {
    #[cfg(unix)]
    {
        let ft = metadata.file_type();
        if ft.is_char_device() || ft.is_block_device() {
            let dev: u64 = metadata.rdev();
            let major = (dev >> 8) as u8;
            let minor = dev as u8;
            return format!("{}, {}", major, minor);
        }
    }

    display_size(metadata.len(), config)
}

fn display_size(size: u64, config: &Config) -> String {
    // NOTE: The human-readable behavior deviates from the GNU ls.
    // The GNU ls uses binary prefixes by default.
    match config.size_format {
        SizeFormat::Binary => format_prefixed(NumberPrefix::binary(size as f64)),
        SizeFormat::Decimal => format_prefixed(NumberPrefix::decimal(size as f64)),
        SizeFormat::Bytes => size.to_string(),
    }
}

#[cfg(unix)]
fn file_is_executable(md: &Metadata) -> bool {
    // Mode always returns u32, but the flags might not be, based on the platform
    // e.g. linux has u32, mac has u16.
    // S_IXUSR -> user has execute permission
    // S_IXGRP -> group has execute permission
    // S_IXOTH -> other users have execute permission
    md.mode() & ((S_IXUSR | S_IXGRP | S_IXOTH) as u32) != 0
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
            } else if file_type.is_file() && file_is_executable(path.md()?) {
                Some('*')
            } else {
                None
            }
        }
        #[cfg(not(unix))]
        None
    }
}

fn display_file_name(path: &PathData, config: &Config, max_blocks: usize) -> Option<Cell> {
    let mut name = escape_name(&path.display_name, &config.quoting_style);

    #[cfg(unix)]
    {
        if config.format != Format::Long && config.inode {
            name = path
                .md()
                .map_or_else(|| "?".to_string(), |md| get_inode(md))
                + " "
                + &name;
        }
    }

    // We need to keep track of the width ourselves instead of letting term_grid
    // infer it because the color codes mess up term_grid's width calculation.
    let mut width = name.width();

    if let Some(ls_colors) = &config.color {
        name = color_name(ls_colors, &path.p_buf, name, path.md()?);
    }

    if config.indicator_style != IndicatorStyle::None {
        let sym = classify_file(path);

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
            width += 1;
        }
    }

    if config.format != Format::Long && config.blocks {
        if let Some(md) = path.md() {
            width += max_blocks + 1;
            name.insert_str(0, &format!("{} ", pad_left(get_display_size(md).to_string(), max_blocks)));
        } else {
            width += max_blocks + 1;
            name.insert_str(0, &format!("{} ", pad_left(String::from(""), max_blocks)));
        }
    }


    if config.format == Format::Long && path.file_type()?.is_symlink() {
        if let Ok(target) = path.p_buf.read_link() {
            name.push_str(" -> ");
            name.push_str(&target.to_string_lossy());
        }
    }

    Some(Cell {
        contents: name,
        width,
    })
}

fn color_name(ls_colors: &LsColors, path: &Path, name: String, md: &Metadata) -> String {
    match ls_colors.style_for_path_with_metadata(path, Some(md)) {
        Some(style) => style.to_ansi_term_style().paint(name).to_string(),
        None => name,
    }
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
