// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) somegroup nlink tabsize dired subdired dtype colorterm stringly
// spell-checker:ignore nohash strtime clocale

use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    io::{IsTerminal, stdout},
    num::IntErrorKind,
};

use glob::Pattern;
use lscolors::LsColors;
use term_grid::SPACES_IN_TAB;

use uucore::{
    diagnostics::OptionValue,
    display::Quotable,
    error::UResult,
    format::human::SizeFormat,
    fsext::MetadataTimeField,
    line_ending::LineEnding,
    parser::{parse_block_size, parse_glob, parse_size::parse_size_non_zero_u64},
    quoting_style::{QuotingStyle, quoting_style_from_env},
    show_error, show_warning,
    time::format,
    translate,
};

use crate::{
    LsError,
    colors::{LsColorsParseError, validate_ls_colors_env},
    display::{Format, IndicatorStyle, LongFormat},
    options::QUOTING_STYLE,
};

pub mod options {
    pub mod format {
        pub static ONE_LINE: &str = "1";
        pub static LONG: &str = "long";
        pub static COLUMNS: &str = "C";
        pub static ACROSS: &str = "x";
        pub static TAB_SIZE: &str = "tabsize";
        pub static COMMAS: &str = "m";
        pub static LONG_NO_OWNER: &str = "g";
        pub static LONG_NO_GROUP: &str = "o";
        pub static LONG_NUMERIC_UID_GID: &str = "numeric-uid-gid";
    }

