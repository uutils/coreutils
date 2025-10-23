// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Although these links don't always seem to describe reality, check out the POSIX and GNU specs:
// https://pubs.opengroup.org/onlinepubs/9699919799/utilities/sort.html
// https://www.gnu.org/software/coreutils/manual/html_node/sort-invocation.html

// spell-checker:ignore (misc) HFKJFK Mbdfhn getrlimit RLIMIT_NOFILE rlim bigdecimal extendedbigdecimal hexdigit

mod buffer_hint;
mod check;
mod chunks;
mod custom_str_cmp;
mod ext_sort;
mod merge;
mod numeric_str_cmp;
mod tmp_dir;

use bigdecimal::BigDecimal;
use chunks::LineData;
use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use custom_str_cmp::custom_str_cmp;
use ext_sort::ext_sort;
use fnv::FnvHasher;
#[cfg(target_os = "linux")]
use nix::libc::{RLIMIT_NOFILE, getrlimit, rlimit};
use numeric_str_cmp::{NumInfo, NumInfoParseSettings, human_numeric_str_cmp, numeric_str_cmp};
use rand::{Rng, rng};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, BufWriter, Read, Write, stdin, stdout};
use std::num::IntErrorKind;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;
use std::str::Utf8Error;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, strip_errno};
use uucore::error::{UError, UResult, USimpleError, UUsageError};
use uucore::extendedbigdecimal::ExtendedBigDecimal;
use uucore::format_usage;
use uucore::line_ending::LineEnding;
use uucore::parser::num_parser::{ExtendedParser, ExtendedParserError};
use uucore::parser::parse_size::{ParseSizeError, Parser};
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::show_error;
use uucore::translate;
use uucore::version_cmp::version_cmp;

use crate::buffer_hint::automatic_buffer_size;
use crate::tmp_dir::TmpDirWrapper;

mod options {
    pub mod modes {
        pub const SORT: &str = "sort";

        pub const HUMAN_NUMERIC: &str = "human-numeric-sort";
        pub const MONTH: &str = "month-sort";
        pub const NUMERIC: &str = "numeric-sort";
        pub const GENERAL_NUMERIC: &str = "general-numeric-sort";
        pub const VERSION: &str = "version-sort";
        pub const RANDOM: &str = "random-sort";

        pub const ALL_SORT_MODES: [&str; 6] = [
            GENERAL_NUMERIC,
            HUMAN_NUMERIC,
            MONTH,
            NUMERIC,
            VERSION,
            RANDOM,
        ];
    }

    pub mod check {
        pub const CHECK: &str = "check";
        pub const CHECK_SILENT: &str = "check-silent";
        pub const SILENT: &str = "silent";
        pub const QUIET: &str = "quiet";
        pub const DIAGNOSE_FIRST: &str = "diagnose-first";
    }

    pub const HELP: &str = "help";
    pub const VERSION: &str = "version";
    pub const DICTIONARY_ORDER: &str = "dictionary-order";
    pub const MERGE: &str = "merge";
    pub const DEBUG: &str = "debug";
    pub const IGNORE_CASE: &str = "ignore-case";
    pub const IGNORE_LEADING_BLANKS: &str = "ignore-leading-blanks";
    pub const IGNORE_NONPRINTING: &str = "ignore-nonprinting";
    pub const OUTPUT: &str = "output";
    pub const REVERSE: &str = "reverse";
    pub const STABLE: &str = "stable";
    pub const UNIQUE: &str = "unique";
    pub const KEY: &str = "key";
    pub const SEPARATOR: &str = "field-separator";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const PARALLEL: &str = "parallel";
    pub const FILES0_FROM: &str = "files0-from";
    pub const BUF_SIZE: &str = "buffer-size";
    pub const TMP_DIR: &str = "temporary-directory";
    pub const COMPRESS_PROG: &str = "compress-program";
    pub const BATCH_SIZE: &str = "batch-size";

    pub const FILES: &str = "files";
}

const DECIMAL_PT: u8 = b'.';

const NEGATIVE: &u8 = &b'-';
const POSITIVE: &u8 = &b'+';

// The automatic buffer heuristics clamp to this range to avoid
// over-committing memory on constrained systems while still keeping
// reasonably large chunks for typical workloads.
const MIN_AUTOMATIC_BUF_SIZE: usize = 512 * 1024; // 512 KiB
const FALLBACK_AUTOMATIC_BUF_SIZE: usize = 32 * 1024 * 1024; // 32 MiB
const MAX_AUTOMATIC_BUF_SIZE: usize = 1024 * 1024 * 1024; // 1 GiB

#[derive(Debug, Error)]
pub enum SortError {
    #[error("{}", format_disorder(.file, .line_number, .line, .silent))]
    Disorder {
        file: OsString,
        line_number: usize,
        line: String,
        silent: bool,
    },

    #[error("{}", translate!("sort-open-failed", "path" => format!("{}", .path.maybe_quote()), "error" => strip_errno(.error)))]
    OpenFailed {
        path: PathBuf,
        error: std::io::Error,
    },

    #[error("{}", translate!("sort-parse-key-error", "key" => .key.quote(), "msg" => .msg.clone()))]
    ParseKeyError { key: String, msg: String },

    #[error("{}", translate!("sort-cannot-read", "path" => format!("{}", .path.maybe_quote()), "error" => strip_errno(.error)))]
    ReadFailed {
        path: PathBuf,
        error: std::io::Error,
    },

    #[error("{}", translate!("sort-open-tmp-file-failed", "error" => strip_errno(.error)))]
    OpenTmpFileFailed { error: std::io::Error },

    #[error("{}", translate!("sort-compress-prog-execution-failed", "code" => .code))]
    CompressProgExecutionFailed { code: i32 },

    #[error("{}", translate!("sort-compress-prog-terminated-abnormally", "prog" => .prog.quote()))]
    CompressProgTerminatedAbnormally { prog: String },

    #[error("{}", translate!("sort-cannot-create-tmp-file", "path" => format!("{}", .path.display())))]
    TmpFileCreationFailed { path: PathBuf },

    #[error("{}", translate!("sort-file-operands-combined", "file" => format!("{}", .file.display()), "help" => uucore::execution_phrase()))]
    FileOperandsCombined { file: PathBuf },

    #[error("{error}")]
    Uft8Error { error: Utf8Error },

    #[error("{}", translate!("sort-multiple-output-files"))]
    MultipleOutputFiles,

    #[error("{}", translate!("sort-minus-in-stdin"))]
    MinusInStdIn,

    #[error("{}", translate!("sort-no-input-from", "file" => format!("{}", .file.display())))]
    EmptyInputFile { file: PathBuf },

    #[error("{}", translate!("sort-invalid-zero-length-filename", "file" => format!("{}", .file.display()), "line_num" => .line_num))]
    ZeroLengthFileName { file: PathBuf, line_num: usize },
}

impl UError for SortError {
    fn code(&self) -> i32 {
        match self {
            Self::Disorder { .. } => 1,
            _ => 2,
        }
    }
}

fn format_disorder(file: &OsString, line_number: &usize, line: &String, silent: &bool) -> String {
    if *silent {
        String::new()
    } else {
        translate!("sort-error-disorder", "file" => file.maybe_quote(), "line_number" => line_number, "line" => line.to_owned())
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy, Debug)]
enum SortMode {
    Numeric,
    HumanNumeric,
    GeneralNumeric,
    Month,
    Version,
    Random,
    Default,
}

impl SortMode {
    fn get_short_name(&self) -> Option<char> {
        match self {
            Self::Numeric => Some('n'),
            Self::HumanNumeric => Some('h'),
            Self::GeneralNumeric => Some('g'),
            Self::Month => Some('M'),
            Self::Version => Some('V'),
            Self::Random => Some('R'),
            Self::Default => None,
        }
    }
}

pub struct Output {
    file: Option<(OsString, File)>,
}

impl Output {
    fn new(name: Option<impl AsRef<OsStr>>) -> UResult<Self> {
        let file = if let Some(name) = name {
            let path = Path::new(name.as_ref());
            // This is different from `File::create()` because we don't truncate the output yet.
            // This allows using the output file as an input file.
            #[allow(clippy::suspicious_open_options)]
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(path)
                .map_err(|e| SortError::OpenFailed {
                    path: path.to_owned(),
                    error: e,
                })?;
            Some((name.as_ref().to_owned(), file))
        } else {
            None
        };
        Ok(Self { file })
    }

    fn into_write(self) -> BufWriter<Box<dyn Write>> {
        BufWriter::new(match self.file {
            Some((_name, file)) => {
                // truncate the file
                let _ = file.set_len(0);
                Box::new(file)
            }
            None => Box::new(stdout()),
        })
    }

