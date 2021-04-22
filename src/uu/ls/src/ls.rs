// This file is part of the uutils coreutils package.
//
// (c) Jeremiah Peschka <jeremiah.peschka@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) cpio svgz webm somegroup nlink rmvb xspf

#[macro_use]
extern crate uucore;

mod quoting_style;
mod version_cmp;

use clap::{App, Arg};
use globset::{self, Glob, GlobSet, GlobSetBuilder};
use lscolors::LsColors;
use number_prefix::NumberPrefix;
use quoting_style::{escape_name, QuotingStyle};
use std::fs;
use std::fs::{DirEntry, FileType, Metadata};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{cmp::Reverse, process::exit};

use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use time::{strftime, Timespec};
#[cfg(unix)]
use uucore::libc::{S_IXGRP, S_IXOTH, S_IXUSR};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "
 By default, ls will list the files and contents of any directories on
 the command line, expect that it will ignore files and directories
 whose names start with '.'
";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

pub mod options {
    pub mod format {
        pub static ONELINE: &str = "1";
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
    pub static QUOTING_STYLE: &str = "quoting-style";

    pub mod indicator_style {
        pub static NONE: &str = "none";
        pub static SLASH: &str = "slash";
        pub static FILE_TYPE: &str = "file-type";
        pub static CLASSIFY: &str = "classify";
    }
    pub mod dereference {
        pub static ALL: &str = "dereference";
        pub static ARGS: &str = "dereference-command-line";
        pub static DIR_ARGS: &str = "dereference-command-line-symlink-to-dir";
    }
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
    pub static CLASSIFY: &str = "classify";
    pub static FILE_TYPE: &str = "file-type";
    pub static SLASH: &str = "p";
    pub static INODE: &str = "inode";
    pub static REVERSE: &str = "reverse";
    pub static RECURSIVE: &str = "recursive";
    pub static COLOR: &str = "color";
    pub static PATHS: &str = "paths";
    pub static INDICATOR_STYLE: &str = "indicator-style";
    pub static HIDE: &str = "hide";
    pub static IGNORE: &str = "ignore";
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
    color: Option<LsColors>,
    long: LongFormat,
    width: Option<u16>,
    quoting_style: QuotingStyle,
    indicator_style: IndicatorStyle,
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
    fn from(options: clap::ArgMatches) -> Config {
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
        // when switching to a different format option inbetween like this:
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
            ]
            .iter()
            .flat_map(|opt| options.indices_of(opt))
            .flatten()
            .any(|i| i >= idx)
            {
                format = Format::Long;
            } else if let Some(mut indices) = options.indices_of(options::format::ONELINE) {
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
        } else {
            Sort::Name
        };

