// This file is part of the uutils coreutils package.
//
// (c) Jeremiah Peschka <jeremiah.peschka@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) cpio svgz webm somegroup nlink rmvb xspf tabsize dired

#[macro_use]
extern crate uucore;

use clap::{
    builder::{NonEmptyStringValueParser, ValueParser},
    crate_version, Arg, ArgAction, Command,
};
use glob::{MatchOptions, Pattern};
use lscolors::LsColors;
use number_prefix::NumberPrefix;
use once_cell::unsync::OnceCell;
use std::collections::HashSet;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::{
    cmp::Reverse,
    error::Error,
    ffi::{OsStr, OsString},
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
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use unicode_width::UnicodeWidthStr;
#[cfg(unix)]
use uucore::libc::{S_IXGRP, S_IXOTH, S_IXUSR};
use uucore::parse_glob;
use uucore::quoting_style::{escape_name, QuotingStyle};
use uucore::{
    display::Quotable,
    error::{set_exit_code, UError, UResult},
    format_usage,
    fs::display_permissions,
    parse_size::parse_size,
    version_cmp::version_cmp,
};

#[cfg(not(feature = "selinux"))]
static CONTEXT_HELP_TEXT: &str = "print any security context of each file (not enabled)";
#[cfg(feature = "selinux")]
static CONTEXT_HELP_TEXT: &str = "print any security context of each file";

const ABOUT: &str = r#"List directory contents.
Ignore files and directories starting with a '.' by default"#;

const USAGE: &str = "{} [OPTION]... [FILE]...";

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
}

const DEFAULT_TERM_WIDTH: u16 = 80;
const POSIXLY_CORRECT_BLOCK_SIZE: u64 = 512;
#[cfg(unix)]
const DEFAULT_BLOCK_SIZE: u64 = 1024;

#[derive(Debug)]
enum LsError {
    InvalidLineWidth(String),
    IOError(std::io::Error),
    IOErrorContext(std::io::Error, PathBuf, bool),
    BlockSizeParseError(String),
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
            Self::BlockSizeParseError(_) => 1,
            Self::AlreadyListedError(_) => 2,
            Self::TimeStyleParseError(_, _) => 1,
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
            Self::TimeStyleParseError(s, possible_time_styles) => {
                write!(
                    f,
                    "invalid --time-style argument {}\nPossible values are: {:?}\n\nFor more information try --help",
                    s.quote(),
                    possible_time_styles
                )
            }
            Self::InvalidLineWidth(s) => write!(f, "invalid line width: {}", s.quote()),
            Self::IOError(e) => write!(f, "general io error: {}", e),
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
}

#[derive(PartialEq)]
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
    block_size: Option<u64>,
    width: u16,
    // Dir and vdir needs access to this field
    pub quoting_style: QuotingStyle,
    indicator_style: IndicatorStyle,
    time_style: TimeStyle,
    context: bool,
    selinux_supported: bool,
    group_directories_first: bool,
    eol: char,
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