    pub mod files {
        pub static ALL: &str = "all";
        pub static ALMOST_ALL: &str = "almost-all";
        pub static UNSORTED_ALL: &str = "f";
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

/// Resolve `(file_size_block_size, block_size)` from environment variables.
///
/// `LS_BLOCK_SIZE` and `BLOCK_SIZE` affect both values.
/// `BLOCKSIZE` only affects `block_size` (allocation display with `-s`).
/// `POSIXLY_CORRECT` sets `block_size` to 512 as a last resort.
/// `-k` (`opt_kb`) forces `block_size` to `DEFAULT_BLOCK_SIZE`.
fn resolve_block_sizes_from_env(opt_kb: bool) -> (u64, u64) {
    match parse_block_size::block_size_from_env(&["LS_BLOCK_SIZE", "BLOCK_SIZE"]) {
        parse_block_size::BlockSizeEnv::Found(size) => {
            if opt_kb {
                (size, DEFAULT_BLOCK_SIZE)
            } else {
                (size, size)
            }
        }
        parse_block_size::BlockSizeEnv::SetButInvalid => (DEFAULT_BLOCK_SIZE, DEFAULT_BLOCK_SIZE),
        parse_block_size::BlockSizeEnv::NotSet => {
            // Neither LS_BLOCK_SIZE nor BLOCK_SIZE was set; check BLOCKSIZE
            // which only affects allocation display, not file size.
            match parse_block_size::block_size_from_env(&["BLOCKSIZE"]) {
                parse_block_size::BlockSizeEnv::Found(size) => {
                    if opt_kb {
                        (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
                    } else {
                        (DEFAULT_FILE_SIZE_BLOCK_SIZE, size)
                    }
                }
                parse_block_size::BlockSizeEnv::SetButInvalid => {
                    // BLOCKSIZE was set but invalid: stop lookup, use defaults.
                    (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
                }
                parse_block_size::BlockSizeEnv::NotSet => {
                    if std::env::var_os("POSIXLY_CORRECT").is_some() && !opt_kb {
                        (DEFAULT_FILE_SIZE_BLOCK_SIZE, POSIXLY_CORRECT_BLOCK_SIZE)
                    } else {
                        (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
                    }
                }
            }
        }
    }
}

pub(crate) enum Dereference {
    None,
    DirArgs,
    Args,
    All,
}

#[derive(PartialEq, Eq)]
pub(crate) enum Sort {
    None,
    Name,
    Size,
    Time,
    Version,
    Extension,
    Width,
}

#[derive(PartialEq, Eq)]
pub(crate) enum Files {
    All,
    AlmostAll,
    Normal,
}

/// Which listing program is constructing this [`Config`].
///
/// `ls` defaults depend on whether stdout is a terminal. `dir` and `vdir`
/// default to a fixed format and escape quoting.
#[derive(Clone, Copy)]
enum ProgramMode {
    Ls,
    Dir,
    Vdir,
}

pub struct Config {
    // Dir and vdir needs access to this field
    pub format: Format,
    pub(crate) files: Files,
    pub(crate) sort: Sort,
    pub(crate) recursive: bool,
    pub(crate) reverse: bool,
    pub(crate) dereference: Dereference,
    pub(crate) ignore_patterns: Vec<Pattern>,
    pub(crate) size_format: SizeFormat,
    pub(crate) directory: bool,
    pub(crate) time: MetadataTimeField,
    #[cfg(unix)]
    pub(crate) inode: bool,
    pub(crate) color: Option<LsColors>,
    pub(crate) long: LongFormat,
    pub(crate) alloc_size: bool,
    pub(crate) file_size_block_size: u64,
    #[allow(dead_code)]
    pub(crate) block_size: u64, // is never read on Windows
    pub(crate) width: u16,
    // Dir and vdir needs access to this field
    pub quoting_style: QuotingStyle,
    pub(crate) show_control_chars: bool,
    pub(crate) indicator_style: Option<IndicatorStyle>,
    pub(crate) time_format_recent: String, // Time format for recent dates
    pub(crate) time_format_older: Option<String>, // Time format for older dates (optional, if not present, time_format_recent is used)
    pub(crate) context: bool,
    #[cfg(all(feature = "selinux", any(target_os = "linux", target_os = "android")))]
    pub(crate) selinux_supported: bool,
    #[cfg(all(feature = "smack", target_os = "linux"))]
    pub(crate) smack_supported: bool,
    pub(crate) group_directories_first: bool,
    pub(crate) line_ending: LineEnding,
    pub(crate) dired: bool,
    pub(crate) hyperlink: bool,
    pub(crate) tab_size: usize,
}

/// Extracts the format to display the information based on the options provided.
///
/// When no format option is given, `mode` selects the program default: `ls`
/// depends on whether stdout is a terminal; `dir` is columns; `vdir` is long.
/// A `None` option id means that default, so `-1` and `--zero` can still
/// override it.
///
/// # Returns
///
/// A tuple containing the Format variant and an Option containing a &'static str
/// which corresponds to the option used to define the format.
fn extract_format(options: &clap::ArgMatches, mode: ProgramMode) -> (Format, Option<&'static str>) {
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
    } else {
        match mode {
            ProgramMode::Dir => (Format::Columns, None),
            ProgramMode::Vdir => (Format::Long, None),
            ProgramMode::Ls => {
                if stdout().is_terminal() {
                    (Format::Columns, None)
                } else {
                    (Format::OneLine, None)
                }
            }
        }
    }
}

/// Extracts the type of files to display
///
/// # Returns
///
/// A Files variant representing the type of files to display.
fn extract_files(options: &clap::ArgMatches) -> Files {
    let get_last_index = |flag: &str| -> usize {
        if options.value_source(flag) == Some(clap::parser::ValueSource::CommandLine) {
            options.index_of(flag).unwrap_or(0)
        } else {
            0
        }
    };

    let all_index = get_last_index(options::files::ALL);
    let almost_all_index = get_last_index(options::files::ALMOST_ALL);
    let unsorted_all_index = get_last_index(options::files::UNSORTED_ALL);

    let max_index = all_index.max(almost_all_index).max(unsorted_all_index);

    if max_index == 0 {
        Files::Normal
    } else if max_index == almost_all_index {
        Files::AlmostAll
    } else {
        // Either -a or -f wins, both show all files
        Files::All
    }
}

/// Extracts the sorting method to use based on the options provided.
///
/// # Returns
///
/// A Sort variant representing the sorting method to use.
fn extract_sort(options: &clap::ArgMatches, format: &Format) -> Sort {
    let get_last_index = |flag: &str| -> usize {
        if options.value_source(flag) == Some(clap::parser::ValueSource::CommandLine) {
            options.index_of(flag).unwrap_or(0)
        } else {
            0
        }
    };

    let sort_index = options
        .indices_of(options::SORT)
        .and_then(|mut it| it.next_back())
        .unwrap_or(0);
    let time_index = get_last_index(options::sort::TIME);
    let size_index = get_last_index(options::sort::SIZE);
    let none_index = get_last_index(options::sort::NONE);
    let version_index = get_last_index(options::sort::VERSION);
    let extension_index = get_last_index(options::sort::EXTENSION);
    let unsorted_all_index = get_last_index(options::files::UNSORTED_ALL);

    let max_sort_index = sort_index
        .max(time_index)
        .max(size_index)
        .max(none_index)
        .max(version_index)
        .max(extension_index)
        .max(unsorted_all_index);

    match max_sort_index {
        0 => {
            // No sort flags specified, use default behavior
            if *format != Format::Long
                && (options.get_flag(options::time::ACCESS)
                    || options.get_flag(options::time::CHANGE)
                    || options.get_one::<String>(options::TIME).is_some())
            {
                Sort::Time
            } else {
                Sort::Name
            }
        }
        idx if idx == unsorted_all_index || idx == none_index => Sort::None,
        idx if idx == sort_index => {
            if let Some(field) = options.get_one::<String>(options::SORT) {
                match field.as_str() {
                    "none" => Sort::None,
                    "name" => Sort::Name,
                    "time" => Sort::Time,
                    "size" => Sort::Size,
                    "version" => Sort::Version,
                    "extension" => Sort::Extension,
                    "width" => Sort::Width,
                    _ => unreachable!("Invalid field for --sort"),
                }
            } else {
                Sort::Name
            }
        }
        idx if idx == time_index => Sort::Time,
        idx if idx == size_index => Sort::Size,
        idx if idx == version_index => Sort::Version,
        idx if idx == extension_index => Sort::Extension,
        _ => Sort::Name,
    }
}

/// Extracts the time to use based on the options provided.
///
/// # Returns
///
/// A `MetadataTimeField` variant representing the time to use.
fn extract_time(options: &clap::ArgMatches) -> MetadataTimeField {
    if let Some(field) = options.get_one::<String>(options::TIME) {
        field.as_str().into()
    } else if options.get_flag(options::time::ACCESS) {
        MetadataTimeField::Access
    } else if options.get_flag(options::time::CHANGE) {
        MetadataTimeField::Change
    } else {
        MetadataTimeField::Modification
    }
}

/// Some env variables can be passed
/// For now, we are only verifying if empty or not and known for `TERM`
fn is_color_compatible_term() -> bool {
    let term = std::env::var_os("TERM");
    let colorterm = std::env::var_os("COLORTERM");

    // Search function in the TERM struct to manage the wildcards
    let term_matches = |term: &OsStr| -> bool {
        uucore::colors::TERMS.iter().any(|&pattern| {
            term == pattern
                || (pattern.ends_with('*')
                    && term
                        .as_encoded_bytes()
                        .starts_with(&pattern.as_bytes()[..pattern.len() - 1]))
        })
    };

    match (term, colorterm) {
        (Some(t), Some(c)) if t.is_empty() && c.is_empty() => false,
        (Some(t), _) if !t.is_empty() => term_matches(&t),
        _ => true,
    }
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

    let get_last_index = |flag: &str| -> usize {
        if options.value_source(flag) == Some(clap::parser::ValueSource::CommandLine) {
            options.index_of(flag).unwrap_or(0)
        } else {
            0
        }
    };

    let color_index = options
        .indices_of(options::COLOR)
        .and_then(|mut it| it.next_back())
        .unwrap_or(0);
    let unsorted_all_index = get_last_index(options::files::UNSORTED_ALL);

    let color_enabled = match options.get_one::<String>(options::COLOR) {
        None => options.contains_id(options::COLOR),
        Some(val) => match val.as_str() {
            "" | "always" | "yes" | "force" => true,
            "auto" | "tty" | "if-tty" => stdout().is_terminal(),
            /* "never" | "no" | "none" | */ _ => false,
        },
    };

    // If --color was explicitly specified, always honor it regardless of -f
    // Otherwise, if -f is present without explicit color, disable color
    if color_index > 0 {
        // Color was explicitly specified
        color_enabled
    } else if unsorted_all_index > 0 {
        // -f present without explicit color, disable implicit color
        false
    } else {
        color_enabled
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
        "auto" | "tty" | "if-tty" => stdout().is_terminal(),
        "never" | "no" | "none" => false,
        _ => unreachable!("should be handled by clap"),
    }
}

/// Extracts the quoting style to use based on the options provided.
/// If no options are given, it looks if a default quoting style is provided
/// through the [`QUOTING_STYLE`] environment variable.
///
/// # Arguments
///
/// * `options` - A reference to a [`clap::ArgMatches`] object containing command line arguments.
/// * `show_control` - A boolean value representing whether or not to show control characters.
///
/// # Returns
///
/// A [`QuotingStyle`] variant representing the quoting style to use.
fn extract_quoting_style(
    options: &clap::ArgMatches,
    show_control: bool,
    mode: ProgramMode,
) -> QuotingStyle {
    let opt_quoting_style = options.get_one::<String>(QUOTING_STYLE);

    if let Some(style) = opt_quoting_style {
        match QuotingStyle::parse(style) {
            Some(qs) => qs.show_control(show_control),
            None => unreachable!("Should have been caught by Clap"),
        }
    } else if options.get_flag(options::quoting::LITERAL) {
        QuotingStyle::Literal { show_control }
    } else if options.get_flag(options::quoting::ESCAPE) {
        QuotingStyle::C_NO_QUOTES
    } else if options.get_flag(options::quoting::C) {
        QuotingStyle::C_DOUBLE
    } else {
        // If set, the QUOTING_STYLE environment variable specifies a default style.
        if let Some(qs) = quoting_style_from_env() {
            return qs.show_control(show_control);
        }

        match mode {
            ProgramMode::Dir | ProgramMode::Vdir => QuotingStyle::C_NO_QUOTES,
            ProgramMode::Ls if stdout().is_terminal() => {
                QuotingStyle::SHELL_ESCAPE.show_control(show_control)
            }
            ProgramMode::Ls => QuotingStyle::Literal { show_control },
        }
    }
}

/// Extracts the indicator style to use based on the options provided.
///
/// # Returns
///
/// An [`IndicatorStyle`] variant representing the indicator style to use.
fn extract_indicator_style(options: &clap::ArgMatches) -> Option<IndicatorStyle> {
    if let Some(field) = options.get_one::<String>(options::INDICATOR_STYLE) {
        match field.as_str() {
            "file-type" => Some(IndicatorStyle::FileType),
            "classify" => Some(IndicatorStyle::Classify),
            "slash" => Some(IndicatorStyle::Slash),
            "none" | &_ => None,
        }
    } else if let Some(field) = options.get_one::<String>(options::indicator_style::CLASSIFY) {
        match field.as_str() {
            "always" | "yes" | "force" => Some(IndicatorStyle::Classify),
            "auto" | "tty" | "if-tty" => stdout().is_terminal().then_some(IndicatorStyle::Classify),
            "never" | "no" | "none" | &_ => None,
        }
    } else if options.get_flag(options::indicator_style::SLASH) {
        Some(IndicatorStyle::Slash)
    } else if options.get_flag(options::indicator_style::FILE_TYPE) {
        Some(IndicatorStyle::FileType)
    } else {
        None
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

    let parse_width_from_env = |columns: OsString| {
        if let Some(columns) = columns.to_str().and_then(|s| s.parse().ok()) {
            columns
        } else {
            show_error!(
                "{}",
                translate!("ls-invalid-columns-width", "width" => columns.quote())
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

/// Parses the tab size value from the command line
fn parse_tab_size(size_str: &str) -> Result<usize, LsError> {
    size_str
        .parse::<usize>()
        .ok()
        .or_else(|| {
            size_str
                .strip_prefix("0x")
                .or_else(|| size_str.strip_prefix("0X"))
                .and_then(|hex| usize::from_str_radix(hex, 16).ok())
        })
        .ok_or_else(|| LsError::InvalidTabSize(size_str.to_string()))
}

impl Config {
    pub fn from(options: &clap::ArgMatches, diag_args: Option<&[OsString]>) -> UResult<Self> {
        Self::from_with_program_mode(options, diag_args, ProgramMode::Ls)
    }

    /// Construct a configuration with dir's default column format and escape quoting.
    pub fn from_dir(options: &clap::ArgMatches, diag_args: Option<&[OsString]>) -> UResult<Self> {
        Self::from_with_program_mode(options, diag_args, ProgramMode::Dir)
    }

    /// Construct a configuration with vdir's default long format and escape quoting.
    pub fn from_vdir(options: &clap::ArgMatches, diag_args: Option<&[OsString]>) -> UResult<Self> {
        Self::from_with_program_mode(options, diag_args, ProgramMode::Vdir)
    }

    #[allow(clippy::cognitive_complexity)]
    fn from_with_program_mode(
        options: &clap::ArgMatches,
        diag_args: Option<&[OsString]>,
        mode: ProgramMode,
    ) -> UResult<Self> {
        let context = options.get_flag(options::CONTEXT);
        let (mut format, opt) = extract_format(options, mode);
        // -1 and --zero override a default long format, but not an explicit one.
        let mut explicit_long = format == Format::Long && opt.is_some();
        let files = extract_files(options);

        // The -o, -n and -g options are tricky. They cannot override with each
        // other because it's possible to combine them. For example, the option
        // -og should hide both owner and group. Furthermore, they are not
        // reset if -l or --format=long is used. So these should just show the
        // group: -gl or "-g --format=long". Finally, they are also not reset
        // when switching to a different format option in-between like this:
        // -ogCl or "-og --format=vertical --format=long".
        //
        // -1 has a similar issue: it does nothing if long format was explicitly
        // requested. This makes it distinct from the --format=singe-column option,
        // which always applies.
        //
        // --dired (-D) implies long format the same way -g, -o and -n do: it
        // wins over earlier format options, loses to later ones, and a -1
        // after it has no effect. Whether dired output is actually emitted is
        // decided below, once the final format is known.
        //
        // The idea here is to not let these options override with the other
        // options, but manually whether they have an index that's greater than
        // the other format options. If so, we set the appropriate format.
        if !explicit_long {
            let idx = opt
                .and_then(|opt| options.indices_of(opt).map(|x| x.max().unwrap()))
                .unwrap_or(0);
            if [
                options::format::LONG_NO_OWNER,
                options::format::LONG_NO_GROUP,
                options::format::LONG_NUMERIC_UID_GID,
                options::FULL_TIME,
                options::DIRED,
            ]
            .iter()
            .filter_map(|opt| {
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
                explicit_long = true;
            } else if let Some(mut indices) = options.indices_of(options::format::ONE_LINE)
                && options.value_source(options::format::ONE_LINE)
                    == Some(clap::parser::ValueSource::CommandLine)
                && indices.any(|i| i > idx)
            {
                format = Format::OneLine;
            }
        }

        let time = extract_time(options);
        let mut needs_color = extract_color(options);
        let hyperlink = extract_hyperlink(options);

        let opt_block_size = options.get_one::<String>(options::size::BLOCK_SIZE);
        let opt_si = opt_block_size.is_some_and(|x| x == options::size::SI)
            || options.get_flag(options::size::SI);
        let opt_hr = opt_block_size.is_some_and(|x| x == options::size::HUMAN_READABLE)
            || options.get_flag(options::size::HUMAN_READABLE);
        let opt_kb = options.get_flag(options::size::KIBIBYTES);

        let size_format = if opt_si {
            SizeFormat::Decimal
        } else if opt_hr {
            SizeFormat::Binary
        } else {
            SizeFormat::Bytes
        };

        let (file_size_block_size, block_size) = if let Some(opt_block_size) = opt_block_size {
            // --block-size command-line argument: parse it, error on invalid
            // If --block-size=si or --block-size=human-readable, skip numeric parsing
            if opt_si {
                (DEFAULT_FILE_SIZE_BLOCK_SIZE, 1000)
            } else if opt_hr {
                (DEFAULT_FILE_SIZE_BLOCK_SIZE, DEFAULT_BLOCK_SIZE)
            } else {
                let size = parse_size_non_zero_u64(opt_block_size).map_err(|error| {
                    let ls_error = LsError::BlockSizeParseError(opt_block_size.clone());
                    let message = ls_error.to_string();
                    error.size_value_error(
                        diag_args,
                        &OptionValue::with_names(
                            opt_block_size,
                            None,
                            Some(options::size::BLOCK_SIZE),
                        ),
                        0,
                        &message,
                        ls_error,
                    )
                })?;
                // --block-size overrides -k
                (size, size)
            }
        } else if !opt_si && !opt_hr {
            resolve_block_sizes_from_env(opt_kb)
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
            let numeric_uid_gid = options.get_flag(options::format::LONG_NUMERIC_UID_GID);
            LongFormat {
                author,
                group,
                owner,
                numeric_uid_gid,
            }
        };
        let width = parse_width(options.get_one::<String>(options::WIDTH))?;

        let mut show_control = if options.get_flag(options::HIDE_CONTROL_CHARS) {
            false
        } else {
            options.get_flag(options::SHOW_CONTROL_CHARS)
                || !matches!(mode, ProgramMode::Ls)
                || !stdout().is_terminal()
        };

        let mut quoting_style = extract_quoting_style(options, show_control, mode);
        let indicator_style = extract_indicator_style(options);

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
            if let Ok(p) = parse_glob::from_str(pattern) {
                ignore_patterns.push(p);
            } else {
                show_warning!(
                    "{}",
                    translate!("ls-invalid-ignore-pattern", "pattern" => pattern.quote())
                );
            }
        }

        if files == Files::Normal {
            for pattern in options
                .get_many::<String>(options::HIDE)
                .into_iter()
                .flatten()
            {
                if let Ok(p) = parse_glob::from_str(pattern) {
                    ignore_patterns.push(p);
                } else {
                    show_warning!(
                        "{}",
                        translate!("ls-invalid-hide-pattern", "pattern" => pattern.quote())
                    );
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
            QUOTING_STYLE,
            options::quoting::C,
            options::quoting::ESCAPE,
            options::quoting::LITERAL,
        ];
        let get_last = |flag: &str| -> usize {
            (options.value_source(flag) == Some(clap::parser::ValueSource::CommandLine))
                .then(|| options.index_of(flag))
                .flatten()
                .unwrap_or(0)
        };
        let zero_idx = get_last(options::ZERO);
        let last_of =
            |flag_list: &[&str]| flag_list.iter().copied().map(get_last).max().unwrap_or(0);

        if zero_idx > last_of(&zero_formats_opts) && !explicit_long {
            format = Format::OneLine;
        }

        if zero_idx > last_of(&zero_colors_opts) {
            needs_color = false;
        }

        if zero_idx > last_of(&zero_show_control_opts) {
            show_control = true;
        }

        if zero_idx > last_of(&zero_quoting_style_opts) {
            quoting_style = QuotingStyle::Literal { show_control };
        }

        if needs_color && let Err(err) = validate_ls_colors_env() {
            if let LsColorsParseError::UnrecognizedPrefix(prefix) = &err {
                show_error!(
                    "{}",
                    translate!(
                        "ls-error-unrecognized-ls-colors-prefix",
                        "prefix" => prefix.quote()
                    )
                );
            }
            show_error!("{}", translate!("ls-error-unparsable-ls-colors"));
            needs_color = false;
        }

        let color = if needs_color {
            Some(LsColors::from_env().unwrap_or_default())
        } else {
            None
        };

        // Hyperlinks enabled after the last --dired cancel the dired output,
        // and a --dired after --hyperlink disables them again. Dired output
        // also requires the final format to be long: a later -C, -x, -m or
        // --format= cancels it.
        let dired_idx = get_last(options::DIRED);
        let hyperlink = hyperlink && get_last(options::HYPERLINK) > dired_idx;
        let dired = dired_idx > 0 && format == Format::Long && !hyperlink;
        if dired && options.get_flag(options::ZERO) {
            return Err(Box::new(LsError::DiredAndZeroAreIncompatible));
        }

        let sort = extract_sort(options, &format);

        // Only parse the time style after the final output format is known.
        let (time_format_recent, time_format_older) = if format == Format::Long {
            parse_time_style(options)?
        } else {
            Default::default()
        };

        let dereference = if options.get_flag(options::dereference::ALL) {
            Dereference::All
        } else if options.get_flag(options::dereference::ARGS) {
            Dereference::Args
        } else if options.get_flag(options::dereference::DIR_ARGS) {
            Dereference::DirArgs
        } else if options.get_flag(options::DIRECTORY)
            || indicator_style == Some(IndicatorStyle::Classify)
            || format == Format::Long
        {
            Dereference::None
        } else {
            Dereference::DirArgs
        };

        let tab_size = if needs_color {
            Some(0)
        } else if let Some(size_str) = options.get_one::<String>(options::format::TAB_SIZE) {
            Some(parse_tab_size(size_str)?)
        } else {
            None
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
            show_control_chars: options.get_flag(options::SHOW_CONTROL_CHARS),
            indicator_style,
            time_format_recent,
            time_format_older,
            context,
            #[cfg(all(feature = "selinux", any(target_os = "linux", target_os = "android")))]
            selinux_supported: uucore::selinux::is_selinux_enabled(),
            #[cfg(all(feature = "smack", target_os = "linux"))]
            smack_supported: uucore::smack::is_smack_enabled(),
            group_directories_first: options.get_flag(options::GROUP_DIRECTORIES_FIRST),
            line_ending: LineEnding::from_zero_flag(options.get_flag(options::ZERO)),
            dired,
            hyperlink,
            tab_size: tab_size.unwrap_or(SPACES_IN_TAB),
        })
    }
}

fn parse_time_style(options: &clap::ArgMatches) -> Result<(String, Option<String>), LsError> {
    // TODO: Using correct locale string is not implemented.
    const LOCALE_FORMAT: (&str, Option<&str>) = ("%b %e %H:%M", Some("%b %e  %Y"));

    // Convert time_styles references to owned String/option.
    #[expect(clippy::unnecessary_wraps, reason = "internal result helper")]
    fn ok((recent, older): (&str, Option<&str>)) -> Result<(String, Option<String>), LsError> {
        Ok((recent.to_string(), older.map(String::from)))
    }

    if let Some(field) = options
        .get_one::<String>(options::TIME_STYLE)
        .map(Cow::from)
        .or_else(|| std::env::var("TIME_STYLE").ok().map(Cow::from))
    {
        //If both FULL_TIME and TIME_STYLE are present
        //The one added last is dominant
        if options.get_flag(options::FULL_TIME)
            && options.indices_of(options::FULL_TIME).unwrap().next_back()
                > options.indices_of(options::TIME_STYLE).unwrap().next_back()
        {
            ok((format::FULL_ISO, None))
        } else {
            let field = if let Some(field) = field.strip_prefix("posix-") {
                // See GNU documentation, set format to "locale" if LC_TIME="POSIX",
                // else just strip the prefix and continue (even "posix+FORMAT" is
                // supported).
                // TODO: This needs to be moved to uucore and handled by icu?
                if std::env::var_os("LC_TIME").as_deref() == Some(OsStr::new("POSIX"))
                    || std::env::var_os("LC_ALL").as_deref() == Some(OsStr::new("POSIX"))
                {
                    return ok(LOCALE_FORMAT);
                }
                field
            } else {
                &field
            };

            // Resolve only unique prefixes, leaving ambiguous or invalid values
            // unchanged so they produce the existing time-style error.
            let mut styles = ["full-iso", "long-iso", "iso", "locale"]
                .into_iter()
                .filter(|style| style.starts_with(field));
            let field = match (styles.next(), styles.next()) {
                (Some(style), None) => style,
                _ => field,
            };

            match field {
                "full-iso" => ok((format::FULL_ISO, None)),
                "long-iso" => ok((format::LONG_ISO, None)),
                // ISO older format needs extra padding.
                "iso" => Ok((
                    "%m-%d %H:%M".to_string(),
                    Some(format::ISO.to_string() + " "),
                )),
                "locale" => ok(LOCALE_FORMAT),
                // `field` can be empty here (e.g. --time-style=posix-), so test
                // the prefix instead of unwrapping the first char.
                _ if field.starts_with('+') => {
                    // recent/older formats are (optionally) separated by a newline
                    let mut it = field[1..].split('\n');
                    let recent = it.next().unwrap_or_default();
                    let older = it.next();
                    match it.next() {
                        None => ok((recent, older)),
                        Some(_) => Err(LsError::TimeStyleParseError(String::from(field))),
                    }
                }
                _ => Err(LsError::TimeStyleParseError(String::from(field))),
            }
        }
    } else if options.get_flag(options::FULL_TIME) {
        ok((format::FULL_ISO, None))
    } else {
        ok(LOCALE_FORMAT)
    }
}