        let time = if let Some(field) = options.value_of(options::TIME) {
            match field {
                "ctime" | "status" => Time::Change,
                "access" | "atime" | "use" => Time::Access,
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

        let width = options
            .value_of(options::WIDTH)
            .map(|x| {
                x.parse::<u16>().unwrap_or_else(|_e| {
                    show_error!("invalid line width: ‘{}’", x);
                    exit(2);
                })
            })
            .or_else(|| termsize::get().map(|s| s.cols));

        #[allow(clippy::needless_bool)]
        let show_control = if options.is_present(options::HIDE_CONTROL_CHARS) {
            false
        } else if options.is_present(options::SHOW_CONTROL_CHARS) {
            true
        } else {
            false // TODO: only if output is a terminal and the program is `ls`
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
        } else if options.is_present(options::indicator_style::NONE) {
            IndicatorStyle::None
        } else if options.is_present(options::indicator_style::CLASSIFY)
            || options.is_present(options::CLASSIFY)
        {
            IndicatorStyle::Classify
        } else if options.is_present(options::indicator_style::SLASH)
            || options.is_present(options::SLASH)
        {
            IndicatorStyle::Slash
        } else if options.is_present(options::indicator_style::FILE_TYPE)
            || options.is_present(options::FILE_TYPE)
        {
            IndicatorStyle::FileType
        } else {
            IndicatorStyle::None
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

        Config {
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
            long,
            width,
            quoting_style,
            indicator_style,
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let usage = get_usage();

    let app = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])

        // Format arguments
        .arg(
            Arg::with_name(options::FORMAT)
                .long(options::FORMAT)
                .help("Set the display format.")
                .takes_value(true)
                .possible_values(&["long", "verbose", "single-column", "columns", "vertical", "across", "horizontal", "commas"])
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
            Arg::with_name(options::format::ONELINE)
                .short(options::format::ONELINE)
                .help("List one file per line.")
                .multiple(true)
        )
        .arg(
            Arg::with_name(options::format::LONG_NO_GROUP)
                .short(options::format::LONG_NO_GROUP)
                .help("Long format without group information. Identical to --format=long with --no-group.")
                .multiple(true)
        )
        .arg(
            Arg::with_name(options::format::LONG_NO_OWNER)
                .short(options::format::LONG_NO_OWNER)
                .help("Long format without owner information.")
                .multiple(true)
        )
        .arg(
            Arg::with_name(options::format::LONG_NUMERIC_UID_GID)
                .short("n")
                .long(options::format::LONG_NUMERIC_UID_GID)
                .help("-l with numeric UIDs and GIDs.")
                .multiple(true)
        )

        // Quoting style
        .arg(
            Arg::with_name(options::QUOTING_STYLE)
                .long(options::QUOTING_STYLE)
                .takes_value(true)
                .help("Set quoting style.")
                .possible_values(&["literal", "shell", "shell-always", "shell-escape", "shell-escape-always", "c", "escape"])
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
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
                ])
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
                ])
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
                ])
        )

        // Control characters
        .arg(
            Arg::with_name(options::HIDE_CONTROL_CHARS)
                .short("q")
                .long(options::HIDE_CONTROL_CHARS)
                .help("Replace control characters with '?' if they are not escaped.")
                .overrides_with_all(&[
                    options::HIDE_CONTROL_CHARS,
                    options::SHOW_CONTROL_CHARS,
                ])
        )
        .arg(
            Arg::with_name(options::SHOW_CONTROL_CHARS)
                .long(options::SHOW_CONTROL_CHARS)
                .help("Show control characters 'as is' if they are not escaped.")
                .overrides_with_all(&[
                    options::HIDE_CONTROL_CHARS,
                    options::SHOW_CONTROL_CHARS,
                ])
        )

        // Time arguments
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .help("Show time in <field>:\n\
                    \taccess time (-u): atime, access, use;\n\
                    \tchange time (-t): ctime, status.")
                .value_name("field")
                .takes_value(true)
                .possible_values(&["atime", "access", "use", "ctime", "status"])
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all(&[
                    options::TIME,
                    options::time::ACCESS,
                    options::time::CHANGE,
                ])
        )
        .arg(
            Arg::with_name(options::time::CHANGE)
                .short(options::time::CHANGE)
                .help("If the long listing format (e.g., -l, -o) is being used, print the status \
                change time (the ‘ctime’ in the inode) instead of the modification time. When \
                explicitly sorting by time (--sort=time or -t) or when not using a long listing \
                format, sort according to the status change time.")
                .overrides_with_all(&[
                    options::TIME,
                    options::time::ACCESS,
                    options::time::CHANGE,
                ])
        )
        .arg(
            Arg::with_name(options::time::ACCESS)
                .short(options::time::ACCESS)
                .help("If the long listing format (e.g., -l, -o) is being used, print the status \
                access time instead of the modification time. When explicitly sorting by time \
                (--sort=time or -t) or when not using a long listing format, sort according to the \
                access time.")
                .overrides_with_all(&[
                    options::TIME,
                    options::time::ACCESS,
                    options::time::CHANGE,
                ])
        )

        // Hide and ignore
        .arg(
            Arg::with_name(options::HIDE)
                .long(options::HIDE)
                .takes_value(true)
                .multiple(true)
        )
        .arg(
            Arg::with_name(options::IGNORE)
                .short("I")
                .long(options::IGNORE)
                .takes_value(true)
                .multiple(true)
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
                .help("Sort by <field>: name, none (-U), time (-t) or size (-S)")
                .value_name("field")
                .takes_value(true)
                .possible_values(&["name", "none", "time", "size", "version"])
                .require_equals(true)
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                ])
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
                ])
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
                ])
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
                ])
        )
        .arg(
            Arg::with_name(options::sort::NONE)
                .short(options::sort::NONE)
                .help("Do not sort; list the files in whatever order they are stored in the \
                directory.  This is especially useful when listing very large directories, \
                since not doing any sorting can be noticeably faster.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                ])
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
                ])
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
                ])
        )
        .arg(
            Arg::with_name(options::dereference::ARGS)
                .short("H")
                .long(options::dereference::ARGS)
                .help(
                    "Do not dereference symlinks except when given as command line arguments.",
                )
                .overrides_with_all(&[
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
        )

        // Long format options
        .arg(
            Arg::with_name(options::NO_GROUP)
                .long(options::NO_GROUP)
                .short("-G")
                .help("Do not show group in long format.")
        )
        .arg(
            Arg::with_name(options::AUTHOR)
                .long(options::AUTHOR)
                .help("Show author in long format. On the supported platforms, the author \
                always matches the file owner.")
        )
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
                "In a directory, do not ignore all file names that start with '.', only ignore \
                '.' and '..'.",
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
                .help("Print human readable file sizes using powers of 1000 instead of 1024.")
        )
        .arg(
            Arg::with_name(options::INODE)
                .short("i")
                .long(options::INODE)
                .help("print the index number of each file"),
        )
        .arg(
            Arg::with_name(options::REVERSE)
                .short("r")
                .long(options::REVERSE)
                .help("Reverse whatever the sorting method is--e.g., list files in reverse \
                alphabetical order, youngest first, smallest first, or whatever.",
        ))
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
                .takes_value(true)
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
                .help(" append indicator with style WORD to entry names: none (default),  slash\
                       (-p), file-type (--file-type), classify (-F)")
                .takes_value(true)
                .possible_values(&["none", "slash", "file-type", "classify"])
                .overrides_with_all(&[
                    options::FILE_TYPE,
                    options::SLASH,
                    options::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]))
                .arg(
            Arg::with_name(options::CLASSIFY)
                .short("F")
                .long(options::CLASSIFY)
                .help("Append a character to each file name indicating the file type. Also, for \
                       regular files that are executable, append '*'. The file type indicators are \
                       '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
                       '>' for doors, and nothing for regular files.")
                .overrides_with_all(&[
                    options::FILE_TYPE,
                    options::SLASH,
                    options::CLASSIFY,
                    options::INDICATOR_STYLE,
                ])
        )
        .arg(
            Arg::with_name(options::FILE_TYPE)
                .long(options::FILE_TYPE)
                .help("Same as --classify, but do not append '*'")
                .overrides_with_all(&[
                    options::FILE_TYPE,
                    options::SLASH,
                    options::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]))
        .arg(
            Arg::with_name(options::SLASH)
                .short(options::SLASH)
                .help("Append / indicator to directories."
                )
                .overrides_with_all(&[
                    options::FILE_TYPE,
                    options::SLASH,
                    options::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]))

    // Positional arguments
        .arg(Arg::with_name(options::PATHS).multiple(true).takes_value(true));

    let matches = app.get_matches_from(args);

    let locs = matches
        .values_of(options::PATHS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_else(|| vec![String::from(".")]);

    list(locs, Config::from(matches))
}

fn list(locs: Vec<String>, config: Config) -> i32 {
    let number_of_locs = locs.len();

    let mut files = Vec::<PathBuf>::new();
    let mut dirs = Vec::<PathBuf>::new();
    let mut has_failed = false;
    for loc in locs {
        let p = PathBuf::from(&loc);
        if !p.exists() {
            show_error!("'{}': {}", &loc, "No such file or directory");
            // We found an error, the return code of ls should not be 0
            // And no need to continue the execution
            has_failed = true;
            continue;
        }

        let show_dir_contents = if !config.directory {
            match config.dereference {
                Dereference::None => {
                    if let Ok(md) = p.symlink_metadata() {
                        md.is_dir()
                    } else {
                        show_error!("'{}': {}", &loc, "No such file or directory");
                        has_failed = true;
                        continue;
                    }
                }
                _ => p.is_dir(),
            }
        } else {
            false
        };

        if show_dir_contents {
            dirs.push(p);
        } else {
            files.push(p);
        }
    }
    sort_entries(&mut files, &config);
    display_items(&files, None, &config, true);

    sort_entries(&mut dirs, &config);
    for dir in dirs {
        if number_of_locs > 1 {
            println!("\n{}:", dir.to_string_lossy());
        }
        enter_directory(&dir, &config);
    }
    if has_failed {
        1
    } else {
        0
    }
}

fn sort_entries(entries: &mut Vec<PathBuf>, config: &Config) {
    match config.sort {
        Sort::Time => entries.sort_by_key(|k| {
            Reverse(
                get_metadata(k, false)
                    .ok()
                    .and_then(|md| get_system_time(&md, config))
                    .unwrap_or(UNIX_EPOCH),
            )
        }),
        Sort::Size => {
            entries.sort_by_key(|k| Reverse(get_metadata(k, false).map(|md| md.len()).unwrap_or(0)))
        }
        // The default sort in GNU ls is case insensitive
        Sort::Name => entries.sort_by_key(|k| k.to_string_lossy().to_lowercase()),
        Sort::Version => entries.sort_by(|a, b| version_cmp::version_cmp(a, b)),
        Sort::None => {}
    }

    if config.reverse {
        entries.reverse();
    }
}

#[cfg(windows)]
fn is_hidden(file_path: &DirEntry) -> bool {
    let metadata = fs::metadata(file_path.path()).unwrap();
    let attr = metadata.file_attributes();
    ((attr & 0x2) > 0) || file_path.file_name().to_string_lossy().starts_with('.')
}

#[cfg(unix)]
fn is_hidden(file_path: &DirEntry) -> bool {
    file_path.file_name().to_string_lossy().starts_with('.')
}

fn should_display(entry: &DirEntry, config: &Config) -> bool {
    let ffi_name = entry.file_name();

    if config.files == Files::Normal && is_hidden(entry) {
        return false;
    }

    if config.ignore_patterns.is_match(&ffi_name) {
        return false;
    }
    true
}

fn enter_directory(dir: &Path, config: &Config) {
    let mut entries: Vec<_> = safe_unwrap!(fs::read_dir(dir).and_then(Iterator::collect));

    entries.retain(|e| should_display(e, config));

    let mut entries: Vec<_> = entries.iter().map(DirEntry::path).collect();
    sort_entries(&mut entries, config);

    if config.files == Files::All {
        let mut display_entries = entries.clone();
        display_entries.insert(0, dir.join(".."));
        display_entries.insert(0, dir.join("."));
        display_items(&display_entries, Some(dir), config, false);
    } else {
        display_items(&entries, Some(dir), config, false);
    }

    if config.recursive {
        for e in entries.iter().filter(|p| p.is_dir()) {
            println!("\n{}:", e.to_string_lossy());
            enter_directory(&e, config);
        }
    }
}

fn get_metadata(entry: &Path, dereference: bool) -> std::io::Result<Metadata> {
    if dereference {
        entry.metadata().or_else(|_| entry.symlink_metadata())
    } else {
        entry.symlink_metadata()
    }
}

fn display_dir_entry_size(entry: &Path, config: &Config) -> (usize, usize) {
    if let Ok(md) = get_metadata(entry, false) {
        (
            display_symlink_count(&md).len(),
            display_file_size(&md, config).len(),
        )
    } else {
        (0, 0)
    }
}

fn pad_left(string: String, count: usize) -> String {
    format!("{:>width$}", string, width = count)
}

fn display_items(items: &[PathBuf], strip: Option<&Path>, config: &Config, command_line: bool) {
    if config.format == Format::Long {
        let (mut max_links, mut max_size) = (1, 1);
        for item in items {
            let (links, size) = display_dir_entry_size(item, config);
            max_links = links.max(max_links);
            max_size = size.max(max_size);
        }
        for item in items {
            display_item_long(item, strip, max_links, max_size, config, command_line);
        }
    } else {
        let names = items.iter().filter_map(|i| {
            let md = get_metadata(i, false);
            match md {
                Err(e) => {
                    let filename = get_file_name(i, strip);
                    show_error!("'{}': {}", filename, e);
                    None
                }
                Ok(md) => Some(display_file_name(&i, strip, &md, config)),
            }
        });

        match (&config.format, config.width) {
            (Format::Columns, Some(width)) => display_grid(names, width, Direction::TopToBottom),
            (Format::Across, Some(width)) => display_grid(names, width, Direction::LeftToRight),
            (Format::Commas, width_opt) => {
                let term_width = width_opt.unwrap_or(1);
                let mut current_col = 0;
                let mut names = names;
                if let Some(name) = names.next() {
                    print!("{}", name.contents);
                    current_col = name.width as u16 + 2;
                }
                for name in names {
                    let name_width = name.width as u16;
                    if current_col + name_width + 1 > term_width {
                        current_col = name_width + 2;
                        print!(",\n{}", name.contents);
                    } else {
                        current_col += name_width + 2;
                        print!(", {}", name.contents);
                    }
                }
                // Current col is never zero again if names have been printed.
                // So we print a newline.
                if current_col > 0 {
                    println!();
                }
            }
            _ => {
                for name in names {
                    println!("{}", name.contents);
                }
            }
        }
    }
}

fn display_grid(names: impl Iterator<Item = Cell>, width: u16, direction: Direction) {
    let mut grid = Grid::new(GridOptions {
        filling: Filling::Spaces(2),
        direction,
    });

    for name in names {
        grid.add(name);
    }

    match grid.fit_into_width(width as usize) {
        Some(output) => print!("{}", output),
        // Width is too small for the grid, so we fit it in one column
        None => print!("{}", grid.fit_into_columns(1)),
    }
}

use uucore::fs::display_permissions;

fn display_item_long(
    item: &Path,
    strip: Option<&Path>,
    max_links: usize,
    max_size: usize,
    config: &Config,
    command_line: bool,
) {
    let dereference = match &config.dereference {
        Dereference::All => true,
        Dereference::Args => command_line,
        Dereference::DirArgs => {
            if command_line {
                if let Ok(md) = item.metadata() {
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

    let md = match get_metadata(item, dereference) {
        Err(e) => {
            let filename = get_file_name(&item, strip);
            show_error!("{}: {}", filename, e);
            return;
        }
        Ok(md) => md,
    };

    #[cfg(unix)]
    {
        if config.inode {
            print!("{} ", get_inode(&md));
        }
    }

    print!(
        "{}{} {}",
        display_file_type(md.file_type()),
        display_permissions(&md),
        pad_left(display_symlink_count(&md), max_links),
    );

    if config.long.owner {
        print!(" {}", display_uname(&md, config));
    }

    if config.long.group {
        print!(" {}", display_group(&md, config));
    }

    // Author is only different from owner on GNU/Hurd, so we reuse
    // the owner, since GNU/Hurd is not currently supported by Rust.
    if config.long.author {
        print!(" {}", display_uname(&md, config));
    }

    println!(
        " {} {} {}",
        pad_left(display_file_size(&md, config), max_size),
        display_date(&md, config),
        display_file_name(&item, strip, &md, config).contents,
    );
}

#[cfg(unix)]
fn get_inode(metadata: &Metadata) -> String {
    format!("{:8}", metadata.ino())
}

// Currently getpwuid is `linux` target only. If it's broken out into
// a posix-compliant attribute this can be updated...
#[cfg(unix)]
use uucore::entries;

#[cfg(unix)]
fn display_uname(metadata: &Metadata, config: &Config) -> String {
    if config.long.numeric_uid_gid {
        metadata.uid().to_string()
    } else {
        entries::uid2usr(metadata.uid()).unwrap_or_else(|_| metadata.uid().to_string())
    }
}

#[cfg(unix)]
fn display_group(metadata: &Metadata, config: &Config) -> String {
    if config.long.numeric_uid_gid {
        metadata.gid().to_string()
    } else {
        entries::gid2grp(metadata.gid()).unwrap_or_else(|_| metadata.gid().to_string())
    }
}

#[cfg(not(unix))]
fn display_uname(_metadata: &Metadata, _config: &Config) -> String {
    "somebody".to_string()
}

#[cfg(not(unix))]
#[allow(unused_variables)]
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
    }
}

#[cfg(not(unix))]
fn get_system_time(md: &Metadata, config: &Config) -> Option<SystemTime> {
    match config.time {
        Time::Modification => md.modified().ok(),
        Time::Access => md.accessed().ok(),
        _ => None,
    }
}

fn get_time(md: &Metadata, config: &Config) -> Option<time::Tm> {
    let duration = get_system_time(md, config)?
        .duration_since(UNIX_EPOCH)
        .ok()?;
    let secs = duration.as_secs() as i64;
    let nsec = duration.subsec_nanos() as i32;
    Some(time::at(Timespec::new(secs, nsec)))
}

fn display_date(metadata: &Metadata, config: &Config) -> String {
    match get_time(metadata, config) {
        Some(time) => strftime("%F %R", &time).unwrap(),
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

fn display_file_size(metadata: &Metadata, config: &Config) -> String {
    // NOTE: The human-readable behaviour deviates from the GNU ls.
    // The GNU ls uses binary prefixes by default.
    match config.size_format {
        SizeFormat::Binary => format_prefixed(NumberPrefix::binary(metadata.len() as f64)),
        SizeFormat::Decimal => format_prefixed(NumberPrefix::decimal(metadata.len() as f64)),
        SizeFormat::Bytes => metadata.len().to_string(),
    }
}

fn display_file_type(file_type: FileType) -> String {
    if file_type.is_dir() {
        "d".to_string()
    } else if file_type.is_symlink() {
        "l".to_string()
    } else {
        "-".to_string()
    }
}

fn get_file_name(name: &Path, strip: Option<&Path>) -> String {
    let mut name = match strip {
        Some(prefix) => name.strip_prefix(prefix).unwrap_or(name),
        None => name,
    };
    if name.as_os_str().is_empty() {
        name = Path::new(".");
    }
    name.to_string_lossy().into_owned()
}

#[cfg(unix)]
fn file_is_executable(md: &Metadata) -> bool {
    // Mode always returns u32, but the flags might not be, based on the platform
    // e.g. linux has u32, mac has u16.
    // S_IXUSR -> user has execute permission
    // S_IXGRP -> group has execute persmission
    // S_IXOTH -> other users have execute permission
    md.mode() & ((S_IXUSR | S_IXGRP | S_IXOTH) as u32) != 0
}

#[allow(clippy::clippy::collapsible_else_if)]
fn classify_file(md: &Metadata) -> Option<char> {
    let file_type = md.file_type();

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
            } else if file_type.is_file() && file_is_executable(&md) {
                Some('*')
            } else {
                None
            }
        }
        #[cfg(not(unix))]
        None
    }
}

fn display_file_name(
    path: &Path,
    strip: Option<&Path>,
    metadata: &Metadata,
    config: &Config,
) -> Cell {
    let mut name = escape_name(get_file_name(path, strip), &config.quoting_style);

    #[cfg(unix)]
    {
        if config.format != Format::Long && config.inode {
            name = get_inode(metadata) + " " + &name;
        }
    }

    if let Some(ls_colors) = &config.color {
        name = color_name(&ls_colors, path, name, metadata);
    }

    if config.indicator_style != IndicatorStyle::None {
        let sym = classify_file(metadata);

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

    if config.format == Format::Long && metadata.file_type().is_symlink() {
        if let Ok(target) = path.read_link() {
            // We don't bother updating width here because it's not used for long
            name.push_str(" -> ");
            name.push_str(&target.to_string_lossy());
        }
    }

    name.into()
}

fn color_name(ls_colors: &LsColors, path: &Path, name: String, md: &Metadata) -> String {
    match ls_colors.style_for_path_with_metadata(path, Some(&md)) {
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