    fn as_output_name(&self) -> Option<&OsStr> {
        match &self.file {
            Some((name, _file)) => Some(name.as_os_str()),
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct GlobalSettings {
    mode: SortMode,
    debug: bool,
    ignore_leading_blanks: bool,
    ignore_case: bool,
    dictionary_order: bool,
    ignore_non_printing: bool,
    merge: bool,
    reverse: bool,
    stable: bool,
    unique: bool,
    check: bool,
    check_silent: bool,
    salt: Option<[u8; 16]>,
    selectors: Vec<FieldSelector>,
    separator: Option<u8>,
    threads: String,
    line_ending: LineEnding,
    buffer_size: usize,
    buffer_size_is_explicit: bool,
    compress_prog: Option<String>,
    merge_batch_size: usize,
    precomputed: Precomputed,
}

/// Data needed for sorting. Should be computed once before starting to sort
/// by calling `GlobalSettings::init_precomputed`.
#[derive(Clone, Debug, Default)]
struct Precomputed {
    needs_tokens: bool,
    num_infos_per_line: usize,
    floats_per_line: usize,
    selections_per_line: usize,
    fast_lexicographic: bool,
    fast_ascii_insensitive: bool,
}

impl GlobalSettings {
    /// Parse a SIZE string into a number of bytes.
    /// A size string comprises an integer and an optional unit.
    /// The unit may be k, K, m, M, g, G, t, T, P, E, Z, Y (powers of 1024), or b which is 1.
    /// Default is K.
    fn parse_byte_count(input: &str) -> Result<usize, ParseSizeError> {
        // GNU sort (8.32)   valid: 1b,        k, K, m, M, g, G, t, T, P, E, Z, Y
        // GNU sort (8.32) invalid:  b, B, 1B,                         p, e, z, y
        let size = Parser::default()
            .with_allow_list(&[
                "b", "k", "K", "m", "M", "g", "G", "t", "T", "P", "E", "Z", "Y", "R", "Q", "%",
            ])
            .with_default_unit("K")
            .with_b_byte_count(true)
            .parse(input.trim())?;

        usize::try_from(size).map_err(|_| {
            ParseSizeError::SizeTooBig(translate!("sort-error-buffer-size-too-big", "size" => size))
        })
    }

    /// Precompute some data needed for sorting.
    /// This function **must** be called before starting to sort, and `GlobalSettings` may not be altered
    /// afterwards.
    fn init_precomputed(&mut self) {
        self.precomputed.needs_tokens = self.selectors.iter().any(|s| s.needs_tokens);
        self.precomputed.selections_per_line =
            self.selectors.iter().filter(|s| s.needs_selection).count();
        self.precomputed.num_infos_per_line = self
            .selectors
            .iter()
            .filter(|s| matches!(s.settings.mode, SortMode::Numeric | SortMode::HumanNumeric))
            .count();
        self.precomputed.floats_per_line = self
            .selectors
            .iter()
            .filter(|s| matches!(s.settings.mode, SortMode::GeneralNumeric))
            .count();

        self.precomputed.fast_lexicographic = self.can_use_fast_lexicographic();
        self.precomputed.fast_ascii_insensitive = self.can_use_fast_ascii_insensitive();
    }

    /// Returns true when the fast lexicographic path can be used safely.
    fn can_use_fast_lexicographic(&self) -> bool {
        self.mode == SortMode::Default
            && !self.ignore_case
            && !self.dictionary_order
            && !self.ignore_non_printing
            && !self.ignore_leading_blanks
            && self.selectors.len() == 1
            && {
                let selector = &self.selectors[0];
                !selector.needs_selection
                    && matches!(selector.settings.mode, SortMode::Default)
                    && !selector.settings.ignore_case
                    && !selector.settings.dictionary_order
                    && !selector.settings.ignore_non_printing
                    && !selector.settings.ignore_blanks
            }
    }

    /// Returns true when the ASCII case-insensitive fast path is valid.
    fn can_use_fast_ascii_insensitive(&self) -> bool {
        self.mode == SortMode::Default
            && self.ignore_case
            && !self.dictionary_order
            && !self.ignore_non_printing
            && !self.ignore_leading_blanks
            && self.selectors.len() == 1
            && {
                let selector = &self.selectors[0];
                !selector.needs_selection
                    && matches!(selector.settings.mode, SortMode::Default)
                    && selector.settings.ignore_case
                    && !selector.settings.dictionary_order
                    && !selector.settings.ignore_non_printing
                    && !selector.settings.ignore_blanks
            }
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            mode: SortMode::Default,
            debug: false,
            ignore_leading_blanks: false,
            ignore_case: false,
            dictionary_order: false,
            ignore_non_printing: false,
            merge: false,
            reverse: false,
            stable: false,
            unique: false,
            check: false,
            check_silent: false,
            salt: None,
            selectors: vec![],
            separator: None,
            threads: String::new(),
            line_ending: LineEnding::Newline,
            buffer_size: FALLBACK_AUTOMATIC_BUF_SIZE,
            buffer_size_is_explicit: false,
            compress_prog: None,
            merge_batch_size: default_merge_batch_size(),
            precomputed: Precomputed::default(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
struct KeySettings {
    mode: SortMode,
    ignore_blanks: bool,
    ignore_case: bool,
    dictionary_order: bool,
    ignore_non_printing: bool,
    reverse: bool,
}

impl KeySettings {
    /// Checks if the supplied combination of `mode`, `ignore_non_printing` and `dictionary_order` is allowed.
    fn check_compatibility(
        mode: SortMode,
        ignore_non_printing: bool,
        dictionary_order: bool,
    ) -> Result<(), String> {
        if matches!(
            mode,
            SortMode::Numeric | SortMode::HumanNumeric | SortMode::GeneralNumeric | SortMode::Month
        ) {
            if dictionary_order {
                return Err(
                    translate!("sort-options-incompatible", "opt1" => "d", "opt2" => mode.get_short_name().unwrap()),
                );
            } else if ignore_non_printing {
                return Err(
                    translate!("sort-options-incompatible", "opt1" => "i", "opt2" => mode.get_short_name().unwrap()),
                );
            }
        }
        Ok(())
    }

    fn set_sort_mode(&mut self, mode: SortMode) -> Result<(), String> {
        if self.mode != SortMode::Default && self.mode != mode {
            return Err(
                translate!("sort-options-incompatible", "opt1" => self.mode.get_short_name().unwrap(), "opt2" => mode.get_short_name().unwrap()),
            );
        }
        Self::check_compatibility(mode, self.ignore_non_printing, self.dictionary_order)?;
        self.mode = mode;
        Ok(())
    }

    fn set_dictionary_order(&mut self) -> Result<(), String> {
        Self::check_compatibility(self.mode, self.ignore_non_printing, true)?;
        self.dictionary_order = true;
        Ok(())
    }

    fn set_ignore_non_printing(&mut self) -> Result<(), String> {
        Self::check_compatibility(self.mode, true, self.dictionary_order)?;
        self.ignore_non_printing = true;
        Ok(())
    }
}

impl From<&GlobalSettings> for KeySettings {
    fn from(settings: &GlobalSettings) -> Self {
        Self {
            mode: settings.mode,
            ignore_blanks: settings.ignore_leading_blanks,
            ignore_case: settings.ignore_case,
            ignore_non_printing: settings.ignore_non_printing,
            reverse: settings.reverse,
            dictionary_order: settings.dictionary_order,
        }
    }
}

impl Default for KeySettings {
    fn default() -> Self {
        Self::from(&GlobalSettings::default())
    }
}
enum Selection<'a> {
    AsBigDecimal(GeneralBigDecimalParseResult),
    WithNumInfo(&'a [u8], NumInfo),
    Str(&'a [u8]),
}

type Field = Range<usize>;

#[derive(Clone, Debug)]
pub struct Line<'a> {
    line: &'a [u8],
    index: usize,
}

impl<'a> Line<'a> {
    /// Creates a new `Line`.
    ///
    /// If additional data is needed for sorting it is added to `line_data`.
    /// `token_buffer` allows to reuse the allocation for tokens.
    fn create(
        line: &'a [u8],
        index: usize,
        line_data: &mut LineData<'a>,
        token_buffer: &mut Vec<Field>,
        settings: &GlobalSettings,
    ) -> Self {
        token_buffer.clear();
        if settings.precomputed.needs_tokens {
            tokenize(line, settings.separator, token_buffer);
        }
        if settings.mode == SortMode::Numeric {
            // exclude inf, nan, scientific notation
            let line_num_float = (!line.iter().any(u8::is_ascii_alphabetic))
                .then(|| std::str::from_utf8(line).ok())
                .flatten()
                .and_then(|s| s.parse::<f64>().ok());
            line_data.line_num_floats.push(line_num_float);
        }
        for (selector, selection) in settings
            .selectors
            .iter()
            .map(|selector| (selector, selector.get_selection(line, token_buffer)))
        {
            match selection {
                Selection::AsBigDecimal(parsed_float) => line_data.parsed_floats.push(parsed_float),
                Selection::WithNumInfo(str, num_info) => {
                    line_data.num_infos.push(num_info);
                    line_data.selections.push(str);
                }
                Selection::Str(str) => {
                    if selector.needs_selection {
                        line_data.selections.push(str);
                    }
                }
            }
        }
        Self { line, index }
    }

    fn print(&self, writer: &mut impl Write, settings: &GlobalSettings) -> std::io::Result<()> {
        if settings.debug {
            self.print_debug(settings, writer)?;
        } else {
            writer.write_all(self.line)?;
            writer.write_all(&[settings.line_ending.into()])?;
        }
        Ok(())
    }

    /// Writes indicators for the selections this line matched. The original line content is NOT expected
    /// to be already printed.
    fn print_debug(
        &self,
        settings: &GlobalSettings,
        writer: &mut impl Write,
    ) -> std::io::Result<()> {
        // We do not consider this function performance critical, as debug output is only useful for small files,
        // which are not a performance problem in any case. Therefore there aren't any special performance
        // optimizations here.

        let line = self
            .line
            .iter()
            .copied()
            .map(|c| if c == b'\t' { b'>' } else { c })
            .collect::<Vec<_>>();

        writer.write_all(&line)?;
        writeln!(writer)?;

        let mut fields = vec![];
        tokenize(self.line, settings.separator, &mut fields);
        for selector in &settings.selectors {
            let mut selection = selector.get_range(self.line, Some(&fields));
            match selector.settings.mode {
                SortMode::Numeric | SortMode::HumanNumeric => {
                    // find out which range is used for numeric comparisons
                    let (_, num_range) = NumInfo::parse(
                        &self.line[selection.clone()],
                        &NumInfoParseSettings {
                            accept_si_units: selector.settings.mode == SortMode::HumanNumeric,
                            ..Default::default()
                        },
                    );
                    let initial_selection = selection.clone();

                    // Shorten selection to num_range.
                    selection.start += num_range.start;
                    selection.end = selection.start + num_range.len();

                    if num_range == (0..0) {
                        // This was not a valid number.
                        // Report no match at the first non-whitespace character.
                        let leading_whitespace = self.line[selection.clone()]
                            .iter()
                            .position(|c| !c.is_ascii_whitespace())
                            .unwrap_or(0);
                        selection.start += leading_whitespace;
                        selection.end += leading_whitespace;
                    } else {
                        // include a trailing si unit
                        if selector.settings.mode == SortMode::HumanNumeric {
                            if let Some(
                                b'k' | b'K' | b'M' | b'G' | b'T' | b'P' | b'E' | b'Z' | b'Y' | b'R'
                                | b'Q',
                            ) = self.line[selection.end..initial_selection.end].first()
                            {
                                selection.end += 1;
                            }
                        }

                        // include leading zeroes, a leading minus or a leading decimal point
                        while let Some(b'-' | b'0' | b'.') =
                            self.line[initial_selection.start..selection.start].last()
                        {
                            selection.start -= 1;
                        }
                    }
                }
                SortMode::GeneralNumeric => {
                    let initial_selection = &self.line[selection.clone()];

                    let leading = get_leading_gen(initial_selection);

                    // Shorten selection to leading.
                    selection.start += leading.start;
                    selection.end = selection.start + leading.len();
                }
                SortMode::Month => {
                    let initial_selection = &self.line[selection.clone()];

                    let mut month_chars = initial_selection
                        .iter()
                        .enumerate()
                        .skip_while(|(_, c)| c.is_ascii_whitespace());

                    let month = if month_parse(initial_selection) == Month::Unknown {
                        // We failed to parse a month, which is equivalent to matching nothing.
                        // Add the "no match for key" marker to the first non-whitespace character.
                        let first_non_whitespace = month_chars.next();
                        first_non_whitespace.map_or(
                            initial_selection.len()..initial_selection.len(),
                            |(idx, _)| idx..idx,
                        )
                    } else {
                        // We parsed a month. Match the first three non-whitespace characters, which must be the month we parsed.
                        month_chars.next().unwrap().0
                            ..month_chars
                                .nth(2)
                                .map_or(initial_selection.len(), |(idx, _)| idx)
                    };

                    // Shorten selection to month.
                    selection.start += month.start;
                    selection.end = selection.start + month.len();
                }
                _ => {}
            }

            let select = &line[..selection.start];
            write!(writer, "{}", " ".repeat(select.len()))?;

            if selection.is_empty() {
                writeln!(writer, "{}", translate!("sort-error-no-match-for-key"))?;
            } else {
                let select = &line[selection];
                writeln!(writer, "{}", "_".repeat(select.len()))?;
            }
        }

        if settings.mode != SortMode::Random
            && !settings.stable
            && !settings.unique
            && (settings.dictionary_order
                || settings.ignore_leading_blanks
                || settings.ignore_case
                || settings.ignore_non_printing
                || settings.mode != SortMode::Default
                || settings
                    .selectors
                    .last()
                    .is_none_or(|selector| selector != &FieldSelector::default()))
        {
            // A last resort comparator is in use, underline the whole line.
            if self.line.is_empty() {
                writeln!(writer, "{}", translate!("sort-error-no-match-for-key"))?;
            } else {
                writeln!(writer, "{}", "_".repeat(self.line.len()))?;
            }
        }
        Ok(())
    }
}

/// Tokenize a line into fields. The result is stored into `token_buffer`.
fn tokenize(line: &[u8], separator: Option<u8>, token_buffer: &mut Vec<Field>) {
    assert!(token_buffer.is_empty());
    if let Some(separator) = separator {
        tokenize_with_separator(line, separator, token_buffer);
    } else {
        tokenize_default(line, token_buffer);
    }
}

/// By default fields are separated by the first whitespace after non-whitespace.
/// Whitespace is included in fields at the start.
/// The result is stored into `token_buffer`.
fn tokenize_default(line: &[u8], token_buffer: &mut Vec<Field>) {
    token_buffer.push(0..0);
    // pretend that there was whitespace in front of the line
    let mut previous_was_whitespace = true;
    for (idx, char) in line.iter().enumerate() {
        if char.is_ascii_whitespace() {
            if !previous_was_whitespace {
                token_buffer.last_mut().unwrap().end = idx;
                token_buffer.push(idx..0);
            }
            previous_was_whitespace = true;
        } else {
            previous_was_whitespace = false;
        }
    }
    token_buffer.last_mut().unwrap().end = line.len();
}

/// Split between separators. These separators are not included in fields.
/// The result is stored into `token_buffer`.
fn tokenize_with_separator(line: &[u8], separator: u8, token_buffer: &mut Vec<Field>) {
    let separator_indices = line
        .iter()
        .enumerate()
        .filter_map(|(i, &c)| if c == separator { Some(i) } else { None });
    let mut start = 0;
    for sep_idx in separator_indices {
        token_buffer.push(start..sep_idx);
        start = sep_idx + 1;
    }
    if start < line.len() {
        token_buffer.push(start..line.len());
    }
}

#[derive(Clone, PartialEq, Debug)]
struct KeyPosition {
    /// 1-indexed, 0 is invalid.
    field: usize,
    /// 1-indexed, 0 is end of field.
    char: usize,
    ignore_blanks: bool,
}

impl KeyPosition {
    fn new(key: &str, default_char_index: usize, ignore_blanks: bool) -> Result<Self, String> {
        let mut field_and_char = key.split('.');

        let field = field_and_char
            .next()
            .ok_or_else(|| translate!("sort-invalid-key", "key" => key.quote()))?;
        let char = field_and_char.next();

        let field = match field.parse::<usize>() {
            Ok(f) => f,
            Err(e) if *e.kind() == IntErrorKind::PosOverflow => usize::MAX,
            Err(e) => {
                return Err(
                    translate!("sort-failed-parse-field-index", "field" => field.quote(), "error" => e),
                );
            }
        };
        if field == 0 {
            return Err(translate!("sort-field-index-cannot-be-zero"));
        }

        let char = char.map_or(Ok(default_char_index), |char| {
            char.parse().map_err(|e: std::num::ParseIntError| {
                translate!("sort-failed-parse-char-index", "char" => char.quote(), "error" => e)
            })
        })?;

        Ok(Self {
            field,
            char,
            ignore_blanks,
        })
    }
}

impl Default for KeyPosition {
    fn default() -> Self {
        Self {
            field: 1,
            char: 1,
            ignore_blanks: false,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
struct FieldSelector {
    from: KeyPosition,
    to: Option<KeyPosition>,
    settings: KeySettings,
    needs_tokens: bool,
    // Whether this selector operates on a sub-slice of a line.
    // Selections are therefore not needed when this selector matches the whole line
    // or the sort mode is general-numeric.
    needs_selection: bool,
}

impl FieldSelector {
    /// Splits this position into the actual position and the attached options.
    fn split_key_options(position: &str) -> (&str, &str) {
        if let Some((options_start, _)) = position.char_indices().find(|(_, c)| c.is_alphabetic()) {
            position.split_at(options_start)
        } else {
            (position, "")
        }
    }

    fn parse(key: &str, global_settings: &GlobalSettings) -> UResult<Self> {
        let mut from_to = key.split(',');
        let (from, from_options) = Self::split_key_options(from_to.next().unwrap());
        let to = from_to.next().map(Self::split_key_options);
        let options_are_empty = from_options.is_empty() && matches!(to, None | Some((_, "")));

        if options_are_empty {
            // Inherit the global settings if there are no options attached to this key.
            (|| {
                // This would be ideal for a try block, I think. In the meantime this closure allows
                // to use the `?` operator here.
                Self::new(
                    KeyPosition::new(from, 1, global_settings.ignore_leading_blanks)?,
                    to.map(|(to, _)| {
                        KeyPosition::new(to, 0, global_settings.ignore_leading_blanks)
                    })
                    .transpose()?,
                    KeySettings::from(global_settings),
                )
            })()
        } else {
            // Do not inherit from `global_settings`, as there are options attached to this key.
            Self::parse_with_options((from, from_options), to)
        }
        .map_err(|msg| {
            SortError::ParseKeyError {
                key: key.to_owned(),
                msg,
            }
            .into()
        })
    }

    fn parse_with_options(
        (from, from_options): (&str, &str),
        to: Option<(&str, &str)>,
    ) -> Result<Self, String> {
        /// Applies `options` to `key_settings`, returning if the 'b'-flag (ignore blanks) was present.
        fn parse_key_settings(
            options: &str,
            key_settings: &mut KeySettings,
        ) -> Result<bool, String> {
            let mut ignore_blanks = false;
            for option in options.chars() {
                match option {
                    'M' => key_settings.set_sort_mode(SortMode::Month)?,
                    'b' => ignore_blanks = true,
                    'd' => key_settings.set_dictionary_order()?,
                    'f' => key_settings.ignore_case = true,
                    'g' => key_settings.set_sort_mode(SortMode::GeneralNumeric)?,
                    'h' => key_settings.set_sort_mode(SortMode::HumanNumeric)?,
                    'i' => key_settings.set_ignore_non_printing()?,
                    'n' => key_settings.set_sort_mode(SortMode::Numeric)?,
                    'R' => key_settings.set_sort_mode(SortMode::Random)?,
                    'r' => key_settings.reverse = true,
                    'V' => key_settings.set_sort_mode(SortMode::Version)?,
                    c => {
                        return Err(translate!("sort-invalid-option", "option" => c));
                    }
                }
            }
            Ok(ignore_blanks)
        }

        let mut key_settings = KeySettings::default();
        let from = parse_key_settings(from_options, &mut key_settings)
            .map(|ignore_blanks| KeyPosition::new(from, 1, ignore_blanks))??;
        let to = if let Some((to, to_options)) = to {
            Some(
                parse_key_settings(to_options, &mut key_settings)
                    .map(|ignore_blanks| KeyPosition::new(to, 0, ignore_blanks))??,
            )
        } else {
            None
        };
        Self::new(from, to, key_settings)
    }

    fn new(
        from: KeyPosition,
        to: Option<KeyPosition>,
        settings: KeySettings,
    ) -> Result<Self, String> {
        if from.char == 0 {
            Err(translate!("sort-invalid-char-index-zero-start"))
        } else {
            Ok(Self {
                needs_selection: (from.field != 1
                    || from.char != 1
                    || to.is_some()
                    || matches!(settings.mode, SortMode::Numeric | SortMode::HumanNumeric)
                    || from.ignore_blanks)
                    && !matches!(settings.mode, SortMode::GeneralNumeric),
                needs_tokens: from.field != 1 || from.char == 0 || to.is_some(),
                from,
                to,
                settings,
            })
        }
    }

    /// Get the selection that corresponds to this selector for the line.
    /// If `needs_fields` returned false, tokens may be empty.
    fn get_selection<'a>(&self, line: &'a [u8], tokens: &[Field]) -> Selection<'a> {
        // `get_range` expects `None` when we don't need tokens and would get confused by an empty vector.
        let tokens = if self.needs_tokens {
            Some(tokens)
        } else {
            None
        };
        let mut range_str = &line[self.get_range(line, tokens)];
        if self.settings.mode == SortMode::Numeric || self.settings.mode == SortMode::HumanNumeric {
            // Parse NumInfo for this number.
            let (info, num_range) = NumInfo::parse(
                range_str,
                &NumInfoParseSettings {
                    accept_si_units: self.settings.mode == SortMode::HumanNumeric,
                    ..Default::default()
                },
            );
            // Shorten the range to what we need to pass to numeric_str_cmp later.
            range_str = &range_str[num_range];
            Selection::WithNumInfo(range_str, info)
        } else if self.settings.mode == SortMode::GeneralNumeric {
            // Parse this number as BigDecimal, as this is the requirement for general numeric sorting.
            Selection::AsBigDecimal(general_bd_parse(&range_str[get_leading_gen(range_str)]))
        } else {
            // This is not a numeric sort, so we don't need a NumCache.
            Selection::Str(range_str)
        }
    }

    /// Look up the range in the line that corresponds to this selector.
    /// If `needs_fields` returned false, tokens must be None.
    fn get_range(&self, line: &[u8], tokens: Option<&[Field]>) -> Range<usize> {
        enum Resolution {
            // The start index of the resolved character, inclusive
            StartOfChar(usize),
            // The end index of the resolved character, exclusive.
            // This is only returned if the character index is 0.
            EndOfChar(usize),
            // The resolved character would be in front of the first character
            TooLow,
            // The resolved character would be after the last character
            TooHigh,
        }

        /// Get the index for this line given the [`KeyPosition`]
        fn resolve_index(
            line: &[u8],
            tokens: Option<&[Field]>,
            position: &KeyPosition,
        ) -> Resolution {
            if matches!(tokens, Some(tokens) if tokens.len() < position.field) {
                Resolution::TooHigh
            } else if position.char == 0 {
                let end = tokens.unwrap()[position.field - 1].end;
                if end == 0 {
                    Resolution::TooLow
                } else {
                    Resolution::EndOfChar(end)
                }
            } else {
                let mut idx = if position.field == 1 {
                    // The first field always starts at 0.
                    // We don't need tokens for this case.
                    0
                } else {
                    tokens.unwrap()[position.field - 1].start
                };
                // strip blanks if needed
                if position.ignore_blanks {
                    idx += line[idx..]
                        .iter()
                        .enumerate()
                        .find(|(_, c)| !c.is_ascii_whitespace())
                        .map_or(line[idx..].len(), |(idx, _)| idx);
                }
                // apply the character index
                idx += line[idx..]
                    .iter()
                    .enumerate()
                    .nth(position.char - 1)
                    .map_or(line[idx..].len(), |(idx, _)| idx);
                if idx >= line.len() {
                    Resolution::TooHigh
                } else {
                    Resolution::StartOfChar(idx)
                }
            }
        }

        match resolve_index(line, tokens, &self.from) {
            Resolution::StartOfChar(from) => {
                let to = self.to.as_ref().map(|to| resolve_index(line, tokens, to));

                let mut range = match to {
                    Some(Resolution::StartOfChar(mut to)) => {
                        // We need to include the character at `to`.
                        to += 1;
                        from..to
                    }
                    Some(Resolution::EndOfChar(to)) => from..to,
                    // If `to` was not given or the match would be after the end of the line,
                    // match everything until the end of the line.
                    None | Some(Resolution::TooHigh) => from..line.len(),
                    // If `to` is before the start of the line, report no match.
                    // This can happen if the line starts with a separator.
                    Some(Resolution::TooLow) => 0..0,
                };
                if range.start > range.end {
                    range.end = range.start;
                }
                range
            }
            Resolution::TooLow | Resolution::EndOfChar(_) => {
                unreachable!(
                    "This should only happen if the field start index is 0, but that should already have caused an error."
                )
            }
            // While for comparisons it's only important that this is an empty slice,
            // to produce accurate debug output we need to match an empty slice at the end of the line.
            Resolution::TooHigh => line.len()..line.len(),
        }
    }
}

/// Creates an `Arg` that conflicts with all other sort modes.
fn make_sort_mode_arg(mode: &'static str, short: char, help: String) -> Arg {
    Arg::new(mode)
        .short(short)
        .long(mode)
        .help(help)
        .action(ArgAction::SetTrue)
        .conflicts_with_all(
            options::modes::ALL_SORT_MODES
                .iter()
                .filter(|&&m| m != mode),
        )
}

#[cfg(target_os = "linux")]
fn get_rlimit() -> UResult<usize> {
    let mut limit = rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    match unsafe { getrlimit(RLIMIT_NOFILE, &raw mut limit) } {
        0 => Ok(limit.rlim_cur as usize),
        _ => Err(UUsageError::new(2, translate!("sort-failed-fetch-rlimit"))),
    }
}

const STDIN_FILE: &str = "-";
#[cfg(target_os = "linux")]
const LINUX_BATCH_DIVISOR: usize = 4;
#[cfg(target_os = "linux")]
const LINUX_BATCH_MIN: usize = 32;
#[cfg(target_os = "linux")]
const LINUX_BATCH_MAX: usize = 256;

fn default_merge_batch_size() -> usize {
    #[cfg(target_os = "linux")]
    {
        // Adjust merge batch size dynamically based on available file descriptors.
        match get_rlimit() {
            Ok(limit) => {
                let usable_limit = limit.saturating_div(LINUX_BATCH_DIVISOR);
                usable_limit.clamp(LINUX_BATCH_MIN, LINUX_BATCH_MAX)
            }
            Err(_) => 64,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        64
    }
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut settings = GlobalSettings::default();

    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 2)?;

    // Prevent -o/--output to be specified multiple times
    if matches
        .get_occurrences::<OsString>(options::OUTPUT)
        .is_some_and(|out| out.len() > 1)
    {
        return Err(SortError::MultipleOutputFiles.into());
    }

    settings.debug = matches.get_flag(options::DEBUG);

    // check whether user specified a zero terminated list of files for input, otherwise read files from args
    let mut files: Vec<OsString> = if matches.contains_id(options::FILES0_FROM) {
        let files0_from: PathBuf = matches
            .get_one::<OsString>(options::FILES0_FROM)
            .map(|v| v.into())
            .unwrap_or_default();

        // Cannot combine FILES with FILES0_FROM
        if let Some(s) = matches.get_one::<OsString>(options::FILES) {
            return Err(SortError::FileOperandsCombined { file: s.into() }.into());
        }

        let mut files = Vec::new();

        // sort errors with "cannot open: [...]" instead of "cannot read: [...]" here
        let reader = open_with_open_failed_error(&files0_from)?;
        let buf_reader = BufReader::new(reader);
        for (line_num, line) in buf_reader.split(b'\0').flatten().enumerate() {
            let f = std::str::from_utf8(&line)
                .expect("Could not parse string from zero terminated input.");
            match f {
                STDIN_FILE => {
                    return Err(SortError::MinusInStdIn.into());
                }
                "" => {
                    return Err(SortError::ZeroLengthFileName {
                        file: files0_from,
                        line_num: line_num + 1,
                    }
                    .into());
                }
                _ => {}
            }

            files.push(OsString::from(
                std::str::from_utf8(&line)
                    .expect("Could not parse string from zero terminated input."),
            ));
        }
        if files.is_empty() {
            return Err(SortError::EmptyInputFile { file: files0_from }.into());
        }
        files
    } else {
        matches
            .get_many::<OsString>(options::FILES)
            .map(|v| v.map(ToOwned::to_owned).collect())
            .unwrap_or_default()
    };

    settings.mode = if matches.get_flag(options::modes::HUMAN_NUMERIC)
        || matches
            .get_one::<String>(options::modes::SORT)
            .is_some_and(|s| s == "human-numeric")
    {
        SortMode::HumanNumeric
    } else if matches.get_flag(options::modes::MONTH)
        || matches
            .get_one::<String>(options::modes::SORT)
            .is_some_and(|s| s == "month")
    {
        SortMode::Month
    } else if matches.get_flag(options::modes::GENERAL_NUMERIC)
        || matches
            .get_one::<String>(options::modes::SORT)
            .is_some_and(|s| s == "general-numeric")
    {
        SortMode::GeneralNumeric
    } else if matches.get_flag(options::modes::NUMERIC)
        || matches
            .get_one::<String>(options::modes::SORT)
            .is_some_and(|s| s == "numeric")
    {
        SortMode::Numeric
    } else if matches.get_flag(options::modes::VERSION)
        || matches
            .get_one::<String>(options::modes::SORT)
            .is_some_and(|s| s == "version")
    {
        SortMode::Version
    } else if matches.get_flag(options::modes::RANDOM)
        || matches
            .get_one::<String>(options::modes::SORT)
            .is_some_and(|s| s == "random")
    {
        settings.salt = Some(get_rand_string());
        SortMode::Random
    } else {
        SortMode::Default
    };

    settings.dictionary_order = matches.get_flag(options::DICTIONARY_ORDER);
    settings.ignore_non_printing = matches.get_flag(options::IGNORE_NONPRINTING);
    if matches.contains_id(options::PARALLEL) {
        // "0" is default - threads = num of cores
        settings.threads = matches
            .get_one::<String>(options::PARALLEL)
            .map_or_else(|| "0".to_string(), String::from);
        unsafe {
            env::set_var("RAYON_NUM_THREADS", &settings.threads);
        }
    }

    if let Some(size_str) = matches.get_one::<String>(options::BUF_SIZE) {
        settings.buffer_size = GlobalSettings::parse_byte_count(size_str).map_err(|e| {
            USimpleError::new(2, format_error_message(&e, size_str, options::BUF_SIZE))
        })?;
        settings.buffer_size_is_explicit = true;
    } else {
        settings.buffer_size = automatic_buffer_size(&files);
        settings.buffer_size_is_explicit = false;
    }

    let mut tmp_dir = TmpDirWrapper::new(
        matches
            .get_one::<String>(options::TMP_DIR)
            .map_or_else(env::temp_dir, PathBuf::from),
    );

    settings.compress_prog = matches
        .get_one::<String>(options::COMPRESS_PROG)
        .map(String::from);

    if let Some(n_merge) = matches.get_one::<String>(options::BATCH_SIZE) {
        match n_merge.parse::<usize>() {
            Ok(parsed_value) => {
                if parsed_value < 2 {
                    show_error!(
                        "{}",
                        translate!("sort-invalid-batch-size-arg", "arg" => n_merge)
                    );
                    return Err(UUsageError::new(
                        2,
                        translate!("sort-minimum-batch-size-two"),
                    ));
                }
                settings.merge_batch_size = parsed_value;
            }
            Err(e) => {
                let error_message = if *e.kind() == IntErrorKind::PosOverflow {
                    let batch_too_large = translate!(
                        "sort-batch-size-too-large",
                        "arg" => n_merge.quote()
                    );

                    #[cfg(target_os = "linux")]
                    {
                        show_error!("{}", batch_too_large);

                        translate!(
                            "sort-maximum-batch-size-rlimit",
                            "rlimit" =>  get_rlimit()?
                        )
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        batch_too_large
                    }
                } else {
                    translate!(
                        "sort-invalid-batch-size-arg",
                        "arg" =>  n_merge,
                    )
                };

                return Err(UUsageError::new(2, error_message));
            }
        }
    }

    settings.line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));
    settings.merge = matches.get_flag(options::MERGE);

    settings.check = matches.contains_id(options::check::CHECK);
    if matches.get_flag(options::check::CHECK_SILENT)
        || matches!(
            matches
                .get_one::<String>(options::check::CHECK)
                .map(|s| s.as_str()),
            Some(options::check::SILENT | options::check::QUIET)
        )
    {
        settings.check_silent = true;
        settings.check = true;
    }

    settings.ignore_case = matches.get_flag(options::IGNORE_CASE);

    settings.ignore_leading_blanks = matches.get_flag(options::IGNORE_LEADING_BLANKS);

    settings.reverse = matches.get_flag(options::REVERSE);
    settings.stable = matches.get_flag(options::STABLE);
    settings.unique = matches.get_flag(options::UNIQUE);

    if files.is_empty() {
        /* if no file, default to stdin */
        files.push(OsString::from(STDIN_FILE));
    } else if settings.check && files.len() != 1 {
        return Err(UUsageError::new(
            2,
            translate!("sort-extra-operand-not-allowed-with-c", "operand" => files[1].quote()),
        ));
    }

    if let Some(arg) = matches.get_one::<OsString>(options::SEPARATOR) {
        let mut separator = arg.to_str().ok_or_else(|| {
            UUsageError::new(
                2,
                translate!("sort-separator-not-valid-unicode", "arg" => arg.quote()),
            )
        })?;
        if separator == "\\0" {
            separator = "\0";
        }
        // This rejects non-ASCII codepoints, but perhaps we don't have to.
        // On the other hand GNU accepts any single byte, valid unicode or not.
        // (Supporting multi-byte chars would require changes in tokenize_with_separator().)
        let &[sep_char] = separator.as_bytes() else {
            return Err(UUsageError::new(
                2,
                translate!("sort-separator-must-be-one-char", "separator" => separator.quote()),
            ));
        };
        settings.separator = Some(sep_char);
    }

    if let Some(values) = matches.get_many::<String>(options::KEY) {
        for value in values {
            let selector = FieldSelector::parse(value, &settings)?;
            if selector.settings.mode == SortMode::Random && settings.salt.is_none() {
                settings.salt = Some(get_rand_string());
            }
            settings.selectors.push(selector);
        }
    }

    if !matches.contains_id(options::KEY) {
        // add a default selector matching the whole line
        let key_settings = KeySettings::from(&settings);
        settings.selectors.push(
            FieldSelector::new(
                KeyPosition {
                    field: 1,
                    char: 1,
                    ignore_blanks: key_settings.ignore_blanks,
                },
                None,
                key_settings,
            )
            .unwrap(),
        );
    }

    // Verify that we can open all input files.
    // It is the correct behavior to close all files afterwards,
    // and to reopen them at a later point. This is different from how the output file is handled,
    // probably to prevent running out of file descriptors.
    for file in &files {
        open(file)?;
    }

    let output = Output::new(matches.get_one::<OsString>(options::OUTPUT))?;

    settings.init_precomputed();

    let result = exec(&mut files, &settings, output, &mut tmp_dir);
    // Wait here if `SIGINT` was received,
    // for signal handler to do its work and terminate the program.
    tmp_dir.wait_if_signal();
    result
}

pub fn uu_app() -> Command {
    uucore::clap_localization::configure_localized_command(
        Command::new(uucore::util_name())
            .version(uucore::crate_version!())
            .about(translate!("sort-about"))
            .after_help(translate!("sort-after-help"))
            .override_usage(format_usage(&translate!("sort-usage"))),
    )
    .infer_long_args(true)
    .disable_help_flag(true)
    .disable_version_flag(true)
    .args_override_self(true)
    .arg(
        Arg::new(options::HELP)
            .long(options::HELP)
            .help(translate!("sort-help-help"))
            .action(ArgAction::Help),
    )
    .arg(
        Arg::new(options::VERSION)
            .long(options::VERSION)
            .help(translate!("sort-help-version"))
            .action(ArgAction::Version),
    )
    .arg(
        Arg::new(options::modes::SORT)
            .long(options::modes::SORT)
            .value_parser(ShortcutValueParser::new([
                "general-numeric",
                "human-numeric",
                "month",
                "numeric",
                "version",
                "random",
            ]))
            .conflicts_with_all(options::modes::ALL_SORT_MODES),
    )
    .arg(make_sort_mode_arg(
        options::modes::HUMAN_NUMERIC,
        'h',
        translate!("sort-help-human-numeric"),
    ))
    .arg(make_sort_mode_arg(
        options::modes::MONTH,
        'M',
        translate!("sort-help-month"),
    ))
    .arg(make_sort_mode_arg(
        options::modes::NUMERIC,
        'n',
        translate!("sort-help-numeric"),
    ))
    .arg(make_sort_mode_arg(
        options::modes::GENERAL_NUMERIC,
        'g',
        translate!("sort-help-general-numeric"),
    ))
    .arg(make_sort_mode_arg(
        options::modes::VERSION,
        'V',
        translate!("sort-help-version-sort"),
    ))
    .arg(make_sort_mode_arg(
        options::modes::RANDOM,
        'R',
        translate!("sort-help-random"),
    ))
    .arg(
        Arg::new(options::DICTIONARY_ORDER)
            .short('d')
            .long(options::DICTIONARY_ORDER)
            .help(translate!("sort-help-dictionary-order"))
            .conflicts_with_all([
                options::modes::NUMERIC,
                options::modes::GENERAL_NUMERIC,
                options::modes::HUMAN_NUMERIC,
                options::modes::MONTH,
            ])
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::MERGE)
            .short('m')
            .long(options::MERGE)
            .help(translate!("sort-help-merge"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::check::CHECK)
            .short('c')
            .long(options::check::CHECK)
            .require_equals(true)
            .num_args(0..)
            .value_parser(ShortcutValueParser::new([
                options::check::SILENT,
                options::check::QUIET,
                options::check::DIAGNOSE_FIRST,
            ]))
            .conflicts_with_all([options::OUTPUT, options::check::CHECK_SILENT])
            .help(translate!("sort-help-check")),
    )
    .arg(
        Arg::new(options::check::CHECK_SILENT)
            .short('C')
            .long(options::check::CHECK_SILENT)
            .conflicts_with_all([options::OUTPUT, options::check::CHECK])
            .help(translate!("sort-help-check-silent"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::IGNORE_CASE)
            .short('f')
            .long(options::IGNORE_CASE)
            .help(translate!("sort-help-ignore-case"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::IGNORE_NONPRINTING)
            .short('i')
            .long(options::IGNORE_NONPRINTING)
            .help(translate!("sort-help-ignore-nonprinting"))
            .conflicts_with_all([
                options::modes::NUMERIC,
                options::modes::GENERAL_NUMERIC,
                options::modes::HUMAN_NUMERIC,
                options::modes::MONTH,
            ])
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::IGNORE_LEADING_BLANKS)
            .short('b')
            .long(options::IGNORE_LEADING_BLANKS)
            .help(translate!("sort-help-ignore-leading-blanks"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::OUTPUT)
            .short('o')
            .long(options::OUTPUT)
            .help(translate!("sort-help-output"))
            .value_parser(ValueParser::os_string())
            .value_name("FILENAME")
            .value_hint(clap::ValueHint::FilePath)
            .num_args(1)
            .allow_hyphen_values(true)
            // To detect multiple occurrences and raise an error
            .action(ArgAction::Append),
    )
    .arg(
        Arg::new(options::REVERSE)
            .short('r')
            .long(options::REVERSE)
            .help(translate!("sort-help-reverse"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::STABLE)
            .short('s')
            .long(options::STABLE)
            .help(translate!("sort-help-stable"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::UNIQUE)
            .short('u')
            .long(options::UNIQUE)
            .help(translate!("sort-help-unique"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::KEY)
            .short('k')
            .long(options::KEY)
            .help(translate!("sort-help-key"))
            .action(ArgAction::Append)
            .num_args(1),
    )
    .arg(
        Arg::new(options::SEPARATOR)
            .short('t')
            .long(options::SEPARATOR)
            .help(translate!("sort-help-separator"))
            .value_parser(ValueParser::os_string()),
    )
    .arg(
        Arg::new(options::ZERO_TERMINATED)
            .short('z')
            .long(options::ZERO_TERMINATED)
            .help(translate!("sort-help-zero-terminated"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::PARALLEL)
            .long(options::PARALLEL)
            .help(translate!("sort-help-parallel"))
            .value_name("NUM_THREADS"),
    )
    .arg(
        Arg::new(options::BUF_SIZE)
            .short('S')
            .long(options::BUF_SIZE)
            .help(translate!("sort-help-buf-size"))
            .value_name("SIZE"),
    )
    .arg(
        Arg::new(options::TMP_DIR)
            .short('T')
            .long(options::TMP_DIR)
            .help(translate!("sort-help-tmp-dir"))
            .value_name("DIR")
            .value_hint(clap::ValueHint::DirPath),
    )
    .arg(
        Arg::new(options::COMPRESS_PROG)
            .long(options::COMPRESS_PROG)
            .help(translate!("sort-help-compress-prog"))
            .value_name("PROG")
            .value_hint(clap::ValueHint::CommandName),
    )
    .arg(
        Arg::new(options::BATCH_SIZE)
            .long(options::BATCH_SIZE)
            .help(translate!("sort-help-batch-size"))
            .value_name("N_MERGE"),
    )
    .arg(
        Arg::new(options::FILES0_FROM)
            .long(options::FILES0_FROM)
            .help(translate!("sort-help-files0-from"))
            .value_name("NUL_FILE")
            .value_parser(ValueParser::os_string())
            .value_hint(clap::ValueHint::FilePath),
    )
    .arg(
        Arg::new(options::DEBUG)
            .long(options::DEBUG)
            .help(translate!("sort-help-debug"))
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new(options::FILES)
            .action(ArgAction::Append)
            .value_parser(ValueParser::os_string())
            .value_hint(clap::ValueHint::FilePath),
    )
}

fn exec(
    files: &mut [OsString],
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    if settings.merge {
        merge::merge(files, settings, output, tmp_dir)
    } else if settings.check {
        if files.len() > 1 {
            Err(UUsageError::new(
                2,
                translate!("sort-only-one-file-allowed-with-c"),
            ))
        } else {
            check::check(files.first().unwrap(), settings)
        }
    } else {
        let mut lines = files.iter().map(open);
        ext_sort(&mut lines, settings, output, tmp_dir)
    }
}

fn sort_by<'a>(unsorted: &mut Vec<Line<'a>>, settings: &GlobalSettings, line_data: &LineData<'a>) {
    if settings.stable || settings.unique {
        unsorted.par_sort_by(|a, b| compare_by(a, b, settings, line_data, line_data));
    } else {
        unsorted.par_sort_unstable_by(|a, b| compare_by(a, b, settings, line_data, line_data));
    }
}

fn compare_by<'a>(
    a: &Line<'a>,
    b: &Line<'a>,
    global_settings: &GlobalSettings,
    a_line_data: &LineData<'a>,
    b_line_data: &LineData<'a>,
) -> Ordering {
    if global_settings.precomputed.fast_lexicographic {
        let cmp = a.line.cmp(b.line);
        return if global_settings.reverse {
            cmp.reverse()
        } else {
            cmp
        };
    }

    if global_settings.precomputed.fast_ascii_insensitive {
        let cmp = ascii_case_insensitive_cmp(a.line, b.line);
        if cmp != Ordering::Equal || a.line == b.line {
            return if global_settings.reverse {
                cmp.reverse()
            } else {
                cmp
            };
        }
    }

    let mut selection_index = 0;
    let mut num_info_index = 0;
    let mut parsed_float_index = 0;

    if let (Some(Some(a_f64)), Some(Some(b_f64))) = (
        a_line_data.line_num_floats.get(a.index),
        b_line_data.line_num_floats.get(b.index),
    ) {
        // we don't use total_cmp() because it always sorts -0 before 0
        if let Some(cmp) = a_f64.partial_cmp(b_f64) {
            // don't trust `Ordering::Equal` if lines are not fully equal
            if cmp != Ordering::Equal || a.line == b.line {
                return if global_settings.reverse {
                    cmp.reverse()
                } else {
                    cmp
                };
            }
        }
    }

    for selector in &global_settings.selectors {
        let (a_str, b_str) = if selector.needs_selection {
            let selections = (
                a_line_data.selections
                    [a.index * global_settings.precomputed.selections_per_line + selection_index],
                b_line_data.selections
                    [b.index * global_settings.precomputed.selections_per_line + selection_index],
            );
            selection_index += 1;
            selections
        } else {
            // We can select the whole line.
            (a.line, b.line)
        };

        let settings = &selector.settings;

        let cmp: Ordering = match settings.mode {
            SortMode::Random => {
                // check if the two strings are equal
                if custom_str_cmp(
                    a_str,
                    b_str,
                    settings.ignore_non_printing,
                    settings.dictionary_order,
                    settings.ignore_case,
                ) == Ordering::Equal
                {
                    Ordering::Equal
                } else {
                    // Only if they are not equal compare by the hash
                    random_shuffle(a_str, b_str, &global_settings.salt.unwrap())
                }
            }
            SortMode::Numeric => {
                let a_num_info = &a_line_data.num_infos
                    [a.index * global_settings.precomputed.num_infos_per_line + num_info_index];
                let b_num_info = &b_line_data.num_infos
                    [b.index * global_settings.precomputed.num_infos_per_line + num_info_index];
                num_info_index += 1;
                numeric_str_cmp((a_str, a_num_info), (b_str, b_num_info))
            }
            SortMode::HumanNumeric => {
                let a_num_info = &a_line_data.num_infos
                    [a.index * global_settings.precomputed.num_infos_per_line + num_info_index];
                let b_num_info = &b_line_data.num_infos
                    [b.index * global_settings.precomputed.num_infos_per_line + num_info_index];
                num_info_index += 1;
                human_numeric_str_cmp((a_str, a_num_info), (b_str, b_num_info))
            }
            SortMode::GeneralNumeric => {
                let a_float = &a_line_data.parsed_floats
                    [a.index * global_settings.precomputed.floats_per_line + parsed_float_index];
                let b_float = &b_line_data.parsed_floats
                    [b.index * global_settings.precomputed.floats_per_line + parsed_float_index];
                parsed_float_index += 1;
                general_numeric_compare(a_float, b_float)
            }
            SortMode::Month => month_compare(a_str, b_str),
            SortMode::Version => version_cmp(a_str, b_str),
            SortMode::Default => custom_str_cmp(
                a_str,
                b_str,
                settings.ignore_non_printing,
                settings.dictionary_order,
                settings.ignore_case,
            ),
        };
        if cmp != Ordering::Equal {
            return if settings.reverse { cmp.reverse() } else { cmp };
        }
    }

    // Call "last resort compare" if all selectors returned Equal
    let cmp = if global_settings.mode == SortMode::Random
        || global_settings.stable
        || global_settings.unique
    {
        Ordering::Equal
    } else {
        a.line.cmp(b.line)
    };

    if global_settings.reverse {
        cmp.reverse()
    } else {
        cmp
    }
}

/// Compare two byte slices in ASCII case-insensitive order without allocating.
/// We lower each byte on the fly so that binary input (including `NUL`) stays
/// untouched and we avoid locale-sensitive routines such as `strcasecmp`.
fn ascii_case_insensitive_cmp(a: &[u8], b: &[u8]) -> Ordering {
    #[inline]
    fn lower(byte: u8) -> u8 {
        byte.to_ascii_lowercase()
    }

    for (lhs, rhs) in a.iter().copied().zip(b.iter().copied()) {
        let l = lower(lhs);
        let r = lower(rhs);
        if l != r {
            return l.cmp(&r);
        }
    }

    a.len().cmp(&b.len())
}

// This function cleans up the initial comparison done by leading_num_common for a general numeric compare.
// In contrast to numeric compare, GNU general numeric/FP sort *should* recognize positive signs and
// scientific notation, so we strip those lines only after the end of the following numeric string.
// For example, 5e10KFD would be 5e10 or 5x10^10 and +10000HFKJFK would become 10000.
#[allow(clippy::cognitive_complexity)]
fn get_leading_gen(inp: &[u8]) -> Range<usize> {
    let trimmed = inp.trim_ascii_start();
    let leading_whitespace_len = inp.len() - trimmed.len();

    // check for inf, -inf and nan
    const ALLOWED_PREFIXES: &[&[u8]] = &[b"inf", b"-inf", b"nan"];
    for &allowed_prefix in ALLOWED_PREFIXES {
        if trimmed.len() >= allowed_prefix.len()
            && trimmed[..allowed_prefix.len()].eq_ignore_ascii_case(allowed_prefix)
        {
            return leading_whitespace_len..(leading_whitespace_len + allowed_prefix.len());
        }
    }
    // Make this iter peekable to see if next char is numeric
    let mut char_indices = itertools::peek_nth(trimmed.iter().enumerate());

    let first = char_indices.peek();

    if matches!(first, Some((_, NEGATIVE | POSITIVE))) {
        char_indices.next();
    }

    let mut had_e_notation = false;
    let mut had_decimal_pt = false;
    let mut had_hex_notation: bool = false;
    while let Some((idx, &c)) = char_indices.next() {
        if had_hex_notation && c.is_ascii_hexdigit() {
            continue;
        }

        if c.is_ascii_digit() {
            if c == b'0' && matches!(char_indices.peek(), Some((_, b'x' | b'X'))) {
                had_hex_notation = true;
                char_indices.next();
            }
            continue;
        }

        if c == DECIMAL_PT && !had_decimal_pt && !had_e_notation {
            had_decimal_pt = true;
            continue;
        }
        let is_decimal_e = (c == b'e' || c == b'E') && !had_hex_notation;
        let is_hex_e = (c == b'p' || c == b'P') && had_hex_notation;
        if (is_decimal_e || is_hex_e) && !had_e_notation {
            // we can only consume the 'e' if what follow is either a digit, or a sign followed by a digit.
            if let Some(&(_, &next_char)) = char_indices.peek() {
                if (next_char == b'+' || next_char == b'-')
                    && matches!(
                        char_indices.peek_nth(2),
                        Some((_, c)) if c.is_ascii_digit()
                    )
                {
                    // Consume the sign. The following digits will be consumed by the main loop.
                    char_indices.next();
                    had_e_notation = true;
                    continue;
                }
                if next_char.is_ascii_digit() {
                    had_e_notation = true;
                    continue;
                }
            }
        }
        return leading_whitespace_len..(leading_whitespace_len + idx);
    }
    leading_whitespace_len..inp.len()
}

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum GeneralBigDecimalParseResult {
    Invalid,
    Nan,
    MinusInfinity,
    Number(BigDecimal),
    Infinity,
}

/// Parse the beginning string into a [`GeneralBigDecimalParseResult`].
/// Using a [`GeneralBigDecimalParseResult`] instead of [`ExtendedBigDecimal`] is necessary to correctly order floats.
#[inline(always)]
fn general_bd_parse(a: &[u8]) -> GeneralBigDecimalParseResult {
    // The string should be valid ASCII to be parsed.
    let Ok(a) = std::str::from_utf8(a) else {
        return GeneralBigDecimalParseResult::Invalid;
    };

    // Parse digits, and fold in recoverable errors
    let ebd = match ExtendedBigDecimal::extended_parse(a) {
        Err(ExtendedParserError::NotNumeric) => return GeneralBigDecimalParseResult::Invalid,
        Err(
            ExtendedParserError::PartialMatch(ebd, _)
            | ExtendedParserError::Overflow(ebd)
            | ExtendedParserError::Underflow(ebd),
        )
        | Ok(ebd) => ebd,
    };

    match ebd {
        ExtendedBigDecimal::BigDecimal(bd) => GeneralBigDecimalParseResult::Number(bd),
        ExtendedBigDecimal::Infinity => GeneralBigDecimalParseResult::Infinity,
        ExtendedBigDecimal::MinusInfinity => GeneralBigDecimalParseResult::MinusInfinity,
        // Minus zero and zero are equal
        ExtendedBigDecimal::MinusZero => GeneralBigDecimalParseResult::Number(0.into()),
        ExtendedBigDecimal::Nan | ExtendedBigDecimal::MinusNan => GeneralBigDecimalParseResult::Nan,
    }
}

/// Compares two floats, with errors and non-numerics assumed to be -inf.
/// Stops coercing at the first non-numeric char.
/// We explicitly need to convert to f64 in this case.
fn general_numeric_compare(
    a: &GeneralBigDecimalParseResult,
    b: &GeneralBigDecimalParseResult,
) -> Ordering {
    a.partial_cmp(b).unwrap()
}

fn get_rand_string() -> [u8; 16] {
    rng().sample(rand::distr::StandardUniform)
}

fn get_hash<T: Hash>(t: &T) -> u64 {
    let mut s = FnvHasher::default();
    t.hash(&mut s);
    s.finish()
}

fn random_shuffle(a: &[u8], b: &[u8], salt: &[u8]) -> Ordering {
    let da = get_hash(&(a, salt));
    let db = get_hash(&(b, salt));
    da.cmp(&db)
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
enum Month {
    Unknown,
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

/// Parse the beginning string into a Month, returning [`Month::Unknown`] on errors.
fn month_parse(line: &[u8]) -> Month {
    let line = line.trim_ascii_start();

    match line.get(..3).map(|x| x.to_ascii_uppercase()).as_deref() {
        Some(b"JAN") => Month::January,
        Some(b"FEB") => Month::February,
        Some(b"MAR") => Month::March,
        Some(b"APR") => Month::April,
        Some(b"MAY") => Month::May,
        Some(b"JUN") => Month::June,
        Some(b"JUL") => Month::July,
        Some(b"AUG") => Month::August,
        Some(b"SEP") => Month::September,
        Some(b"OCT") => Month::October,
        Some(b"NOV") => Month::November,
        Some(b"DEC") => Month::December,
        _ => Month::Unknown,
    }
}

fn month_compare(a: &[u8], b: &[u8]) -> Ordering {
    let ma = month_parse(a);
    let mb = month_parse(b);

    ma.cmp(&mb)
}

fn print_sorted<'a, T: Iterator<Item = &'a Line<'a>>>(
    iter: T,
    settings: &GlobalSettings,
    output: Output,
) -> UResult<()> {
    let output_name = output
        .as_output_name()
        .unwrap_or(OsStr::new("standard output"))
        .to_owned();
    let ctx = || translate!("sort-error-write-failed", "output" => output_name.maybe_quote());

    let mut writer = output.into_write();
    for line in iter {
        line.print(&mut writer, settings).map_err_context(ctx)?;
    }
    writer.flush().map_err_context(ctx)?;
    Ok(())
}

fn open(path: impl AsRef<OsStr>) -> UResult<Box<dyn Read + Send>> {
    let path = path.as_ref();
    if path == STDIN_FILE {
        let stdin = stdin();
        return Ok(Box::new(stdin) as Box<dyn Read + Send>);
    }

    let path = Path::new(path);
    match File::open(path) {
        Ok(f) => Ok(Box::new(f) as Box<dyn Read + Send>),
        Err(error) => Err(SortError::ReadFailed {
            path: path.to_owned(),
            error,
        }
        .into()),
    }
}

fn open_with_open_failed_error(path: impl AsRef<OsStr>) -> UResult<Box<dyn Read + Send>> {
    // On error, returns an OpenFailed error instead of a ReadFailed error
    let path = path.as_ref();
    if path == STDIN_FILE {
        let stdin = stdin();
        return Ok(Box::new(stdin) as Box<dyn Read + Send>);
    }

    let path = Path::new(path);
    match File::open(path) {
        Ok(f) => Ok(Box::new(f) as Box<dyn Read + Send>),
        Err(error) => Err(SortError::OpenFailed {
            path: path.to_owned(),
            error,
        }
        .into()),
    }
}

fn format_error_message(error: &ParseSizeError, s: &str, option: &str) -> String {
    // NOTE:
    // GNU's sort echos affected flag, -S or --buffer-size, depending on user's selection
    match error {
        ParseSizeError::InvalidSuffix(_) => {
            translate!("sort-invalid-suffix-in-option-arg", "option" => option, "arg" => s.quote())
        }
        ParseSizeError::ParseFailure(_) | ParseSizeError::PhysicalMem(_) => {
            translate!("sort-invalid-option-arg", "option" => option, "arg" => s.quote())
        }
        ParseSizeError::SizeTooBig(_) => {
            translate!("sort-option-arg-too-large", "option" => option, "arg" => s.quote())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn tokenize_helper(line: &[u8], separator: Option<u8>) -> Vec<Field> {
        let mut buffer = vec![];
        tokenize(line, separator, &mut buffer);
        buffer
    }

    #[test]
    fn test_get_hash() {
        let a = "Ted".to_string();

        assert_eq!(2_646_829_031_758_483_623, get_hash(&a));
    }

    #[test]
    fn test_random_shuffle() {
        let a = b"Ted";
        let b = b"Ted";
        let c = get_rand_string();

        assert_eq!(Ordering::Equal, random_shuffle(a, b, &c));
    }

    #[test]
    fn test_month_compare() {
        let a = b"JaN";
        let b = b"OCt";

        assert_eq!(Ordering::Less, month_compare(a, b));
    }
    #[test]
    fn test_version_compare() {
        let a = b"1.2.3-alpha2";
        let b = b"1.4.0";

        assert_eq!(Ordering::Less, version_cmp(a, b));
    }

    #[test]
    fn test_random_compare() {
        let a = b"9";
        let b = b"9";
        let c = get_rand_string();

        assert_eq!(Ordering::Equal, random_shuffle(a, b, &c));
    }

    #[test]
    fn test_tokenize_fields() {
        let line = b"foo bar b    x";
        assert_eq!(tokenize_helper(line, None), vec![0..3, 3..7, 7..9, 9..14]);
    }

    #[test]
    fn test_tokenize_fields_leading_whitespace() {
        let line = b"    foo bar b    x";
        assert_eq!(
            tokenize_helper(line, None),
            vec![0..7, 7..11, 11..13, 13..18]
        );
    }

    #[test]
    fn test_tokenize_fields_custom_separator() {
        let line = b"aaa foo bar b    x";
        assert_eq!(
            tokenize_helper(line, Some(b'a')),
            vec![0..0, 1..1, 2..2, 3..9, 10..18]
        );
    }

    #[test]
    fn test_tokenize_fields_trailing_custom_separator() {
        let line = b"a";
        assert_eq!(tokenize_helper(line, Some(b'a')), vec![0..0]);
        let line = b"aa";
        assert_eq!(tokenize_helper(line, Some(b'a')), vec![0..0, 1..1]);
        let line = b"..a..a";
        assert_eq!(tokenize_helper(line, Some(b'a')), vec![0..2, 3..5]);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_line_size() {
        // We should make sure to not regress the size of the Line struct because
        // it is unconditional overhead for every line we sort.
        assert_eq!(size_of::<Line>(), 24);
    }

    #[test]
    fn test_parse_byte_count() {
        let valid_input = [
            ("0", 0),
            ("50K", 50 * 1024),
            ("50k", 50 * 1024),
            ("1M", 1024 * 1024),
            ("100M", 100 * 1024 * 1024),
            #[cfg(not(target_pointer_width = "32"))]
            ("1000G", 1000 * 1024 * 1024 * 1024),
            #[cfg(not(target_pointer_width = "32"))]
            ("10T", 10 * 1024 * 1024 * 1024 * 1024),
            ("1b", 1),
            ("1024b", 1024),
            ("1024Mb", 1024 * 1024 * 1024), // NOTE: This might not be how GNU `sort` behaves for 'Mb'
            ("1", 1024),                    // K is default
            ("50", 50 * 1024),
            ("K", 1024),
            ("k", 1024),
            ("m", 1024 * 1024),
            #[cfg(not(target_pointer_width = "32"))]
            ("E", 1024 * 1024 * 1024 * 1024 * 1024 * 1024),
        ];
        for (input, expected_output) in &valid_input {
            assert_eq!(
                GlobalSettings::parse_byte_count(input),
                Ok(*expected_output)
            );
        }

        // SizeTooBig
        let invalid_input = ["500E", "1Y"];
        for input in &invalid_input {
            assert!(GlobalSettings::parse_byte_count(input).is_err());
        }

        // ParseFailure
        let invalid_input = ["nonsense", "1B", "B", "b", "p", "e", "z", "y"];
        for input in &invalid_input {
            assert!(GlobalSettings::parse_byte_count(input).is_err());
        }
    }
}