impl Config {
    #[allow(clippy::cognitive_complexity)]
    pub fn from(options: &clap::ArgMatches) -> UResult<Self> {
        let context = options.get_flag(options::CONTEXT);
        let (mut format, opt) = if let Some(format_) = options.get_one::<String>(options::FORMAT) {
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
        } else if atty::is(atty::Stream::Stdout) {
            (Format::Columns, None)
        } else {
            (Format::OneLine, None)
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

        let files = if options.get_flag(options::files::ALL) {
            Files::All
        } else if options.get_flag(options::files::ALMOST_ALL) {
            Files::AlmostAll
        } else {
            Files::Normal
        };

        let sort = if let Some(field) = options.get_one::<String>(options::SORT) {
            match field.as_str() {
                "none" => Sort::None,
                "name" => Sort::Name,
                "time" => Sort::Time,
                "size" => Sort::Size,
                "version" => Sort::Version,
                "extension" => Sort::Extension,
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
        };

        let time = if let Some(field) = options.get_one::<String>(options::TIME) {
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
        };

        let mut needs_color = match options.get_one::<String>(options::COLOR) {
            None => options.contains_id(options::COLOR),
            Some(val) => match val.as_str() {
                "" | "always" | "yes" | "force" => true,
                "auto" | "tty" | "if-tty" => atty::is(atty::Stream::Stdout),
                /* "never" | "no" | "none" | */ _ => false,
            },
        };

        let cmd_line_bs = options.get_one::<String>(options::size::BLOCK_SIZE);
        let opt_si = cmd_line_bs.is_some()
            && options
                .get_one::<String>(options::size::BLOCK_SIZE)
                .unwrap()
                .eq("si")
            || options.get_flag(options::size::SI);
        let opt_hr = (cmd_line_bs.is_some()
            && options
                .get_one::<String>(options::size::BLOCK_SIZE)
                .unwrap()
                .eq("human-readable"))
            || options.get_flag(options::size::HUMAN_READABLE);
        let opt_kb = options.get_flag(options::size::KIBIBYTES);

        let bs_env_var = std::env::var_os("BLOCK_SIZE");
        let ls_bs_env_var = std::env::var_os("LS_BLOCK_SIZE");
        let pc_env_var = std::env::var_os("POSIXLY_CORRECT");

        let size_format = if opt_si {
            SizeFormat::Decimal
        } else if opt_hr {
            SizeFormat::Binary
        } else {
            SizeFormat::Bytes
        };

        let raw_bs = if let Some(cmd_line_bs) = cmd_line_bs {
            OsString::from(cmd_line_bs)
        } else if !opt_kb {
            if let Some(ls_bs_env_var) = ls_bs_env_var {
                ls_bs_env_var
            } else if let Some(bs_env_var) = bs_env_var {
                bs_env_var
            } else {
                OsString::from("")
            }
        } else {
            OsString::from("")
        };

        let block_size: Option<u64> = if !opt_si && !opt_hr && !raw_bs.is_empty() {
            match parse_size(&raw_bs.to_string_lossy()) {
                Ok(size) => Some(size),
                Err(_) => {
                    show!(LsError::BlockSizeParseError(
                        cmd_line_bs.unwrap().to_owned()
                    ));
                    None
                }
            }
        } else if let Some(pc) = pc_env_var {
            if pc.as_os_str() == OsStr::new("true") || pc == OsStr::new("1") {
                Some(POSIXLY_CORRECT_BLOCK_SIZE)
            } else {
                None
            }
        } else {
            None
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

        let width = match options.get_one::<String>(options::WIDTH) {
            Some(x) => {
                if x.starts_with('0') && x.len() > 1 {
                    // Read number as octal
                    match u16::from_str_radix(x, 8) {
                        Ok(v) => v,
                        Err(_) => return Err(LsError::InvalidLineWidth(x.into()).into()),
                    }
                } else {
                    match x.parse::<u16>() {
                        Ok(u) => u,
                        Err(_) => return Err(LsError::InvalidLineWidth(x.into()).into()),
                    }
                }
            }
            None => match terminal_size::terminal_size() {
                Some((width, _)) => width.0,
                None => match std::env::var_os("COLUMNS") {
                    Some(columns) => match columns.to_str().and_then(|s| s.parse().ok()) {
                        Some(columns) => columns,
                        None => {
                            show_error!(
                                "ignoring invalid width in environment variable COLUMNS: {}",
                                columns.quote()
                            );
                            DEFAULT_TERM_WIDTH
                        }
                    },
                    None => DEFAULT_TERM_WIDTH,
                },
            },
        };

        #[allow(clippy::needless_bool)]
        let mut show_control = if options.get_flag(options::HIDE_CONTROL_CHARS) {
            false
        } else if options.get_flag(options::SHOW_CONTROL_CHARS) {
            true
        } else {
            !atty::is(atty::Stream::Stdout)
        };

        let opt_quoting_style = options
            .get_one::<String>(options::QUOTING_STYLE)
            .map(|cmd_line_qs| cmd_line_qs.to_owned());

        let mut quoting_style = if let Some(style) = opt_quoting_style {
            match style.as_str() {
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
        } else {
            // TODO: use environment variable if available
            QuotingStyle::Shell {
                escape: true,
                always_quote: false,
                show_control,
            }
        };

        let indicator_style = if let Some(field) =
            options.get_one::<String>(options::INDICATOR_STYLE)
        {
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
                    if atty::is(atty::Stream::Stdout) {
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
        };
        let time_style = parse_time_style(options)?;

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
            eol: if options.get_flag(options::ZERO) {
                '\0'
            } else {
                '\n'
            },
        })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let command = uu_app();

    let matches = command.get_matches_from(args);

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
                .value_parser([
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
            Arg::new(options::format::COLUMNS)
                .short('C')
                .help("Display the files in columns.")
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .conflicts_with(options::DIRED)
                .overrides_with(options::ZERO)
                .help("List entries separated by ASCII NUL characters.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIRED)
                .long(options::DIRED)
                .short('D')
                .hide(true)
                .action(ArgAction::SetTrue),
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
                .value_parser([
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
            Arg::new(options::quoting::LITERAL)
                .short('N')
                .long(options::quoting::LITERAL)
                .help("Use literal quoting style. Equivalent to `--quoting-style=literal`")
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_CONTROL_CHARS)
                .long(options::SHOW_CONTROL_CHARS)
                .help("Show control characters 'as is' if they are not escaped.")
                .overrides_with_all(&[options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
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
                .value_parser([
                    "atime", "access", "use", "ctime", "status", "birth", "creation",
                ])
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all(&[options::TIME, options::time::ACCESS, options::time::CHANGE]),
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
                .overrides_with_all(&[options::TIME, options::time::ACCESS, options::time::CHANGE])
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
                .overrides_with_all(&[options::TIME, options::time::ACCESS, options::time::CHANGE])
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
                .help("Sort by <field>: name, none (-U), time (-t), size (-S) or extension (-X)")
                .value_name("field")
                .value_parser(["name", "none", "time", "size", "version", "extension"])
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
            Arg::new(options::sort::SIZE)
                .short('S')
                .help("Sort by file size, largest first.")
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                    "Do not dereference symlinks except when they link to directories and are \
                    given as command line arguments.",
                )
                .overrides_with_all(&[
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
                .help("Do not dereference symlinks except when given as command line arguments.")
                .overrides_with_all(&[
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
                .overrides_with(options::size::SI)
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
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::BLOCK_SIZE)
                .long(options::size::BLOCK_SIZE)
                .require_equals(true)
                .value_name("BLOCK_SIZE")
                .help("scale sizes by BLOCK_SIZE when printing them"),
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
                .value_parser([
                    "always", "yes", "force", "auto", "tty", "if-tty", "never", "no", "none",
                ])
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
                .value_parser(["none", "slash", "file-type", "classify"])
                .overrides_with_all(&[
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
                .value_parser([
                    "always", "yes", "force", "auto", "tty", "if-tty", "never", "no", "none",
                ])
                .default_missing_value("always")
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[
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
                .overrides_with_all(&[options::TIME_STYLE]),
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
        .after_help(
            "The TIME_STYLE argument can be full-iso, long-iso, iso, locale or +FORMAT. FORMAT is interpreted like in date. \
            Also the TIME_STYLE environment variable sets the default style to use.",
        )
}

/// Represents a Path along with it's associated data.
/// Any data that will be reused several times makes sense to be added to this structure.
/// Caching data here helps eliminate redundant syscalls to fetch same information.
#[derive(Debug)]
struct PathData {
    // Result<MetaData> got from symlink_metadata() or metadata() based on config
    md: OnceCell<Option<Metadata>>,
    ft: OnceCell<Option<FileType>>,
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

    fn md(&self, out: &mut BufWriter<Stdout>) -> Option<&Metadata> {
        self.md
            .get_or_init(|| {
                // check if we can use DirEntry metadata
                if !self.must_dereference {
                    if let Some(dir_entry) = &self.de {
                        return dir_entry.metadata().ok();
                    }
                }

                // if not, check if we can use Path metadata
                match get_metadata(self.p_buf.as_path(), self.must_dereference) {
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
            .get_or_init(|| self.md(out).map(|md| md.file_type()))
            .as_ref()
    }
}

pub fn list(locs: Vec<&Path>, config: &Config) -> UResult<()> {
    let mut files = Vec::<PathData>::new();
    let mut dirs = Vec::<PathData>::new();
    let mut out = BufWriter::new(stdout());
    let initial_locs_len = locs.len();

    for loc in locs {
        let path_data = PathData::new(PathBuf::from(loc), None, None, config, true);

        // Getting metadata here is no big deal as it's just the CWD
        // and we really just want to know if the strings exist as files/dirs
        //
        // Proper GNU handling is don't show if dereferenced symlink DNE
        // but only for the base dir, for a child dir show, and print ?s
        // in long format
        if path_data.md(&mut out).is_none() {
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

    display_items(&files, config, &mut out)?;

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
                writeln!(out, "{}:", path_data.p_buf.display())?;
            } else {
                writeln!(out, "\n{}:", path_data.p_buf.display())?;
            }
        }
        let mut listed_ancestors = HashSet::new();
        listed_ancestors.insert(FileInformation::from_path(
            &path_data.p_buf,
            path_data.must_dereference,
        )?);
        enter_directory(path_data, read_dir, config, &mut out, &mut listed_ancestors)?;
    }

    Ok(())
}

fn sort_entries(entries: &mut [PathData], config: &Config, out: &mut BufWriter<Stdout>) {
    match config.sort {
        Sort::Time => entries.sort_by_key(|k| {
            Reverse(
                k.md(out)
                    .and_then(|md| get_system_time(md, config))
                    .unwrap_or(UNIX_EPOCH),
            )
        }),
        Sort::Size => entries.sort_by_key(|k| Reverse(k.md(out).map(|md| md.len()).unwrap_or(0))),
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
                    get_metadata(p.p_buf.as_path(), true).map_or_else(|_| false, |m| m.is_dir())
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
    let file_name = entry.file_name().into_string().unwrap();
    !config
        .ignore_patterns
        .iter()
        .any(|p| p.matches_with(&file_name, options))
}

fn enter_directory(
    path_data: &PathData,
    read_dir: ReadDir,
    config: &Config,
    out: &mut BufWriter<Stdout>,
    listed_ancestors: &mut HashSet<FileInformation>,
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
        display_total(&entries, config, out)?;
    }

    display_items(&entries, config, out)?;

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
                    if !listed_ancestors
                        .insert(FileInformation::from_path(&e.p_buf, e.must_dereference)?)
                    {
                        out.flush()?;
                        show!(LsError::AlreadyListedError(e.p_buf.clone()));
                    } else {
                        writeln!(out, "\n{}:", e.p_buf.display())?;
                        enter_directory(e, rd, config, out, listed_ancestors)?;
                        listed_ancestors
                            .remove(&FileInformation::from_path(&e.p_buf, e.must_dereference)?);
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_metadata(p_buf: &Path, dereference: bool) -> std::io::Result<Metadata> {
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
    if let Some(md) = entry.md(out) {
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
    format!("{:>width$}", string, width = count)
}

fn pad_right(string: &str, count: usize) -> String {
    format!("{:<width$}", string, width = count)
}

fn display_total(items: &[PathData], config: &Config, out: &mut BufWriter<Stdout>) -> UResult<()> {
    let mut total_size = 0;
    for item in items {
        total_size += item
            .md(out)
            .as_ref()
            .map_or(0, |md| get_block_size(md, config));
    }
    write!(
        out,
        "total {}{}",
        display_size(total_size, config),
        config.eol
    )?;
    Ok(())
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
            let i = if let Some(md) = item.md(out) {
                get_inode(md)
            } else {
                "?".to_owned()
            };
            write!(result, "{} ", pad_left(&i, padding.inode)).unwrap();
        }
    }

    if config.alloc_size {
        let s = if let Some(md) = item.md(out) {
            display_size(get_block_size(md, config), config)
        } else {
            "?".to_owned()
        };
        // extra space is insert to align the sizes, as needed for all formats, except for the comma format.
        if config.format == Format::Commas {
            write!(result, "{} ", s).unwrap();
        } else {
            write!(result, "{} ", pad_left(&s, padding.block_size)).unwrap();
        };
    }
    Ok(result)
}

fn display_items(items: &[PathData], config: &Config, out: &mut BufWriter<Stdout>) -> UResult<()> {
    // `-Z`, `--context`:
    // Display the SELinux security context or '?' if none is found. When used with the `-l`
    // option, print the security context to the left of the size column.

    if config.format == Format::Long {
        let padding_collection = calculate_padding_collection(items, config, out);

        for item in items {
            #[cfg(unix)]
            if config.inode || config.alloc_size {
                let more_info =
                    display_additional_leading_info(item, &padding_collection, config, out)?;
                write!(out, "{}", more_info)?;
            }
            #[cfg(not(unix))]
            if config.alloc_size {
                let more_info =
                    display_additional_leading_info(item, &padding_collection, config, out)?;
                write!(out, "{}", more_info)?;
            }
            display_item_long(item, &padding_collection, config, out)?;
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

        let mut names_vec = Vec::new();

        for i in items {
            let more_info = display_additional_leading_info(i, &padding, config, out)?;
            let cell = display_file_name(i, config, prefix_context, more_info, out);
            names_vec.push(cell);
        }

        let names = names_vec.into_iter();

        match config.format {
            Format::Columns => display_grid(names, config.width, Direction::TopToBottom, out)?,
            Format::Across => display_grid(names, config.width, Direction::LeftToRight, out)?,
            Format::Commas => {
                let mut current_col = 0;
                let mut names = names;
                if let Some(name) = names.next() {
                    write!(out, "{}", name.contents)?;
                    current_col = name.width as u16 + 2;
                }
                for name in names {
                    let name_width = name.width as u16;
                    // If the width is 0 we print one single line
                    if config.width != 0 && current_col + name_width + 1 > config.width {
                        current_col = name_width + 2;
                        write!(out, ",\n{}", name.contents)?;
                    } else {
                        current_col += name_width + 2;
                        write!(out, ", {}", name.contents)?;
                    }
                }
                // Current col is never zero again if names have been printed.
                // So we print a newline.
                if current_col > 0 {
                    write!(out, "{}", config.eol)?;
                }
            }
            _ => {
                for name in names {
                    write!(out, "{}{}", name.contents, config.eol)?;
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
            SizeFormat::Bytes => {
                if cfg!(unix) {
                    if let Some(user_block_size) = config.block_size {
                        raw_blocks / user_block_size
                    } else {
                        raw_blocks / DEFAULT_BLOCK_SIZE
                    }
                } else {
                    raw_blocks
                }
            }
        }
    }
    #[cfg(not(unix))]
    {
        // no way to get block size for windows, fall-back to file size
        md.len()
    }
}

fn display_grid(
    names: impl Iterator<Item = Cell>,
    width: u16,
    direction: Direction,
    out: &mut BufWriter<Stdout>,
) -> UResult<()> {
    if width == 0 {
        // If the width is 0 we print one single line
        let mut printed_something = false;
        for name in names {
            if printed_something {
                write!(out, "  ")?;
            }
            printed_something = true;
            write!(out, "{}", name.contents)?;
        }
        if printed_something {
            writeln!(out)?;
        }
    } else {
        let mut grid = Grid::new(GridOptions {
            filling: Filling::Spaces(2),
            direction,
        });

        for name in names {
            grid.add(name);
        }

        match grid.fit_into_width(width as usize) {
            Some(output) => {
                write!(out, "{}", output)?;
            }
            // Width is too small for the grid, so we fit it in one column
            None => {
                write!(out, "{}", grid.fit_into_columns(1))?;
            }
        }
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
/// * `file_name` ([`display_file_name`])
///
/// This function needs to display information in columns:
/// * permissions and system_time are already guaranteed to be pre-formatted in fixed length.
/// * file_name is the last column and is left-aligned.
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
fn display_item_long(
    item: &PathData,
    padding: &PaddingCollection,
    config: &Config,
    out: &mut BufWriter<Stdout>,
) -> UResult<()> {
    if let Some(md) = item.md(out) {
        write!(
            out,
            "{}{} {}",
            display_permissions(md, true),
            if item.security_context.len() > 1 {
                // GNU `ls` uses a "." character to indicate a file with a security context,
                // but not other alternate access method.
                "."
            } else {
                ""
            },
            pad_left(&display_symlink_count(md), padding.link_count)
        )?;

        if config.long.owner {
            write!(
                out,
                " {}",
                pad_right(&display_uname(md, config), padding.uname)
            )?;
        }

        if config.long.group {
            write!(
                out,
                " {}",
                pad_right(&display_group(md, config), padding.group)
            )?;
        }

        if config.context {
            write!(
                out,
                " {}",
                pad_right(&item.security_context, padding.context)
            )?;
        }

        // Author is only different from owner on GNU/Hurd, so we reuse
        // the owner, since GNU/Hurd is not currently supported by Rust.
        if config.long.author {
            write!(
                out,
                " {}",
                pad_right(&display_uname(md, config), padding.uname)
            )?;
        }

        match display_len_or_rdev(md, config) {
            SizeOrDeviceId::Size(size) => {
                write!(out, " {}", pad_left(&size, padding.size))?;
            }
            SizeOrDeviceId::Device(major, minor) => {
                write!(
                    out,
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
                        )
                    ),
                    pad_left(
                        &minor,
                        #[cfg(not(unix))]
                        0usize,
                        #[cfg(unix)]
                        padding.minor,
                    ),
                )?;
            }
        };

        let dfn = display_file_name(item, config, None, "".to_owned(), out).contents;

        write!(out, " {} {}{}", display_date(md, config), dfn, config.eol)?;
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
            out,
            "{}{} {}",
            format_args!("{}?????????", leading_char),
            if item.security_context.len() > 1 {
                // GNU `ls` uses a "." character to indicate a file with a security context,
                // but not other alternate access method.
                "."
            } else {
                ""
            },
            pad_left("?", padding.link_count)
        )?;

        if config.long.owner {
            write!(out, " {}", pad_right("?", padding.uname))?;
        }

        if config.long.group {
            write!(out, " {}", pad_right("?", padding.group))?;
        }

        if config.context {
            write!(
                out,
                " {}",
                pad_right(&item.security_context, padding.context)
            )?;
        }

        // Author is only different from owner on GNU/Hurd, so we reuse
        // the owner, since GNU/Hurd is not currently supported by Rust.
        if config.long.author {
            write!(out, " {}", pad_right("?", padding.uname))?;
        }

        let dfn = display_file_name(item, config, None, "".to_owned(), out).contents;
        let date_len = 12;

        writeln!(
            out,
            " {} {} {}",
            pad_left("?", padding.size),
            pad_left("?", date_len),
            dfn,
        )?;
    }

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
fn display_group(metadata: &Metadata, config: &Config) -> String {
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
            let recent = time + chrono::Duration::seconds(31_556_952 / 2) > chrono::Local::now();

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

// There are a few peculiarities to how GNU formats the sizes:
// 1. One decimal place is given if and only if the size is smaller than 10
// 2. It rounds sizes up.
// 3. The human-readable format uses powers for 1024, but does not display the "i"
//    that is commonly used to denote Kibi, Mebi, etc.
// 4. Kibi and Kilo are denoted differently ("k" and "K", respectively)
fn format_prefixed(prefixed: &NumberPrefix<f64>) -> String {
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

#[allow(dead_code)]
enum SizeOrDeviceId {
    Size(String),
    Device(String, String),
}

fn display_len_or_rdev(metadata: &Metadata, config: &Config) -> SizeOrDeviceId {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let ft = metadata.file_type();
        if ft.is_char_device() || ft.is_block_device() {
            let dev: u64 = metadata.rdev();
            let major = (dev >> 24) as u8;
            let minor = (dev & 0xff) as u8;
            return SizeOrDeviceId::Device(major.to_string(), minor.to_string());
        }
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let ft = metadata.file_type();
        if ft.is_char_device() || ft.is_block_device() {
            let dev: u64 = metadata.rdev();
            let major = (dev >> 8) as u8;
            let minor = (dev & 0xff) as u8;
            return SizeOrDeviceId::Device(major.to_string(), minor.to_string());
        }
    }
    // Reported file len only adjusted for block_size when block_size is set
    if let Some(user_block_size) = config.block_size {
        // ordinary division of unsigned integers rounds down,
        // this is similar to the Rust API for division that rounds up,
        // currently in nightly only, however once
        // https://github.com/rust-lang/rust/pull/88582 : "div_ceil"
        // is stable we should use that instead
        let len_adjusted = {
            let d = metadata.len() / user_block_size;
            let r = metadata.len() % user_block_size;
            if r == 0 {
                d
            } else {
                d + 1
            }
        };
        SizeOrDeviceId::Size(display_size(len_adjusted, config))
    } else {
        SizeOrDeviceId::Size(display_size(metadata.len(), config))
    }
}

fn display_size(size: u64, config: &Config) -> String {
    // NOTE: The human-readable behavior deviates from the GNU ls.
    // The GNU ls uses binary prefixes by default.
    match config.size_format {
        SizeFormat::Binary => format_prefixed(&NumberPrefix::binary(size as f64)),
        SizeFormat::Decimal => format_prefixed(&NumberPrefix::decimal(size as f64)),
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
            } else if file_type.is_file() && file_is_executable(path.md(out).as_ref().unwrap()) {
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
///
/// Note that non-unicode sequences in symlink targets are dealt with using
/// [`std::path::Path::to_string_lossy`].
#[allow(unused_variables)]
fn display_file_name(
    path: &PathData,
    config: &Config,
    prefix_context: Option<usize>,
    more_info: String,
    out: &mut BufWriter<Stdout>,
) -> Cell {
    // This is our return value. We start by `&path.display_name` and modify it along the way.
    let mut name = escape_name(&path.display_name, &config.quoting_style);

    // We need to keep track of the width ourselves instead of letting term_grid
    // infer it because the color codes mess up term_grid's width calculation.
    let mut width = name.width();

    if let Some(ls_colors) = &config.color {
        let md = path.md(out);
        name = if md.is_some() {
            color_name(name, &path.p_buf, md, ls_colors)
        } else {
            color_name(
                name,
                &path.p_buf,
                path.p_buf.symlink_metadata().ok().as_ref(),
                ls_colors,
            )
        };
    }

    if config.format != Format::Long && !more_info.is_empty() {
        // increment width here b/c name was given colors and name.width() is now the wrong
        // size for display
        width += more_info.width();
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
            width += 1;
        }
    }

    if config.format == Format::Long
        && path.file_type(out).is_some()
        && path.file_type(out).unwrap().is_symlink()
        && !path.must_dereference
    {
        if let Ok(target) = path.p_buf.read_link() {
            name.push_str(" -> ");

            // We might as well color the symlink output after the arrow.
            // This makes extra system calls, but provides important information that
            // people run `ls -l --color` are very interested in.
            if let Some(ls_colors) = &config.color {
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
                if path.md(out).is_none()
                    && get_metadata(target_data.p_buf.as_path(), target_data.must_dereference)
                        .is_err()
                {
                    name.push_str(&path.p_buf.read_link().unwrap().to_string_lossy());
                } else {
                    // Use fn get_metadata instead of md() here and above because ls
                    // should not exit with an err, if we are unable to obtain the target_metadata
                    let target_metadata = match get_metadata(
                        target_data.p_buf.as_path(),
                        target_data.must_dereference,
                    ) {
                        Ok(md) => md,
                        Err(_) => path.md(out).unwrap().to_owned(),
                    };

                    name.push_str(&color_name(
                        escape_name(target.as_os_str(), &config.quoting_style),
                        &target_data.p_buf,
                        Some(&target_metadata),
                        ls_colors,
                    ));
                }
            } else {
                // If no coloring is required, we just use target as is.
                // Apply the right quoting
                name.push_str(&escape_name(target.as_os_str(), &config.quoting_style));
            }
        }
    }

    // Prepend the security context to the `name` and adjust `width` in order
    // to get correct alignment from later calls to`display_grid()`.
    if config.context {
        if let Some(pad_count) = prefix_context {
            let security_context = if !matches!(config.format, Format::Commas) {
                pad_left(&path.security_context, pad_count)
            } else {
                path.security_context.to_owned()
            };
            name = format!("{} {}", security_context, name);
            width += security_context.len() + 1;
        }
    }

    Cell {
        contents: name,
        width,
    }
}

fn color_name(name: String, path: &Path, md: Option<&Metadata>, ls_colors: &LsColors) -> String {
    match ls_colors.style_for_path_with_metadata(path, md) {
        Some(style) => {
            return style.to_ansi_term_style().paint(name).to_string();
        }
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

#[cfg(unix)]
fn display_inode(metadata: &Metadata) -> String {
    get_inode(metadata)
}

// This returns the SELinux security context as UTF8 `String`.
// In the long term this should be changed to `OsStr`, see discussions at #2621/#2656
#[allow(unused_variables)]
fn get_security_context(config: &Config, p_buf: &Path, must_dereference: bool) -> String {
    let substitute_string = "?".to_string();
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
            let inode_len = if let Some(md) = item.md(out) {
                display_inode(md).len()
            } else {
                continue;
            };
            padding_collections.inode = inode_len.max(padding_collections.inode);
        }

        if config.alloc_size {
            if let Some(md) = item.md(out) {
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
                    .max(padding_collections.major + padding_collections.minor + 2usize);
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
            if let Some(md) = item.md(out) {
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
