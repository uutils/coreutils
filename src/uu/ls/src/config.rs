// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) somegroup nlink tabsize dired subdired dtype colorterm stringly
// spell-checker:ignore nohash strtime clocale uerror

use std::{
    ffi::{OsStr, OsString},
    io::{IsTerminal, stdout},
    num::IntErrorKind,
};

use glob::Pattern;
use lscolors::LsColors;
use term_grid::SPACES_IN_TAB;

use uucore::{
    display::Quotable,
    error::UResult,
    format::human::SizeFormat,
    fsext::MetadataTimeField,
    line_ending::LineEnding,
    parser::{
        gnu::{Occurrence, ParseError, Position},
        parse_block_size, parse_glob,
        parse_size::parse_size_non_zero_u64,
    },
    quoting_style::{QuotingStyle, quoting_style_from_env},
    show_error, show_warning,
    time::format,
    translate,
};

use crate::{
    LsError,
    colors::{LsColorsParseError, validate_ls_colors_env},
    display::{Format, IndicatorStyle, LongFormat},
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
pub enum ProgramMode {
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

/// Parses the width value from either the command line arguments or the environment variables.
fn parse_width(width_match: Option<&str>) -> Result<u16, LsError> {
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
        .ok_or_else(|| LsError::InvalidTabSize(size_str.into()))
}

/// Formatting options retain positions because `-1` only overrides a default
/// or a non-long format, while `-g`, `-o`, and `-n` retain their other settings.
/// Clap callers can also supply resolved defaults with synthetic indices.
#[derive(Default)]
struct DisplayOptions {
    selected: Option<(Format, usize)>,
    long_at: usize,
    one_line_at: usize,
    zero_at: usize,
    last_command_line_format: usize,
}

impl DisplayOptions {
    fn select(&mut self, format: Format, position: usize) {
        self.selected = Some((format, position));
        self.last_command_line_format = position;
    }

    fn imply_long(&mut self, position: usize, counts_for_zero: bool) {
        self.long_at = position;
        if counts_for_zero {
            self.last_command_line_format = position;
        }
    }

    fn one_line(&mut self, position: usize) {
        self.one_line_at = position;
        self.last_command_line_format = position;
    }

    fn resolve(self, mode: ProgramMode) -> Format {
        let explicit = self.selected.is_some();
        let (mut format, position) = self.selected.unwrap_or_else(|| {
            (
                match mode {
                    ProgramMode::Dir => Format::Columns,
                    ProgramMode::Vdir => Format::Long,
                    ProgramMode::Ls if stdout().is_terminal() => Format::Columns,
                    ProgramMode::Ls => Format::OneLine,
                },
                0,
            )
        });
        let mut explicit_long = explicit && format == Format::Long;
        if !explicit_long {
            if self.long_at > 0 && self.long_at >= position {
                format = Format::Long;
                explicit_long = true;
            } else if self.one_line_at > position {
                format = Format::OneLine;
            }
        }
        if !explicit_long && self.zero_at > self.last_command_line_format {
            format = Format::OneLine;
        }
        format
    }
}

fn resolve_time_style(
    style: Option<&(OsString, usize)>,
    full_time_at: Option<usize>,
) -> Option<&OsStr> {
    match (style, full_time_at) {
        (Some((_, position)), Some(full)) if full > *position => Some(OsStr::new("full-iso")),
        (Some((value, _)), _) => Some(value),
        (None, Some(_)) => Some(OsStr::new("full-iso")),
        (None, None) => None,
    }
}

fn last_clap_index(matches: &clap::ArgMatches, id: &str) -> usize {
    matches
        .indices_of(id)
        .and_then(|mut indices| indices.next_back())
        .unwrap_or(0)
}

fn command_line_index(matches: &clap::ArgMatches, id: &str) -> usize {
    if matches.value_source(id) == Some(clap::parser::ValueSource::CommandLine) {
        last_clap_index(matches, id)
    } else {
        0
    }
}

fn clap_value(matches: &clap::ArgMatches, id: &'static str) -> Option<Occurrence> {
    matches.get_one::<String>(id).map(|value| Occurrence {
        id,
        value: Some(value.into()),
        spelling: format!("--{id}").into(),
        value_position: None,
        position: Position {
            argument: 0,
            short: None,
            sequence: last_clap_index(matches, id),
        },
    })
}

fn clap_choice(
    matches: &clap::ArgMatches,
    id: &'static str,
    diag_args: Option<&[OsString]>,
) -> UResult<Option<&'static str>> {
    clap_value(matches, id)
        .map(|value| crate::args::resolve_value(&value, diag_args))
        .transpose()
}

fn sort_value(value: &str) -> Sort {
    match value {
        "none" => Sort::None,
        "time" => Sort::Time,
        "size" => Sort::Size,
        "version" => Sort::Version,
        "extension" => Sort::Extension,
        "width" => Sort::Width,
        _ => Sort::Name,
    }
}

fn indicator_value(value: &str) -> Option<IndicatorStyle> {
    match value {
        "file-type" => Some(IndicatorStyle::FileType),
        "classify" => Some(IndicatorStyle::Classify),
        "slash" => Some(IndicatorStyle::Slash),
        _ => None,
    }
}
/// State updated in argument order, before any filesystem access. These are
/// utility settings rather than an untyped bag of parser matches: composite
/// options update the same fields as their individual counterparts.
pub(crate) struct Settings {
    mode: ProgramMode,
    format: DisplayOptions,
    files: Files,
    sort: Option<Sort>,
    time: MetadataTimeField,
    time_explicit: bool,
    color: When,
    color_explicit: bool,
    hyperlink: When,
    long: LongFormat,
    size: SizeSetting,
    kibibytes: bool,
    width: Option<u16>,
    tab_size: Option<usize>,
    captured_tab_size: Option<OsString>,
    show_control: Option<bool>,
    captured_show_control_chars: Option<bool>,
    quoting_style: Option<QuotingStyle>,
    indicator_style: Option<IndicatorStyle>,
    dereference: Option<Dereference>,
    ignore: Vec<OsString>,
    hide: Vec<OsString>,
    ignore_backups: bool,
    recursive: bool,
    reverse: bool,
    directory: bool,
    inode: bool,
    alloc_size: bool,
    context: bool,
    group_directories_first: bool,
    zero: bool,
    dired: bool,
    time_style: Option<(OsString, usize)>,
    full_time_at: Option<usize>,
}

#[derive(Clone, Copy)]
enum When {
    Always,
    Auto,
    Never,
}

impl When {
    fn enabled(self) -> bool {
        match self {
            Self::Always => true,
            Self::Auto => stdout().is_terminal(),
            Self::Never => false,
        }
    }
}

enum SizeSetting {
    Environment,
    Binary,
    Decimal,
    Blocks(u64),
}

impl Settings {
    pub(crate) fn new(mode: ProgramMode) -> Self {
        Self {
            mode,
            format: DisplayOptions::default(),
            files: Files::Normal,
            sort: None,
            time: MetadataTimeField::Modification,
            time_explicit: false,
            color: When::Never,
            color_explicit: false,
            hyperlink: When::Never,
            long: LongFormat {
                author: false,
                group: true,
                owner: true,
                numeric_uid_gid: false,
            },
            size: SizeSetting::Environment,
            kibibytes: false,
            width: None,
            tab_size: None,
            captured_tab_size: std::env::var_os("TABSIZE"),
            show_control: None,
            captured_show_control_chars: None,
            quoting_style: None,
            indicator_style: None,
            dereference: None,
            ignore: Vec::new(),
            hide: Vec::new(),
            ignore_backups: false,
            recursive: false,
            reverse: false,
            directory: false,
            inode: false,
            alloc_size: false,
            context: false,
            group_directories_first: false,
            zero: false,
            dired: false,
            time_style: std::env::var_os("TIME_STYLE").map(|value| (value, 0)),
            full_time_at: None,
        }
    }

    /// Validate values when encountered, so a later --help does not swallow
    /// an earlier invalid option. TIME_STYLE is deliberately deferred: it is
    /// only interpreted when the final output format is long.
    pub(crate) fn apply(
        &mut self,
        option: Occurrence,
        diag_args: Option<&[OsString]>,
    ) -> UResult<()> {
        let value = option.value.as_deref().unwrap_or_else(|| OsStr::new(""));
        let choice = crate::args::resolve_value(&option, diag_args)?;
        // The existing glob matcher accepts Unicode patterns. Keep rejecting
        // unsupported raw patterns rather than accepting and ignoring them.
        if [options::HIDE, options::IGNORE].contains(&option.id) && value.to_str().is_none() {
            return Err(ParseError::invalid_value(&option, &[]).to_uerror_with_args(2, diag_args));
        }
        match option.id {
            id if id == options::FORMAT => self.format.select(
                match choice {
                    "long" | "verbose" => Format::Long,
                    "single-column" => Format::OneLine,
                    "columns" | "vertical" => Format::Columns,
                    "across" | "horizontal" => Format::Across,
                    "commas" => Format::Commas,
                    _ => return Err(ParseError::invalid_value(&option, &[]).to_uerror(1)),
                },
                option.position.sequence,
            ),
            id if id == options::format::LONG => {
                self.format.select(Format::Long, option.position.sequence);
            }
            id if id == options::format::COLUMNS => self
                .format
                .select(Format::Columns, option.position.sequence),
            id if id == options::format::ACROSS => {
                self.format.select(Format::Across, option.position.sequence);
            }
            id if id == options::format::COMMAS => {
                self.format.select(Format::Commas, option.position.sequence);
            }
            id if id == options::format::ONE_LINE => self.format.one_line(option.position.sequence),
            id if id == options::format::LONG_NO_OWNER => {
                self.long.owner = false;
                self.format.imply_long(option.position.sequence, true);
            }
            id if id == options::format::LONG_NO_GROUP => {
                self.long.group = false;
                self.format.imply_long(option.position.sequence, true);
            }
            id if id == options::format::LONG_NUMERIC_UID_GID => {
                self.long.numeric_uid_gid = true;
                self.format.imply_long(option.position.sequence, true);
            }
            id if id == options::FULL_TIME => {
                self.full_time_at = Some(option.position.sequence);
                self.format.imply_long(option.position.sequence, false);
            }
            id if id == options::TIME_STYLE => {
                self.time_style = Some((value.into(), option.position.sequence));
            }
            id if id == options::DIRED => {
                self.dired = true;
                self.hyperlink = When::Never;
                self.format.imply_long(option.position.sequence, false);
            }
            id if id == options::ZERO => {
                self.zero = true;
                self.format.zero_at = option.position.sequence;
                self.color = When::Never;
                self.show_control = Some(true);
                self.quoting_style = Some(QuotingStyle::Literal { show_control: true });
            }
            id if id == options::files::ALL => self.files = Files::All,
            id if id == options::files::ALMOST_ALL => self.files = Files::AlmostAll,
            id if id == options::files::UNSORTED_ALL => {
                self.files = Files::All;
                self.sort = Some(Sort::None);
                if !self.color_explicit {
                    self.color = When::Never;
                }
            }
            id if id == options::SORT => self.sort = Some(sort_value(choice)),
            id if id == options::sort::SIZE => self.sort = Some(Sort::Size),
            id if id == options::sort::TIME => self.sort = Some(Sort::Time),
            id if id == options::sort::NONE => self.sort = Some(Sort::None),
            id if id == options::sort::VERSION => self.sort = Some(Sort::Version),
            id if id == options::sort::EXTENSION => self.sort = Some(Sort::Extension),
            id if id == options::TIME => {
                self.time = choice.into();
                self.time_explicit = true;
            }
            id if id == options::time::ACCESS => {
                self.time = MetadataTimeField::Access;
                self.time_explicit = true;
            }
            id if id == options::time::CHANGE => {
                self.time = MetadataTimeField::Change;
                self.time_explicit = true;
            }
            id if id == options::COLOR => {
                self.color = parse_when(choice);
                self.color_explicit = true;
            }
            id if id == options::HYPERLINK => self.hyperlink = parse_when(choice),
            id if id == options::QUOTING_STYLE => self.quoting_style = QuotingStyle::parse(choice),
            id if id == options::quoting::LITERAL => {
                self.quoting_style = Some(QuotingStyle::Literal {
                    show_control: false,
                });
            }
            id if id == options::quoting::ESCAPE => self.quoting_style = Some(QuotingStyle::Escape),
            id if id == options::quoting::C => self.quoting_style = Some(QuotingStyle::C_ALWAYS),
            id if id == options::HIDE_CONTROL_CHARS => self.show_control = Some(false),
            id if id == options::SHOW_CONTROL_CHARS => self.show_control = Some(true),
            id if id == options::INDICATOR_STYLE => self.indicator_style = indicator_value(choice),
            id if id == options::indicator_style::CLASSIFY => {
                self.indicator_style = parse_when(choice)
                    .enabled()
                    .then_some(IndicatorStyle::Classify);
            }
            id if id == options::indicator_style::FILE_TYPE => {
                self.indicator_style = Some(IndicatorStyle::FileType);
            }
            id if id == options::indicator_style::SLASH => {
                self.indicator_style = Some(IndicatorStyle::Slash);
            }
            id if id == options::dereference::ALL => self.dereference = Some(Dereference::All),
            id if id == options::dereference::ARGS => self.dereference = Some(Dereference::Args),
            id if id == options::dereference::DIR_ARGS => {
                self.dereference = Some(Dereference::DirArgs);
            }
            id if id == options::size::HUMAN_READABLE => self.size = SizeSetting::Binary,
            id if id == options::size::SI => self.size = SizeSetting::Decimal,
            id if id == options::size::KIBIBYTES => self.kibibytes = true,
            id if id == options::size::BLOCK_SIZE => {
                self.size = block_size_setting(&option, diag_args)?;
            }
            id if id == options::WIDTH => {
                let text = value
                    .to_str()
                    .ok_or_else(|| LsError::InvalidLineWidth(value.into()))?;
                self.width = Some(parse_width(Some(text))?);
            }
            id if id == options::format::TAB_SIZE => {
                let text = value
                    .to_str()
                    .ok_or_else(|| LsError::InvalidTabSize(value.into()))?;
                self.tab_size = Some(parse_tab_size(text)?);
            }
            id if id == options::HIDE => self.hide.push(value.into()),
            id if id == options::IGNORE => self.ignore.push(value.into()),
            id if id == options::IGNORE_BACKUPS => self.ignore_backups = true,
            id if id == options::NO_GROUP => self.long.group = false,
            id if id == options::AUTHOR => self.long.author = true,
            id if id == options::RECURSIVE => self.recursive = true,
            id if id == options::REVERSE => self.reverse = true,
            id if id == options::DIRECTORY => self.directory = true,
            id if id == options::INODE => self.inode = true,
            id if id == options::size::ALLOCATION_SIZE => self.alloc_size = true,
            id if id == options::CONTEXT => self.context = true,
            id if id == options::GROUP_DIRECTORIES_FIRST => self.group_directories_first = true,
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> UResult<Config> {
        let format = self.format.resolve(self.mode);
        let sort = self.sort.unwrap_or_else(|| {
            if format != Format::Long && self.time_explicit {
                Sort::Time
            } else {
                Sort::Name
            }
        });
        let (size_format, file_size_block_size, block_size) = match self.size {
            SizeSetting::Environment => {
                let (file, block) = resolve_block_sizes_from_env(self.kibibytes);
                (SizeFormat::Bytes, file, block)
            }
            SizeSetting::Binary => (
                SizeFormat::Binary,
                DEFAULT_FILE_SIZE_BLOCK_SIZE,
                DEFAULT_BLOCK_SIZE,
            ),
            SizeSetting::Decimal => (SizeFormat::Decimal, DEFAULT_FILE_SIZE_BLOCK_SIZE, 1000),
            SizeSetting::Blocks(size) => (SizeFormat::Bytes, size, size),
        };
        let width = self.width.map_or_else(|| parse_width(None), Ok)?;
        let show_control = self
            .show_control
            .unwrap_or_else(|| !matches!(self.mode, ProgramMode::Ls) || !stdout().is_terminal());
        let quoting_style = self
            .quoting_style
            .or_else(quoting_style_from_env)
            .unwrap_or_else(|| match self.mode {
                ProgramMode::Dir | ProgramMode::Vdir => QuotingStyle::Escape,
                ProgramMode::Ls if stdout().is_terminal() => QuotingStyle::SHELL_ESCAPE,
                ProgramMode::Ls => QuotingStyle::Literal { show_control },
            })
            .show_control(show_control);
        let mut needs_color = self.color.enabled() && is_color_compatible_term();
        if needs_color && let Err(err) = validate_ls_colors_env() {
            if let LsColorsParseError::UnrecognizedPrefix(prefix) = &err {
                show_error!(
                    "{}",
                    translate!("ls-error-unrecognized-ls-colors-prefix", "prefix" => prefix.quote())
                );
            }
            show_error!("{}", translate!("ls-error-unparsable-ls-colors"));
            needs_color = false;
        }
        let color = needs_color.then(|| LsColors::from_env().unwrap_or_default());
        let hyperlink = self.hyperlink.enabled();
        let dired = self.dired && format == Format::Long && !hyperlink;
        if dired && self.zero {
            return Err(Box::new(LsError::DiredAndZeroAreIncompatible));
        }
        let (time_format_recent, time_format_older) = if format == Format::Long {
            parse_time_style(resolve_time_style(
                self.time_style.as_ref(),
                self.full_time_at,
            ))?
        } else {
            Default::default()
        };
        let dereference = self.dereference.unwrap_or_else(|| {
            if self.directory
                || self.indicator_style == Some(IndicatorStyle::Classify)
                || format == Format::Long
            {
                Dereference::None
            } else {
                Dereference::DirArgs
            }
        });
        let mut ignore_patterns = Vec::new();
        if self.ignore_backups {
            for pattern in ["*~", ".*~"] {
                if let Ok(pattern) = Pattern::new(pattern) {
                    ignore_patterns.push(pattern);
                }
            }
        }
        for pattern in &self.ignore {
            add_pattern(&mut ignore_patterns, pattern, "ls-invalid-ignore-pattern");
        }
        if self.files == Files::Normal {
            for pattern in &self.hide {
                add_pattern(&mut ignore_patterns, pattern, "ls-invalid-hide-pattern");
            }
        }
        let tab_size = if needs_color {
            0
        } else if let Some(tab_size) = self.tab_size {
            tab_size
        } else {
            self.captured_tab_size
                .map(|raw| {
                    let text = raw
                        .to_str()
                        .ok_or_else(|| LsError::InvalidTabSize(raw.clone()))?;
                    parse_tab_size(text)
                })
                .transpose()?
                .unwrap_or(SPACES_IN_TAB)
        };
        Ok(Config {
            format,
            files: self.files,
            sort,
            recursive: self.recursive,
            reverse: self.reverse,
            dereference,
            ignore_patterns,
            size_format,
            directory: self.directory,
            time: self.time,
            #[cfg(unix)]
            inode: self.inode,
            color,
            long: self.long,
            alloc_size: self.alloc_size,
            file_size_block_size,
            block_size,
            width,
            quoting_style,
            show_control_chars: self
                .captured_show_control_chars
                .unwrap_or_else(|| self.show_control.unwrap_or(false)),
            indicator_style: self.indicator_style,
            time_format_recent,
            time_format_older,
            context: self.context,
            #[cfg(all(feature = "selinux", any(target_os = "linux", target_os = "android")))]
            selinux_supported: uucore::selinux::is_selinux_enabled(),
            #[cfg(all(feature = "smack", target_os = "linux"))]
            smack_supported: uucore::smack::is_smack_enabled(),
            group_directories_first: self.group_directories_first,
            line_ending: LineEnding::from_zero_flag(self.zero),
            dired,
            hyperlink,
            tab_size,
        })
    }
}

fn block_size_setting(option: &Occurrence, diag_args: Option<&[OsString]>) -> UResult<SizeSetting> {
    let value = option.value.as_deref().unwrap_or_else(|| OsStr::new(""));
    Ok(match value.to_str() {
        Some("human-readable") => SizeSetting::Binary,
        Some("si") => SizeSetting::Decimal,
        Some(text) => SizeSetting::Blocks(parse_size_non_zero_u64(text).map_err(|error| {
            let ls_error = LsError::BlockSizeParseError(value.into());
            let message = ls_error.to_string();
            let Some(position) = option.value_position else {
                return error.size_value_error(
                    diag_args,
                    &uucore::diagnostics::OptionValue::with_names(
                        text,
                        None,
                        Some(options::size::BLOCK_SIZE),
                    ),
                    0,
                    &message,
                    ls_error,
                );
            };
            uucore::diagnostics::error_after_report(diag_args, ls_error, |args, _| {
                use uucore::parser::parse_size::ParseSizeError;
                let label = match error {
                    ParseSizeError::InvalidSuffix(_) => {
                        Some(translate!("size-diag-label-invalid-suffix"))
                    }
                    ParseSizeError::SizeTooBig(_) => Some(translate!("size-diag-label-too-big")),
                    _ => None,
                };
                uucore::diagnostics::Snapshot::with_program(args).render_inside_at(
                    position.argument,
                    text,
                    error.span(text),
                    &message,
                    label.as_deref(),
                    Some(&translate!("size-diag-help-syntax")),
                )
            })
        })?),
        None => return Err(Box::new(LsError::BlockSizeParseError(value.into()))),
    })
}

fn parse_when(value: &str) -> When {
    match value {
        "always" => When::Always,
        "auto" => When::Auto,
        _ => When::Never,
    }
}

fn add_pattern(patterns: &mut Vec<Pattern>, raw: &OsStr, warning: &str) {
    if let Some(text) = raw.to_str()
        && let Ok(pattern) = parse_glob::from_str(text)
    {
        patterns.push(pattern);
    } else {
        show_warning!("{}", translate!(warning, "pattern" => raw.quote()));
    }
}

impl Settings {
    /// Ingest the values already resolved by a caller's Clap command. Defaults
    /// and captured environment values are settings, not later argv events.
    fn capture_clap(
        &mut self,
        matches: &clap::ArgMatches,
        diag_args: Option<&[OsString]>,
    ) -> UResult<()> {
        let get_flag = |id: &str| matches.get_flag(id);
        let last = |ids: &[&str]| {
            ids.iter()
                .map(|id| command_line_index(matches, id))
                .max()
                .unwrap_or(0)
        };
        let zero_at = command_line_index(matches, options::ZERO);

        self.format.selected = if let Some(value) =
            clap_choice(matches, options::FORMAT, diag_args)?
        {
            Some((
                match value {
                    "long" | "verbose" => Format::Long,
                    "single-column" => Format::OneLine,
                    "columns" | "vertical" => Format::Columns,
                    "across" | "horizontal" => Format::Across,
                    _ => Format::Commas,
                },
                last_clap_index(matches, options::FORMAT),
            ))
        } else {
            [
                (options::format::LONG, Format::Long),
                (options::format::ACROSS, Format::Across),
                (options::format::COMMAS, Format::Commas),
                (options::format::COLUMNS, Format::Columns),
            ]
            .into_iter()
            .find_map(|(id, format)| get_flag(id).then(|| (format, last_clap_index(matches, id))))
        };
        self.format.long_at = last(&[
            options::format::LONG_NO_OWNER,
            options::format::LONG_NO_GROUP,
            options::format::LONG_NUMERIC_UID_GID,
            options::FULL_TIME,
            options::DIRED,
        ]);
        self.format.one_line_at = command_line_index(matches, options::format::ONE_LINE);
        self.format.zero_at = zero_at;
        self.format.last_command_line_format = last(&[
            options::FORMAT,
            options::format::LONG,
            options::format::COLUMNS,
            options::format::ACROSS,
            options::format::COMMAS,
            options::format::LONG_NO_OWNER,
            options::format::LONG_NO_GROUP,
            options::format::LONG_NUMERIC_UID_GID,
            options::format::ONE_LINE,
        ]);

        let all = command_line_index(matches, options::files::ALL);
        let almost_all = command_line_index(matches, options::files::ALMOST_ALL);
        let unsorted_all = command_line_index(matches, options::files::UNSORTED_ALL);
        self.files = if all.max(almost_all).max(unsorted_all) == 0 {
            Files::Normal
        } else if almost_all == all.max(almost_all).max(unsorted_all) {
            Files::AlmostAll
        } else {
            Files::All
        };

        self.sort = clap_choice(matches, options::SORT, diag_args)?.map(sort_value);
        let mut sort_at = if self.sort.is_some() {
            last_clap_index(matches, options::SORT)
        } else {
            0
        };
        for (id, sort) in [
            (options::sort::TIME, Sort::Time),
            (options::sort::SIZE, Sort::Size),
            (options::sort::NONE, Sort::None),
            (options::sort::VERSION, Sort::Version),
            (options::sort::EXTENSION, Sort::Extension),
            (options::files::UNSORTED_ALL, Sort::None),
        ] {
            let index = command_line_index(matches, id);
            if index > sort_at {
                sort_at = index;
                self.sort = Some(sort);
            }
        }
        let time = clap_choice(matches, options::TIME, diag_args)?;
        self.time_explicit =
            time.is_some() || get_flag(options::time::ACCESS) || get_flag(options::time::CHANGE);
        self.time = if let Some(time) = time {
            time.into()
        } else if get_flag(options::time::ACCESS) {
            MetadataTimeField::Access
        } else if get_flag(options::time::CHANGE) {
            MetadataTimeField::Change
        } else {
            MetadataTimeField::Modification
        };

        let block_size = clap_value(matches, options::size::BLOCK_SIZE);
        let block_text = block_size
            .as_ref()
            .and_then(|value| value.value.as_ref())
            .and_then(|value| value.to_str());
        self.size = if get_flag(options::size::SI) || block_text == Some("si") {
            SizeSetting::Decimal
        } else if get_flag(options::size::HUMAN_READABLE) || block_text == Some("human-readable") {
            SizeSetting::Binary
        } else if let Some(block_size) = block_size {
            block_size_setting(&block_size, diag_args)?
        } else {
            SizeSetting::Environment
        };
        self.kibibytes = get_flag(options::size::KIBIBYTES);
        self.width = Some(parse_width(
            matches
                .get_one::<String>(options::WIDTH)
                .map(String::as_str),
        )?);
        self.tab_size = None;
        self.captured_tab_size = matches
            .get_one::<String>(options::format::TAB_SIZE)
            .map(OsString::from);
        self.time_style = matches
            .get_one::<String>(options::TIME_STYLE)
            .map(|value| (value.into(), last_clap_index(matches, options::TIME_STYLE)))
            .or_else(|| {
                std::env::var("TIME_STYLE")
                    .ok()
                    .map(|value| (value.into(), 0))
            });
        self.full_time_at =
            get_flag(options::FULL_TIME).then(|| last_clap_index(matches, options::FULL_TIME));

        self.color = clap_choice(matches, options::COLOR, diag_args)?.map_or_else(
            || {
                if matches.contains_id(options::COLOR) {
                    When::Always
                } else {
                    When::Never
                }
            },
            parse_when,
        );
        if zero_at > command_line_index(matches, options::COLOR) {
            self.color = When::Never;
        }
        self.color_explicit = matches.contains_id(options::COLOR);
        let dired_at = command_line_index(matches, options::DIRED);
        self.dired = dired_at > 0;
        self.hyperlink = if command_line_index(matches, options::HYPERLINK) > dired_at {
            clap_choice(matches, options::HYPERLINK, diag_args)?.map_or(When::Never, parse_when)
        } else {
            When::Never
        };
        self.show_control = if get_flag(options::HIDE_CONTROL_CHARS) {
            Some(false)
        } else if get_flag(options::SHOW_CONTROL_CHARS) {
            Some(true)
        } else {
            None
        };
        if zero_at > last(&[options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS]) {
            self.show_control = Some(true);
        }
        self.captured_show_control_chars = Some(get_flag(options::SHOW_CONTROL_CHARS));
        self.quoting_style =
            if let Some(value) = clap_choice(matches, options::QUOTING_STYLE, diag_args)? {
                QuotingStyle::parse(value)
            } else if get_flag(options::quoting::LITERAL) {
                Some(QuotingStyle::Literal {
                    show_control: false,
                })
            } else if get_flag(options::quoting::ESCAPE) {
                Some(QuotingStyle::Escape)
            } else if get_flag(options::quoting::C) {
                Some(QuotingStyle::C_ALWAYS)
            } else {
                None
            };
        if zero_at
            > last(&[
                options::QUOTING_STYLE,
                options::quoting::LITERAL,
                options::quoting::ESCAPE,
                options::quoting::C,
            ])
        {
            self.quoting_style = Some(QuotingStyle::Literal { show_control: true });
        }
        self.indicator_style =
            if let Some(value) = clap_choice(matches, options::INDICATOR_STYLE, diag_args)? {
                indicator_value(value)
            } else if let Some(value) =
                clap_choice(matches, options::indicator_style::CLASSIFY, diag_args)?
            {
                parse_when(value)
                    .enabled()
                    .then_some(IndicatorStyle::Classify)
            } else if get_flag(options::indicator_style::SLASH) {
                Some(IndicatorStyle::Slash)
            } else if get_flag(options::indicator_style::FILE_TYPE) {
                Some(IndicatorStyle::FileType)
            } else {
                None
            };
        self.dereference = if get_flag(options::dereference::ALL) {
            Some(Dereference::All)
        } else if get_flag(options::dereference::ARGS) {
            Some(Dereference::Args)
        } else if get_flag(options::dereference::DIR_ARGS) {
            Some(Dereference::DirArgs)
        } else {
            None
        };
        self.long = LongFormat {
            author: get_flag(options::AUTHOR),
            group: !get_flag(options::NO_GROUP) && !get_flag(options::format::LONG_NO_GROUP),
            owner: !get_flag(options::format::LONG_NO_OWNER),
            numeric_uid_gid: get_flag(options::format::LONG_NUMERIC_UID_GID),
        };
        self.ignore = matches
            .get_many::<String>(options::IGNORE)
            .into_iter()
            .flatten()
            .map(OsString::from)
            .collect();
        self.hide = matches
            .get_many::<String>(options::HIDE)
            .into_iter()
            .flatten()
            .map(OsString::from)
            .collect();
        self.ignore_backups = get_flag(options::IGNORE_BACKUPS);
        self.recursive = get_flag(options::RECURSIVE);
        self.reverse = get_flag(options::REVERSE);
        self.directory = get_flag(options::DIRECTORY);
        self.inode = get_flag(options::INODE);
        self.alloc_size = get_flag(options::size::ALLOCATION_SIZE);
        self.context = get_flag(options::CONTEXT);
        self.group_directories_first = get_flag(options::GROUP_DIRECTORIES_FIRST);
        self.zero = get_flag(options::ZERO);
        Ok(())
    }
}

impl Config {
    /// Compatibility entry point for callers which already parsed a Clap command.
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

    fn from_with_program_mode(
        options: &clap::ArgMatches,
        diag_args: Option<&[OsString]>,
        mode: ProgramMode,
    ) -> UResult<Self> {
        let mut settings = Settings::new(mode);
        settings.capture_clap(options, diag_args)?;
        settings.finish()
    }

    /// Check if leading info (inode and/or block size) should be displayed
    #[inline]
    pub(crate) fn should_display_leading_info(&self) -> bool {
        #[cfg(unix)]
        {
            self.inode || self.alloc_size
        }
        #[cfg(not(unix))]
        {
            self.alloc_size
        }
    }
}
fn parse_time_style(style: Option<&OsStr>) -> Result<(String, Option<String>), LsError> {
    // TODO: Using correct locale string is not implemented.
    const LOCALE_FORMAT: (&str, Option<&str>) = ("%b %e %H:%M", Some("%b %e  %Y"));

    // Convert time_styles references to owned String/option.
    #[expect(clippy::unnecessary_wraps, reason = "internal result helper")]
    fn ok((recent, older): (&str, Option<&str>)) -> Result<(String, Option<String>), LsError> {
        Ok((recent.to_string(), older.map(String::from)))
    }

    if let Some(raw) = style {
        let field = raw
            .to_str()
            .ok_or_else(|| LsError::TimeStyleParseError(raw.into()))?;
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
            field
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
                // Formats are (optionally) separated by a newline:
                // FORMAT1 NEWLINE FORMAT2 -> FORMAT1 for older files, FORMAT2 for recent.
                // A single format applies to all files (stored as recent).
                let formats: Vec<_> = field[1..].split('\n').collect();
                match formats.as_slice() {
                    [format] => ok((*format, None)),
                    [older, recent] => ok((*recent, Some(*older))),
                    _ => Err(LsError::TimeStyleParseError(field.into())),
                }
            }
            _ => Err(LsError::TimeStyleParseError(field.into())),
        }
    } else {
        ok(LOCALE_FORMAT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::discriminant;
    use uucore::parser::gnu::{Event, Parser, Policy};

    #[test]
    fn legacy_clap_configuration_uses_the_same_settings() {
        let _ = uucore::locale::setup_localization("ls");
        for args in [
            vec!["-l", "-1"],
            vec!["-g", "-C", "-1"],
            vec!["-o", "--format=long", "-n"],
            vec!["-f", "-A", "--sort=time", "-u"],
            vec!["--block-size=512", "-h", "--si", "--block-size=1024", "-k"],
            vec!["--quoting-style=shell", "-N", "-q"],
            vec!["--zero", "-l", "--show-control-chars"],
            vec!["-D", "--hyperlink=always", "--hyperlink=never"],
            vec!["--full-time", "--time-style=+DATE"],
            vec!["-H", "-L", "--dereference-command-line-symlink-to-dir"],
        ] {
            let command_line = std::iter::once("ls").chain(args.iter().copied());
            let matches = crate::uu_app().try_get_matches_from(command_line).unwrap();
            for mode in [ProgramMode::Ls, ProgramMode::Dir, ProgramMode::Vdir] {
                let mut settings = Settings::new(mode);
                for event in Parser::new(
                    args.iter().map(OsString::from),
                    crate::args::OPTIONS,
                    Policy::gnu(),
                ) {
                    if let Event::Option(option) = event.unwrap() {
                        settings.apply(option, None).unwrap();
                    }
                }
                let runtime = settings.finish().unwrap();
                let legacy = Config::from_with_program_mode(&matches, None, mode).unwrap();
                assert_eq!(runtime.format, legacy.format, "{args:?}");
                assert_eq!(
                    discriminant(&runtime.files),
                    discriminant(&legacy.files),
                    "{args:?}"
                );
                assert_eq!(
                    discriminant(&runtime.sort),
                    discriminant(&legacy.sort),
                    "{args:?}"
                );
                assert_eq!(
                    discriminant(&runtime.time),
                    discriminant(&legacy.time),
                    "{args:?}"
                );
                assert_eq!(
                    discriminant(&runtime.dereference),
                    discriminant(&legacy.dereference),
                    "{args:?}"
                );
                assert_eq!(runtime.quoting_style, legacy.quoting_style, "{args:?}");
                assert_eq!(runtime.long.group, legacy.long.group, "{args:?}");
                assert_eq!(runtime.long.owner, legacy.long.owner, "{args:?}");
                assert_eq!(
                    runtime.long.numeric_uid_gid, legacy.long.numeric_uid_gid,
                    "{args:?}"
                );
                assert!(runtime.size_format == legacy.size_format, "{args:?}");
                assert_eq!(
                    runtime.file_size_block_size, legacy.file_size_block_size,
                    "{args:?}"
                );
                assert_eq!(runtime.block_size, legacy.block_size, "{args:?}");
                assert_eq!(runtime.dired, legacy.dired, "{args:?}");
                assert_eq!(runtime.hyperlink, legacy.hyperlink, "{args:?}");
                assert_eq!(
                    runtime.time_format_recent, legacy.time_format_recent,
                    "{args:?}"
                );
            }
        }
    }
    #[test]
    fn legacy_clap_configuration_honors_custom_format_defaults() {
        // These results were checked against the original public Config API,
        // including its distinction between a mode default and a Clap value.
        let cases: &[(&str, &[&str], Format)] = &[
            ("long", &[], Format::Long),
            ("long", &["-1"], Format::Long),
            ("long", &["--zero"], Format::Long),
            ("long", &["-C"], Format::Long),
            ("columns", &["-g"], Format::Columns),
            ("columns", &["-1"], Format::Columns),
            ("columns", &["--zero"], Format::OneLine),
            ("columns", &["--zero", "-g"], Format::Columns),
            ("columns", &["-g", "--zero"], Format::OneLine),
            ("columns", &["--zero", "-C"], Format::Columns),
            ("columns", &["-C", "--zero"], Format::OneLine),
        ];
        for (default, arguments, expected) in cases {
            let matches = crate::uu_app()
                .mut_arg(options::FORMAT, |arg| arg.default_value(*default))
                .try_get_matches_from(std::iter::once("ls").chain(arguments.iter().copied()))
                .unwrap();
            for constructor in [Config::from, Config::from_dir, Config::from_vdir] {
                let config = constructor(&matches, None).unwrap();
                assert_eq!(
                    &config.format, expected,
                    "default={default}, args={arguments:?}"
                );
            }
        }
    }

    fn check_resolved_format_value(arguments: &[&str]) {
        let matches = crate::uu_app()
            .mut_arg(options::FORMAT, |arg| {
                arg.default_value("custom").value_parser(|value: &str| {
                    Ok::<String, String>(if value == "custom" { "long" } else { value }.to_owned())
                })
            })
            .try_get_matches_from(std::iter::once("ls").chain(arguments.iter().copied()))
            .unwrap();
        assert_eq!(matches.get_one::<String>(options::FORMAT).unwrap(), "long");
        for constructor in [Config::from, Config::from_dir, Config::from_vdir] {
            assert_eq!(constructor(&matches, None).unwrap().format, Format::Long);
        }
    }

    #[test]
    fn legacy_clap_configuration_uses_resolved_default_value() {
        check_resolved_format_value(&[]);
    }

    #[test]
    fn legacy_clap_configuration_uses_resolved_explicit_value() {
        check_resolved_format_value(&["--format", "custom"]);
    }

    #[test]
    fn legacy_clap_configuration_honors_resolved_scalar_defaults() {
        let cases = [
            (options::WIDTH, "7"),
            (options::size::BLOCK_SIZE, "2"),
            (options::SORT, "size"),
            (options::TIME, "atime"),
            (options::QUOTING_STYLE, "c"),
            (options::INDICATOR_STYLE, "slash"),
            (options::IGNORE, "a"),
            (options::HIDE, "a"),
            (options::REVERSE, "true"),
            (options::RECURSIVE, "true"),
            (options::format::LONG, "true"),
            (options::ZERO, "true"),
            (options::files::UNSORTED_ALL, "true"),
            (options::format::LONG_NO_OWNER, "true"),
        ];
        for (id, value) in cases {
            let matches = crate::uu_app()
                .mut_arg(id, |arg| arg.default_value(value))
                .try_get_matches_from(["ls"])
                .unwrap();
            for (constructor, mode_format) in [
                (Config::from as fn(_, _) -> _, Format::OneLine),
                (Config::from_dir, Format::Columns),
                (Config::from_vdir, Format::Long),
            ] {
                let config = constructor(&matches, None).unwrap();
                if id == options::WIDTH {
                    assert_eq!(config.width, 7);
                } else if id == options::size::BLOCK_SIZE {
                    assert_eq!(config.file_size_block_size, 2);
                    assert_eq!(config.block_size, 2);
                } else if id == options::SORT {
                    assert!(config.sort == Sort::Size);
                } else if id == options::TIME {
                    assert!(matches!(config.time, MetadataTimeField::Access));
                } else if id == options::QUOTING_STYLE {
                    assert_eq!(config.quoting_style, QuotingStyle::C_ALWAYS);
                } else if id == options::INDICATOR_STYLE {
                    assert!(config.indicator_style == Some(IndicatorStyle::Slash));
                } else if id == options::IGNORE || id == options::HIDE {
                    assert!(
                        config
                            .ignore_patterns
                            .iter()
                            .any(|pattern| pattern.matches("a"))
                    );
                } else if id == options::REVERSE {
                    assert!(config.reverse);
                } else if id == options::RECURSIVE {
                    assert!(config.recursive);
                } else if id == options::format::LONG {
                    assert_eq!(config.format, Format::Long);
                } else if id == options::ZERO {
                    assert_eq!(config.line_ending, LineEnding::Nul);
                    assert_eq!(config.format, mode_format);
                } else if id == options::files::UNSORTED_ALL {
                    assert!(config.files == Files::Normal);
                    assert!(config.sort == Sort::Name);
                } else if id == options::format::LONG_NO_OWNER {
                    assert!(!config.long.owner);
                    assert_eq!(config.format, mode_format);
                }
            }
        }
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn legacy_clap_configuration_uses_captured_environment() {
        const CHILD: &str = "UUTILS_LS_CONFIG_CAPTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "config::tests::legacy_clap_configuration_uses_captured_environment",
                    "--test-threads=1",
                ])
                .env(CHILD, "1")
                .env("TIME_STYLE", "+CAPTURED")
                .env("TABSIZE", "5")
                .env("TERM", "xterm")
                .env_remove("UUTILS_LS_CONFIG_ABSENT_CAPTURE")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "child failed:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let cases: &[(&[&str], &str, usize)] = &[
            (&["-l"], "CAPTURED", 5),
            (&["--full-time"], "CAPTURED", 5),
            (&["-C"], "", 5),
            (&["-l", "--time-style=+EXPLICIT"], "EXPLICIT", 5),
            (&["-C", "-T8"], "", 8),
        ];
        let captured: Vec<_> = cases
            .iter()
            .map(|(arguments, _, _)| {
                crate::uu_app()
                    .try_get_matches_from(std::iter::once("ls").chain(arguments.iter().copied()))
                    .unwrap()
            })
            .collect();
        let absent = crate::uu_app()
            .mut_arg(options::TIME_STYLE, |arg| {
                arg.env("UUTILS_LS_CONFIG_ABSENT_CAPTURE")
            })
            .mut_arg(options::format::TAB_SIZE, |arg| {
                arg.env("UUTILS_LS_CONFIG_ABSENT_CAPTURE")
            })
            .try_get_matches_from(["ls", "-l"])
            .unwrap();
        let captured_color = crate::uu_app()
            .mut_arg(options::COLOR, |arg| arg.default_value("always"))
            .try_get_matches_from(["ls"])
            .unwrap();
        let unused_tab_size = crate::uu_app()
            .mut_arg(options::format::TAB_SIZE, |arg| {
                arg.env("UUTILS_LS_CONFIG_ABSENT_CAPTURE")
                    .default_value("invalid")
            })
            .try_get_matches_from(["ls", "--color=always"])
            .unwrap();
        // SAFETY: this branch runs in an isolated child process, selected as
        // the only test. No other test can read or change this environment.
        unsafe {
            std::env::set_var("TIME_STYLE", "+LATER");
            std::env::set_var("TABSIZE", "1");
        }
        // TIME_STYLE historically has a live fallback when Clap captured no
        // value; TABSIZE has no such fallback and retains its default of 8.
        for constructor in [Config::from, Config::from_dir, Config::from_vdir] {
            let config = constructor(&absent, None).unwrap();
            assert_eq!(config.time_format_recent, "LATER");
            assert_eq!(config.tab_size, SPACES_IN_TAB);
        }
        for constructor in [Config::from, Config::from_dir, Config::from_vdir] {
            assert!(constructor(&captured_color, None).unwrap().color.is_some());
            assert_eq!(constructor(&unused_tab_size, None).unwrap().tab_size, 0);
        }
        for (matches, (arguments, time_format, tab_size)) in captured.iter().zip(cases) {
            for constructor in [Config::from, Config::from_dir, Config::from_vdir] {
                let config = constructor(matches, None).unwrap();
                assert_eq!(
                    &config.time_format_recent, time_format,
                    "args={arguments:?}"
                );
                assert_eq!(config.tab_size, *tab_size, "args={arguments:?}");
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;

            // SAFETY: the environment still belongs to this isolated child,
            // which executes only this test.
            unsafe {
                std::env::set_var("TIME_STYLE", OsString::from_vec(b"raw_\xff".to_vec()));
            }
            for constructor in [Config::from, Config::from_dir, Config::from_vdir] {
                let config = constructor(&absent, None).unwrap();
                assert_eq!(config.time_format_recent, "%b %e %H:%M");
                assert_eq!(config.tab_size, SPACES_IN_TAB);
            }
        }
    }
}
