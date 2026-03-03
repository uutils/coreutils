// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Although these links don't always seem to describe reality, check out the POSIX and GNU specs:
// https://pubs.opengroup.org/onlinepubs/9699919799/utilities/sort.html
// https://www.gnu.org/software/coreutils/manual/html_node/sort-invocation.html

// spell-checker:ignore (misc) HFKJFK Mbdfhn getrlimit RLIMIT_NOFILE rlim bigdecimal extendedbigdecimal hexdigit behaviour keydef GETFD localeconv

mod buffer_hint;
mod check;
mod chunks;
mod custom_str_cmp;
mod ext_sort;
mod merge;
mod numeric_str_cmp;
mod tmp_dir;

use ahash::AHashMap;
use bigdecimal::BigDecimal;
use chunks::LineData;
use clap::builder::ValueParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use custom_str_cmp::custom_str_cmp;
use ext_sort::ext_sort;
use numeric_str_cmp::{NumInfo, NumInfoParseSettings, human_numeric_str_cmp, numeric_str_cmp};
use rand::{Rng, rng};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::hash::{BuildHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, BufWriter, Read, Write, stdin, stdout};
use std::num::{IntErrorKind, NonZero};
use std::ops::Range;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use std::str::Utf8Error;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, strip_errno};
use uucore::error::{UError, UResult, USimpleError, UUsageError};
use uucore::extendedbigdecimal::ExtendedBigDecimal;
#[cfg(feature = "i18n-collator")]
use uucore::i18n::collator::locale_cmp;
use uucore::i18n::decimal::locale_decimal_separator;
use uucore::line_ending::LineEnding;
use uucore::parser::num_parser::{ExtendedParser, ExtendedParserError};
use uucore::parser::parse_size::{ParseSizeError, Parser};
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::posix::{MODERN, TRADITIONAL};
use uucore::show_error;
use uucore::translate;
use uucore::version_cmp::version_cmp;
use uucore::{format_usage, i18n};

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
    pub const RANDOM_SOURCE: &str = "random-source";

    pub const FILES: &str = "files";
}

const DECIMAL_PT: u8 = b'.';

fn locale_decimal_pt() -> u8 {
    match locale_decimal_separator().as_bytes().first().copied() {
        Some(b'.') => b'.',
        Some(b',') => b',',
        _ => DECIMAL_PT,
    }
}

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

    #[error("{}", translate!("sort-cannot-read", "path" => format!("{}", .path.maybe_quote()), "error" => strip_errno(.error)))]
    ReadFailed {
        path: PathBuf,
        error: std::io::Error,
    },

    #[error("{}", translate!("sort-open-tmp-file-failed", "error" => strip_errno(.error)))]
    OpenTmpFileFailed { error: std::io::Error },

    #[error("{}", translate!("sort-compress-prog-execution-failed", "prog" => .prog, "error" => strip_errno(.error)))]
    CompressProgExecutionFailed { prog: String, error: std::io::Error },

    #[error("{}", translate!("sort-compress-prog-terminated-abnormally", "prog" => .prog.quote()))]
    CompressProgTerminatedAbnormally { prog: String },

    #[error("{}", translate!("sort-cannot-create-tmp-file", "path" => format!("{}", .path.quote())))]
    TmpFileCreationFailed { path: PathBuf },

    #[error("{}", translate!("sort-file-operands-combined", "file" => format!("{}", .file.quote()), "help" => uucore::execution_phrase()))]
    FileOperandsCombined { file: PathBuf },

    #[error("{error}")]
    Uft8Error { error: Utf8Error },

    #[error("{}", translate!("sort-multiple-output-files"))]
    MultipleOutputFiles,

    #[error("{}", translate!("sort-minus-in-stdin"))]
    MinusInStdIn,

    #[error("{}", translate!("sort-no-input-from", "file" => format!("{}", .file.quote())))]
    EmptyInputFile { file: PathBuf },

    #[error("{}", translate!("sort-invalid-zero-length-filename", "file" => .file.maybe_quote(), "line_num" => .line_num))]
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

// refs are required because this fn is used by thiserror macro
#[expect(clippy::trivially_copy_pass_by_ref)]
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

/// Return the length of the byte slice while ignoring embedded NULs (used for debug underline alignment).
fn count_non_null_bytes(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&c| c != b'\0').count()
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
    random_source: Option<PathBuf>,
    selectors: Vec<FieldSelector>,
    separator: Option<u8>,
    threads: String,
    line_ending: LineEnding,
    buffer_size: usize,
    buffer_size_is_explicit: bool,
    compress_prog: Option<String>,
    merge_batch_size: usize,
    numeric_locale: NumericLocaleSettings,
    precomputed: Precomputed,
}

#[derive(Clone, Copy, Debug)]
struct NumericLocaleSettings {
    thousands_sep: Option<u8>,
    decimal_pt: Option<u8>,
}

impl Default for NumericLocaleSettings {
    fn default() -> Self {
        Self {
            thousands_sep: None,
            decimal_pt: Some(DECIMAL_PT),
        }
    }
}

impl NumericLocaleSettings {
    fn num_info_settings(self, accept_si_units: bool) -> NumInfoParseSettings {
        NumInfoParseSettings {
            accept_si_units,
            thousands_separator: self.thousands_sep,
            decimal_pt: self.decimal_pt,
        }
    }
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
    tokenize_blank_thousands_sep: bool,
    tokenize_allow_unit_after_blank: bool,
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
    ///
    /// When i18n-collator is enabled, `disable_fast_lexicographic` should be set to true if we're
    /// in a UTF-8 locale (to force locale-aware collation instead of byte comparison).
    fn init_precomputed(&mut self, disable_fast_lexicographic: bool) {
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

        let uses_numeric = self
            .selectors
            .iter()
            .any(|s| matches!(s.settings.mode, SortMode::Numeric | SortMode::HumanNumeric));
        let uses_human_numeric = self
            .selectors
            .iter()
            .any(|s| matches!(s.settings.mode, SortMode::HumanNumeric));
        self.precomputed.tokenize_blank_thousands_sep = self.separator.is_none()
            && uses_numeric
            && self.numeric_locale.thousands_sep == Some(b' ');
        self.precomputed.tokenize_allow_unit_after_blank =
            self.precomputed.tokenize_blank_thousands_sep && uses_human_numeric;

        self.precomputed.fast_lexicographic =
            !disable_fast_lexicographic && self.can_use_fast_lexicographic();
        self.precomputed.fast_ascii_insensitive = self.can_use_fast_ascii_insensitive();
    }

    /// Returns true when the fast lexicographic path can be used safely.
    /// Note: When i18n-collator is enabled, the caller must have already determined
    /// whether locale-aware collation is needed (via checking if we're in a UTF-8 locale).
    /// This check is performed in uumain() before init_precomputed() is called.
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
            random_source: None,
            selectors: vec![],
            separator: None,
            threads: String::new(),
            line_ending: LineEnding::Newline,
            buffer_size: FALLBACK_AUTOMATIC_BUF_SIZE,
            buffer_size_is_explicit: false,
            compress_prog: None,
            merge_batch_size: default_merge_batch_size(),
            numeric_locale: NumericLocaleSettings::default(),
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

#[derive(Clone, Copy, Debug, Default)]
struct ModeFlags {
    numeric: bool,
    general_numeric: bool,
    human_numeric: bool,
    month: bool,
    version: bool,
    random: bool,
}

impl ModeFlags {
    fn from_mode(mode: SortMode) -> Self {
        let mut flags = Self::default();
        match mode {
            SortMode::Numeric => flags.numeric = true,
            SortMode::GeneralNumeric => flags.general_numeric = true,
            SortMode::HumanNumeric => flags.human_numeric = true,
            SortMode::Month => flags.month = true,
            SortMode::Version => flags.version = true,
            SortMode::Random => flags.random = true,
            SortMode::Default => {}
        }
        flags
    }

    fn to_mode(self) -> SortMode {
        if self.numeric {
            SortMode::Numeric
        } else if self.general_numeric {
            SortMode::GeneralNumeric
        } else if self.human_numeric {
            SortMode::HumanNumeric
        } else if self.month {
            SortMode::Month
        } else if self.random {
            SortMode::Random
        } else if self.version {
            SortMode::Version
        } else {
            SortMode::Default
        }
    }
}

fn ordering_opts_string(
    flags: ModeFlags,
    dictionary_order: bool,
    ignore_non_printing: bool,
    ignore_case: bool,
) -> String {
    let mut opts = String::new();
    if dictionary_order {
        opts.push('d');
    }
    if ignore_case {
        opts.push('f');
    }
    if flags.general_numeric {
        opts.push('g');
    }
    if flags.human_numeric {
        opts.push('h');
    }
    if !dictionary_order && ignore_non_printing {
        opts.push('i');
    }
    if flags.month {
        opts.push('M');
    }
    if flags.numeric {
        opts.push('n');
    }
    if flags.random {
        opts.push('R');
    }
    if flags.version {
        opts.push('V');
    }
    opts
}

fn ordering_incompatible(
    flags: ModeFlags,
    dictionary_order: bool,
    ignore_non_printing: bool,
) -> bool {
    let mode_count = u8::from(flags.numeric)
        + u8::from(flags.general_numeric)
        + u8::from(flags.human_numeric)
        + u8::from(flags.month);

    // Multiple numeric/month modes are incompatible
    if mode_count > 1 {
        return true;
    }

    // A numeric/month mode combined with version/random/dictionary/ignore_non_printing is incompatible
    if mode_count == 1 {
        return flags.version || flags.random || dictionary_order || ignore_non_printing;
    }

    false
}

fn incompatible_options_error(opts: &str) -> Box<dyn UError> {
    USimpleError::new(
        2,
        translate!(
            "sort-options-incompatible",
            "opt1" => opts,
            "opt2" => ""
        ),
    )
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
        let needs_line_data = settings.precomputed.needs_tokens
            || settings.precomputed.selections_per_line > 0
            || settings.precomputed.num_infos_per_line > 0
            || settings.precomputed.floats_per_line > 0
            || settings.mode == SortMode::Numeric;
        if !needs_line_data {
            return Self { line, index };
        }
        token_buffer.clear();
        if settings.precomputed.needs_tokens {
            tokenize(
                line,
                settings.separator,
                token_buffer,
                &settings.precomputed,
            );
        }
        if settings.mode == SortMode::Numeric {
            // exclude inf, nan, scientific notation
            let line_num_float = (!line.iter().any(u8::is_ascii_alphabetic))
                .then(|| std::str::from_utf8(line).ok())
                .flatten()
                .and_then(|s| s.parse::<f64>().ok());
            line_data.line_num_floats.push(line_num_float);
        }
        for (selector, selection) in settings.selectors.iter().map(|selector| {
            (
                selector,
                selector.get_selection(line, token_buffer, settings.numeric_locale),
            )
        }) {
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
            self.write_debug(settings, writer)?;
        } else {
            writer.write_all(self.line)?;
            writer.write_all(&[settings.line_ending.into()])?;
        }
        Ok(())
    }

    /// Writes indicators for the selections this line matched. The original line content is NOT expected
    /// to be already printed.
    fn write_debug(
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
        tokenize(
            self.line,
            settings.separator,
            &mut fields,
            &settings.precomputed,
        );
        for selector in &settings.selectors {
            let mut selection = selector.get_range(self.line, Some(&fields));
            match selector.settings.mode {
                SortMode::Numeric | SortMode::HumanNumeric => {
                    // find out which range is used for numeric comparisons
                    let mut parse_settings = settings
                        .numeric_locale
                        .num_info_settings(selector.settings.mode == SortMode::HumanNumeric);
                    // Debug annotations should ignore thousands separators to match GNU output.
                    parse_settings.thousands_separator = None;
                    let (_, num_range) =
                        NumInfo::parse(&self.line[selection.clone()], &parse_settings);
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
                    let decimal_pt = locale_decimal_pt();
                    let leading = get_leading_gen(initial_selection, decimal_pt);

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

            // Don't let embedded NUL bytes influence column alignment in the
            // debug underline output, since they are often filtered out (e.g.
            // via `tr -d '\0'`) before inspection.
            let select = &line[..selection.start];
            let indent = count_non_null_bytes(select);
            write!(writer, "{}", " ".repeat(indent))?;

            if selection.is_empty() {
                writeln!(writer, "{}", translate!("sort-error-no-match-for-key"))?;
            } else {
                let select = &line[selection];
                let underline_len = count_non_null_bytes(select);
                writeln!(writer, "{}", "_".repeat(underline_len))?;
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
fn tokenize(
    line: &[u8],
    separator: Option<u8>,
    token_buffer: &mut Vec<Field>,
    precomputed: &Precomputed,
) {
    assert!(token_buffer.is_empty());
    if let Some(separator) = separator {
        tokenize_with_separator(line, separator, token_buffer);
    } else {
        tokenize_default(
            line,
            token_buffer,
            precomputed.tokenize_blank_thousands_sep,
            precomputed.tokenize_allow_unit_after_blank,
        );
    }
}

/// By default fields are separated by the first whitespace after non-whitespace.
/// Whitespace is included in fields at the start.
/// The result is stored into `token_buffer`.
fn tokenize_default(
    line: &[u8],
    token_buffer: &mut Vec<Field>,
    blank_thousands_sep: bool,
    allow_unit_after_blank: bool,
) {
    token_buffer.push(0..0);
    // pretend that there was whitespace in front of the line
    let mut previous_was_whitespace = true;
    for (idx, char) in line.iter().enumerate() {
        let is_whitespace = char.is_ascii_whitespace();
        let treat_as_separator = if is_whitespace {
            if blank_thousands_sep && *char == b' ' {
                !is_blank_thousands_sep(line, idx, allow_unit_after_blank)
            } else {
                true
            }
        } else {
            false
        };

        if treat_as_separator {
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

fn is_blank_thousands_sep(line: &[u8], idx: usize, allow_unit_after_blank: bool) -> bool {
    if line.get(idx) != Some(&b' ') {
        return false;
    }

    let prev_is_digit = idx
        .checked_sub(1)
        .and_then(|prev_idx| line.get(prev_idx))
        .is_some_and(u8::is_ascii_digit);
    if !prev_is_digit {
        return false;
    }

    let next = line.get(idx + 1).copied();
    match next {
        Some(c) if c.is_ascii_digit() => true,
        Some(b'K' | b'k' | b'M' | b'G' | b'T' | b'P' | b'E' | b'Z' | b'Y' | b'R' | b'Q')
            if allow_unit_after_blank =>
        {
            true
        }
        _ => false,
    }
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

impl Default for KeyPosition {
    fn default() -> Self {
        Self {
            field: 1,
            char: 1,
            ignore_blanks: false,
        }
    }
}

fn bad_field_spec(spec: &str, msg_key: &str) -> Box<dyn UError> {
    USimpleError::new(
        2,
        translate!(
            "sort-invalid-field-spec",
            "msg" => translate!(msg_key),
            "spec" => spec.quote()
        ),
    )
}

fn invalid_count_error(msg_key: &str, input: &str) -> Box<dyn UError> {
    USimpleError::new(
        2,
        format!(
            "{}: {}",
            translate!(msg_key),
            translate!("sort-invalid-count-at-start-of", "string" => input.quote())
        ),
    )
}

fn parse_field_count<'a>(input: &'a str, msg_key: &str) -> UResult<(usize, &'a str)> {
    let bytes = input.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == 0 {
        return Err(invalid_count_error(msg_key, input));
    }
    let (num_str, rest) = input.split_at(idx);
    let value = match num_str.parse::<usize>() {
        Ok(v) => v,
        Err(e) if *e.kind() == IntErrorKind::PosOverflow => usize::MAX,
        Err(_) => return Err(invalid_count_error(msg_key, input)),
    };
    Ok((value, rest))
}

fn is_ordering_option_char(byte: u8) -> bool {
    matches!(
        byte,
        b'b' | b'd' | b'f' | b'g' | b'h' | b'i' | b'M' | b'n' | b'R' | b'r' | b'V'
    )
}

fn parse_ordering_options<'a>(
    input: &'a str,
    settings: &mut KeySettings,
    flags: &mut ModeFlags,
) -> (&'a str, bool) {
    let mut ignore_blanks = false;
    let bytes = input.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            b'b' => ignore_blanks = true,
            b'd' => {
                settings.dictionary_order = true;
                settings.ignore_non_printing = false;
            }
            b'f' => settings.ignore_case = true,
            b'g' => flags.general_numeric = true,
            b'h' => flags.human_numeric = true,
            b'i' => {
                if !settings.dictionary_order {
                    settings.ignore_non_printing = true;
                }
            }
            b'M' => flags.month = true,
            b'n' => flags.numeric = true,
            b'R' => flags.random = true,
            b'r' => settings.reverse = true,
            b'V' => flags.version = true,
            _ => break,
        }
        idx += 1;
    }
    (&input[idx..], ignore_blanks)
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
    fn parse(key: &str, global_settings: &GlobalSettings) -> UResult<Self> {
        let has_options = key.as_bytes().iter().copied().any(is_ordering_option_char);
        let mut settings = if has_options {
            KeySettings::default()
        } else {
            KeySettings::from(global_settings)
        };
        let mut flags = if has_options {
            ModeFlags::default()
        } else {
            ModeFlags::from_mode(settings.mode)
        };

        let mut from_ignore_blanks = if has_options {
            false
        } else {
            settings.ignore_blanks
        };
        let mut to_ignore_blanks = if has_options {
            false
        } else {
            settings.ignore_blanks
        };

        let (from_field, mut rest) = parse_field_count(key, "sort-invalid-number-at-field-start")?;
        if from_field == 0 {
            return Err(bad_field_spec(key, "sort-field-number-is-zero"));
        }

        let mut from_char = 1;
        if let Some(stripped) = rest.strip_prefix('.') {
            let (char_idx, rest_after) =
                parse_field_count(stripped, "sort-invalid-number-after-dot")?;
            if char_idx == 0 {
                return Err(bad_field_spec(key, "sort-character-offset-is-zero"));
            }
            from_char = char_idx;
            rest = rest_after;
        }

        let (rest_after_opts, ignore_blanks) =
            parse_ordering_options(rest, &mut settings, &mut flags);
        if ignore_blanks {
            from_ignore_blanks = true;
        }

        let mut to = None;
        if let Some(rest_after_comma) = rest_after_opts.strip_prefix(',') {
            let (to_field, mut rest) =
                parse_field_count(rest_after_comma, "sort-invalid-number-after-comma")?;
            if to_field == 0 {
                return Err(bad_field_spec(key, "sort-field-number-is-zero"));
            }

            let mut to_char = 0;
            if let Some(stripped) = rest.strip_prefix('.') {
                let (char_idx, rest_after) =
                    parse_field_count(stripped, "sort-invalid-number-after-dot")?;
                to_char = char_idx;
                rest = rest_after;
            }

            let (rest, ignore_blanks_end) = parse_ordering_options(rest, &mut settings, &mut flags);
            if ignore_blanks_end {
                to_ignore_blanks = true;
            }
            if !rest.is_empty() {
                return Err(bad_field_spec(key, "sort-stray-character-field-spec"));
            }
            to = Some(KeyPosition {
                field: to_field,
                char: to_char,
                ignore_blanks: to_ignore_blanks,
            });
        } else if !rest_after_opts.is_empty() {
            return Err(bad_field_spec(key, "sort-stray-character-field-spec"));
        }

        if ordering_incompatible(
            flags,
            settings.dictionary_order,
            settings.ignore_non_printing,
        ) {
            let opts = ordering_opts_string(
                flags,
                settings.dictionary_order,
                settings.ignore_non_printing,
                settings.ignore_case,
            );
            return Err(incompatible_options_error(&opts));
        }

        settings.mode = flags.to_mode();

        let from = KeyPosition {
            field: from_field,
            char: from_char,
            ignore_blanks: from_ignore_blanks,
        };
        Self::new(from, to, settings).map_err(|msg| USimpleError::new(2, msg))
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
    fn get_selection<'a>(
        &self,
        line: &'a [u8],
        tokens: &[Field],
        numeric_locale: NumericLocaleSettings,
    ) -> Selection<'a> {
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
                &numeric_locale.num_info_settings(self.settings.mode == SortMode::HumanNumeric),
            );
            // Shorten the range to what we need to pass to numeric_str_cmp later.
            range_str = &range_str[num_range];
            Selection::WithNumInfo(range_str, info)
        } else if self.settings.mode == SortMode::GeneralNumeric {
            // Parse this number as BigDecimal, as this is the requirement for general numeric sorting.
            let decimal_pt = locale_decimal_pt();
            Selection::AsBigDecimal(general_bd_parse(
                &range_str[get_leading_gen(range_str, decimal_pt)],
                decimal_pt,
            ))
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

fn detect_numeric_locale() -> NumericLocaleSettings {
    let numeric_locale = i18n::get_numeric_locale();
    let locale = &numeric_locale.0;
    let encoding = numeric_locale.1;
    let is_c_locale = encoding == i18n::UEncoding::Ascii && locale.to_string() == "und";

    if is_c_locale {
        return NumericLocaleSettings {
            decimal_pt: Some(DECIMAL_PT),
            thousands_sep: None,
        };
    }

    let grouping = i18n::decimal::locale_grouping_separator();
    NumericLocaleSettings {
        decimal_pt: Some(locale_decimal_pt()),
        // Upstream GNU coreutils ignore multibyte thousands separators
        // (FIXME in C source). We keep the same single-byte behavior.
        thousands_sep: match grouping.as_bytes() {
            [b] => Some(*b),
            // ICU returns NBSP as UTF-8 (0xC2 0xA0). In non-UTF8 locales like ISO-8859-1,
            // the input byte is 0xA0, so map it to a single-byte separator.
            [0xC2, 0xA0] if encoding != i18n::UEncoding::Utf8 => Some(0xA0),
            _ => None,
        },
    }
}
/// Creates an `Arg` for a sort mode flag.
fn make_sort_mode_arg(mode: &'static str, short: char, help: String) -> Arg {
    Arg::new(mode)
        .short(short)
        .long(mode)
        .help(help)
        .action(ArgAction::SetTrue)
}

#[cfg(all(
    unix,
    not(any(
        target_os = "redox",
        target_os = "fuchsia",
        target_os = "haiku",
        target_os = "solaris",
        target_os = "illumos"
    ))
))]
fn get_rlimit() -> UResult<usize> {
    use nix::sys::resource::{RLIM_INFINITY, Resource, getrlimit};

    let (rlim_cur, _rlim_max) = getrlimit(Resource::RLIMIT_NOFILE)
        .map_err(|_| UUsageError::new(2, translate!("sort-failed-fetch-rlimit")))?;
    if rlim_cur == RLIM_INFINITY {
        return Err(UUsageError::new(2, translate!("sort-failed-fetch-rlimit")));
    }
    usize::try_from(rlim_cur)
        .map_err(|_| UUsageError::new(2, translate!("sort-failed-fetch-rlimit")))
}

#[cfg(all(
    unix,
    not(any(
        target_os = "redox",
        target_os = "fuchsia",
        target_os = "haiku",
        target_os = "solaris",
        target_os = "illumos"
    ))
))]
pub(crate) fn fd_soft_limit() -> Option<usize> {
    get_rlimit().ok()
}

#[cfg(any(
    not(unix),
    target_os = "redox",
    target_os = "fuchsia",
    target_os = "haiku",
    target_os = "solaris",
    target_os = "illumos"
))]
pub(crate) fn fd_soft_limit() -> Option<usize> {
    None
}

#[cfg(unix)]
pub(crate) fn current_open_fd_count() -> Option<usize> {
    use nix::libc;

    fn count_dir(path: &str) -> Option<usize> {
        let entries = std::fs::read_dir(path).ok()?;
        let mut count = 0usize;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.parse::<usize>().is_ok() {
                count = count.saturating_add(1);
            }
        }
        Some(count)
    }

    if let Some(count) = count_dir("/proc/self/fd").or_else(|| count_dir("/dev/fd")) {
        return Some(count);
    }

    let limit = fd_soft_limit()?;
    if limit > 16_384 {
        return None;
    }

    let mut count = 0usize;
    for fd in 0..limit {
        let fd = fd as libc::c_int;
        // Probe with libc::fcntl because the fd may be invalid.
        if unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1 {
            count = count.saturating_add(1);
        }
    }
    Some(count)
}

#[cfg(not(unix))]
pub(crate) fn current_open_fd_count() -> Option<usize> {
    None
}

const STDIN_FILE: &str = "-";

/// Legacy `+POS1 [-POS2]` syntax is permitted unless `_POSIX2_VERSION` is in
/// the [TRADITIONAL, MODERN) range (matches GNU behaviour).
fn allows_traditional_usage() -> bool {
    !matches!(uucore::posix::posix_version(), Some(ver) if (TRADITIONAL..MODERN).contains(&ver))
}

#[derive(Debug, Clone)]
struct LegacyKeyPart {
    field: usize,
    char_pos: usize,
    opts: String,
}

#[derive(Debug, Clone)]
struct LegacyKeyWarning {
    arg_index: usize,
    key_index: Option<usize>,
    from_field: usize,
    to_field: Option<usize>,
    to_char: Option<usize>,
}

impl LegacyKeyWarning {
    fn legacy_key_display(&self) -> String {
        match self.to_field {
            Some(to) => format!("+{} -{to}", self.from_field),
            None => format!("+{}", self.from_field),
        }
    }

    fn replacement_key_display(&self) -> String {
        let start_field = self.from_field.saturating_add(1);
        match self.to_field {
            Some(to_field) => {
                let end_field = match self.to_char {
                    Some(0) | None => to_field.max(1),
                    Some(_) => to_field.saturating_add(1),
                };
                format!("{start_field},{end_field}")
            }
            None => start_field.to_string(),
        }
    }
}

#[derive(Default)]
struct GlobalOptionFlags {
    keys_specified: bool,
    ignore_leading_blanks: bool,
    dictionary_order: bool,
    ignore_case: bool,
    ignore_non_printing: bool,
    reverse: bool,
    mode_numeric: bool,
    mode_general: bool,
    mode_human: bool,
    mode_month: bool,
    mode_random: bool,
    mode_version: bool,
}

impl GlobalOptionFlags {
    fn from_matches(matches: &ArgMatches) -> Self {
        let sort_value = matches
            .get_one::<String>(options::modes::SORT)
            .map(String::as_str);
        Self {
            keys_specified: matches.contains_id(options::KEY),
            ignore_leading_blanks: matches.get_flag(options::IGNORE_LEADING_BLANKS),
            dictionary_order: matches.get_flag(options::DICTIONARY_ORDER),
            ignore_case: matches.get_flag(options::IGNORE_CASE),
            ignore_non_printing: matches.get_flag(options::IGNORE_NONPRINTING),
            reverse: matches.get_flag(options::REVERSE),
            mode_human: matches.get_flag(options::modes::HUMAN_NUMERIC)
                || sort_value == Some("human-numeric"),
            mode_month: matches.get_flag(options::modes::MONTH) || sort_value == Some("month"),
            mode_general: matches.get_flag(options::modes::GENERAL_NUMERIC)
                || sort_value == Some("general-numeric"),
            mode_numeric: matches.get_flag(options::modes::NUMERIC)
                || sort_value == Some("numeric"),
            mode_version: matches.get_flag(options::modes::VERSION)
                || sort_value == Some("version"),
            mode_random: matches.get_flag(options::modes::RANDOM) || sort_value == Some("random"),
        }
    }
}

fn parse_usize_or_max(num: &str) -> Option<usize> {
    match num.parse::<usize>() {
        Ok(v) => Some(v),
        Err(e) if *e.kind() == IntErrorKind::PosOverflow => Some(usize::MAX),
        Err(_) => None,
    }
}

fn parse_legacy_part(spec: &str) -> Option<LegacyKeyPart> {
    let idx = spec.chars().take_while(char::is_ascii_digit).count();
    if idx == 0 {
        return None;
    }

    let field = parse_usize_or_max(&spec[..idx])?;
    let mut char_pos = 0;
    let mut rest = &spec[idx..];

    if let Some(stripped) = rest.strip_prefix('.') {
        let char_idx = stripped.chars().take_while(char::is_ascii_digit).count();
        if char_idx == 0 {
            return None;
        }
        char_pos = parse_usize_or_max(&stripped[..char_idx])?;
        rest = &stripped[char_idx..];
    }

    Some(LegacyKeyPart {
        field,
        char_pos,
        opts: rest.to_string(),
    })
}

/// Convert legacy +POS1 [-POS2] into a `-k` key specification using saturating arithmetic.
fn legacy_key_to_k(from: &LegacyKeyPart, to: Option<&LegacyKeyPart>) -> String {
    let start_field = from.field.saturating_add(1);
    let start_char = from.char_pos.saturating_add(1);

    let mut keydef = format!(
        "{start_field}{}{}",
        if from.char_pos == 0 {
            String::new()
        } else {
            format!(".{start_char}")
        },
        from.opts,
    );

    if let Some(to) = to {
        let end_field = if to.char_pos == 0 {
            // When the end character index is zero, GNU keeps the field number as-is.
            // Clamp to 1 to avoid generating an invalid field 0.
            to.field.max(1)
        } else {
            to.field.saturating_add(1)
        };

        keydef.push(',');
        keydef.push_str(&end_field.to_string());
        if to.char_pos != 0 {
            keydef.push('.');
            keydef.push_str(&to.char_pos.to_string());
        }
        keydef.push_str(&to.opts);
    }

    keydef
}

/// Preprocess argv to handle legacy +POS1 [-POS2] syntax by converting it into -k forms
/// before clap sees the arguments.
fn preprocess_legacy_args<I>(args: I) -> (Vec<OsString>, Vec<LegacyKeyWarning>)
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    if !allows_traditional_usage() {
        return (args.into_iter().map(Into::into).collect(), Vec::new());
    }

    let mut processed = Vec::new();
    let mut legacy_warnings = Vec::new();
    let mut iter = args.into_iter().map(Into::into).peekable();

    while let Some(arg) = iter.next() {
        if arg == "--" {
            processed.push(arg);
            processed.extend(iter);
            break;
        }

        if starts_with_plus(&arg) {
            let as_str = arg.to_string_lossy();
            if let Some(from_spec) = as_str.strip_prefix('+') {
                if let Some(from) = parse_legacy_part(from_spec) {
                    let mut to_part = None;

                    let next_candidate = iter.peek().map(|next| next.to_string_lossy().to_string());

                    if let Some(next_str) = next_candidate {
                        if let Some(stripped) = next_str.strip_prefix('-') {
                            if stripped.starts_with(|c: char| c.is_ascii_digit()) {
                                let next_arg = iter.next().unwrap();
                                if let Some(parsed) = parse_legacy_part(stripped) {
                                    to_part = Some(parsed);
                                } else {
                                    processed.push(arg);
                                    processed.push(next_arg);
                                    continue;
                                }
                            }
                        }
                    }

                    let keydef = legacy_key_to_k(&from, to_part.as_ref());
                    let arg_index = processed.len();
                    legacy_warnings.push(LegacyKeyWarning {
                        arg_index,
                        key_index: None,
                        from_field: from.field,
                        to_field: to_part.as_ref().map(|p| p.field),
                        to_char: to_part.as_ref().map(|p| p.char_pos),
                    });
                    processed.push(OsString::from(format!("-k{keydef}")));
                    continue;
                }
            }
        }

        processed.push(arg);
    }

    (processed, legacy_warnings)
}

fn starts_with_plus(arg: &OsStr) -> bool {
    #[cfg(unix)]
    {
        arg.as_bytes().first() == Some(&b'+')
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().starts_with('+')
    }
}

fn index_legacy_warnings(processed_args: &[OsString], legacy_warnings: &mut [LegacyKeyWarning]) {
    if legacy_warnings.is_empty() {
        return;
    }

    let mut index_by_arg = AHashMap::default();
    for (warning_idx, warning) in legacy_warnings.iter().enumerate() {
        index_by_arg.insert(warning.arg_index, warning_idx);
    }

    let mut key_index = 0usize;
    let mut i = 0usize;
    while i < processed_args.len() {
        let arg = &processed_args[i];
        if arg == OsStr::new("--") {
            break;
        }

        let mut matched_key = false;
        if arg == OsStr::new("-k") || arg == OsStr::new("--key") {
            if i + 1 < processed_args.len() {
                key_index = key_index.saturating_add(1);
                matched_key = true;
                i += 2;
            } else {
                i += 1;
            }
        } else {
            let as_str = arg.to_string_lossy();
            if let Some(spec) = as_str.strip_prefix("-k") {
                if !spec.is_empty() {
                    key_index = key_index.saturating_add(1);
                    matched_key = true;
                }
            } else if let Some(spec) = as_str.strip_prefix("--key=") {
                if !spec.is_empty() {
                    key_index = key_index.saturating_add(1);
                    matched_key = true;
                }
            }
            i += 1;
        }

        if matched_key {
            if let Some(&warning_idx) = index_by_arg.get(&i.saturating_sub(1)) {
                legacy_warnings[warning_idx].key_index = Some(key_index);
            }
        }
    }
}

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
        match fd_soft_limit() {
            Some(limit) => {
                let usable_limit = limit.saturating_div(LINUX_BATCH_DIVISOR);
                usable_limit.clamp(LINUX_BATCH_MIN, LINUX_BATCH_MAX)
            }
            None => 64,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        64
    }
}

#[cfg(not(unix))]
fn locale_failed_to_set() -> bool {
    matches!(env::var("LC_ALL").ok().as_deref(), Some("missing"))
}

#[cfg(unix)]
fn locale_failed_to_set() -> bool {
    use nix::libc;
    unsafe { libc::setlocale(libc::LC_COLLATE, c"".as_ptr()).is_null() }
}

fn key_zero_width(selector: &FieldSelector) -> bool {
    let Some(to) = &selector.to else {
        return false;
    };
    if to.field < selector.from.field {
        return true;
    }
    if to.field == selector.from.field {
        return to.char != 0 && to.char < selector.from.char;
    }
    false
}

fn key_spans_multiple_fields(selector: &FieldSelector) -> bool {
    if !matches!(
        selector.settings.mode,
        SortMode::Numeric | SortMode::HumanNumeric | SortMode::GeneralNumeric
    ) {
        return false;
    }
    match &selector.to {
        None => true,
        Some(to) => to.field > selector.from.field,
    }
}

fn key_leading_blanks_significant(selector: &FieldSelector) -> bool {
    selector.settings.mode == SortMode::Default
        && !selector.from.ignore_blanks
        && !selector.settings.ignore_blanks
}

fn emit_debug_warnings(
    settings: &GlobalSettings,
    flags: &GlobalOptionFlags,
    legacy_warnings: &[LegacyKeyWarning],
) {
    if locale_failed_to_set() {
        show_error!("{}", translate!("sort-warning-failed-to-set-locale"));
    }

    let (locale, encoding) = i18n::get_collating_locale();

    if matches!(encoding, i18n::UEncoding::Utf8) {
        let locale_as_posix = format!("{}.UTF-8", locale.to_string().replace('-', "_"));
        show_error!(
            "{}",
            translate!("sort-warning-sort-rule", "locale" => locale_as_posix)
        );
    } else {
        show_error!("{}", translate!("sort-warning-simple-byte-comparison"));
    }

    for (key_index, selector) in (1..).zip(settings.selectors.iter()) {
        if let Some(legacy) = legacy_warnings
            .iter()
            .find(|warning| warning.key_index == Some(key_index))
        {
            show_error!(
                "{}",
                translate!(
                    "sort-warning-obsolescent-key",
                    "key" => legacy.legacy_key_display(),
                    "replacement" => legacy.replacement_key_display()
                )
            );
        }

        if key_zero_width(selector) {
            show_error!(
                "{}",
                translate!("sort-warning-key-zero-width", "key" => key_index)
            );
            continue;
        }

        if flags.keys_specified && key_spans_multiple_fields(selector) {
            show_error!(
                "{}",
                translate!(
                    "sort-warning-key-numeric-spans-fields",
                    "key" => key_index
                )
            );
        } else if flags.keys_specified && key_leading_blanks_significant(selector) {
            show_error!(
                "{}",
                translate!(
                    "sort-warning-leading-blanks-significant",
                    "key" => key_index
                )
            );
        }
    }

    let numeric_used = settings.selectors.iter().any(|selector| {
        matches!(
            selector.settings.mode,
            SortMode::Numeric | SortMode::HumanNumeric | SortMode::GeneralNumeric
        )
    });

    let mut suppress_decimal_warning = false;
    if numeric_used {
        if let Some(sep) = settings.separator {
            match sep {
                b'.' => {
                    show_error!(
                        "{}",
                        translate!("sort-warning-separator-decimal", "sep" => ".")
                    );
                    suppress_decimal_warning = true;
                }
                b'-' => {
                    show_error!(
                        "{}",
                        translate!("sort-warning-separator-minus", "sep" => "-")
                    );
                }
                b'+' => {
                    show_error!(
                        "{}",
                        translate!("sort-warning-separator-plus", "sep" => "+")
                    );
                }
                _ => {}
            }
        }

        if !suppress_decimal_warning {
            show_error!("{}", translate!("sort-warning-numbers-use-decimal-point"));
        }
    }

    let uses_reverse = settings
        .selectors
        .iter()
        .any(|selector| selector.settings.reverse);
    let uses_blanks = settings
        .selectors
        .iter()
        .any(|selector| selector.settings.ignore_blanks || selector.from.ignore_blanks);
    let uses_dictionary = settings
        .selectors
        .iter()
        .any(|selector| selector.settings.dictionary_order);
    let uses_case = settings
        .selectors
        .iter()
        .any(|selector| selector.settings.ignore_case);
    let uses_non_printing = settings
        .selectors
        .iter()
        .any(|selector| selector.settings.ignore_non_printing);

    let uses_mode = |mode| {
        settings
            .selectors
            .iter()
            .any(|selector| selector.settings.mode == mode)
    };

    let reverse_unused = flags.reverse && !uses_reverse;
    let last_resort_active =
        settings.mode != SortMode::Random && !settings.stable && !settings.unique;
    let reverse_ignored = reverse_unused && !last_resort_active;
    let reverse_last_resort_warning = reverse_unused && last_resort_active;

    let mut ignored_opts = String::new();
    if flags.ignore_leading_blanks && !uses_blanks {
        ignored_opts.push('b');
    }
    if flags.dictionary_order && !uses_dictionary {
        ignored_opts.push('d');
    }
    if flags.ignore_case && !uses_case {
        ignored_opts.push('f');
    }
    if flags.ignore_non_printing && !uses_non_printing {
        ignored_opts.push('i');
    }
    if flags.mode_general && !uses_mode(SortMode::GeneralNumeric) {
        ignored_opts.push('g');
    }
    if flags.mode_human && !uses_mode(SortMode::HumanNumeric) {
        ignored_opts.push('h');
    }
    if flags.mode_month && !uses_mode(SortMode::Month) {
        ignored_opts.push('M');
    }
    if flags.mode_numeric && !uses_mode(SortMode::Numeric) {
        ignored_opts.push('n');
    }
    if flags.mode_random && !uses_mode(SortMode::Random) {
        ignored_opts.push('R');
    }
    if reverse_ignored {
        ignored_opts.push('r');
    }
    if flags.mode_version && !uses_mode(SortMode::Version) {
        ignored_opts.push('V');
    }

    if ignored_opts.len() == 1 {
        show_error!(
            "{}",
            translate!("sort-warning-option-ignored", "option" => ignored_opts)
        );
    } else if ignored_opts.len() > 1 {
        show_error!(
            "{}",
            translate!("sort-warning-options-ignored", "options" => ignored_opts)
        );
    }

    if reverse_last_resort_warning {
        show_error!("{}", translate!("sort-warning-option-reverse-last-resort"));
    }
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut settings = GlobalSettings {
        numeric_locale: detect_numeric_locale(),
        ..Default::default()
    };

    let (processed_args, mut legacy_warnings) = preprocess_legacy_args(args);
    if !legacy_warnings.is_empty() {
        index_legacy_warnings(&processed_args, &mut legacy_warnings);
    }
    let matches =
        uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), processed_args, 2)?;

    // Prevent -o/--output to be specified multiple times
    if let Some(mut outputs) = matches.get_many::<OsString>(options::OUTPUT) {
        if let Some(first) = outputs.next() {
            if outputs.any(|out| out != first) {
                return Err(SortError::MultipleOutputFiles.into());
            }
        }
    }

    settings.debug = matches.get_flag(options::DEBUG);
    if let Some(path) = matches.get_one::<OsString>(options::RANDOM_SOURCE) {
        settings.random_source = Some(PathBuf::from(path));
    }

    // check whether user specified a zero terminated list of files for input, otherwise read files from args
    let mut files: Vec<OsString> = if matches.contains_id(options::FILES0_FROM) {
        let files0_from: PathBuf = matches
            .get_one::<OsString>(options::FILES0_FROM)
            .map(Into::into)
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

    let mut mode_flags = ModeFlags {
        human_numeric: matches.get_flag(options::modes::HUMAN_NUMERIC),
        month: matches.get_flag(options::modes::MONTH),
        general_numeric: matches.get_flag(options::modes::GENERAL_NUMERIC),
        numeric: matches.get_flag(options::modes::NUMERIC),
        version: matches.get_flag(options::modes::VERSION),
        random: matches.get_flag(options::modes::RANDOM),
    };
    if let Some(sort_arg) = matches.get_one::<String>(options::modes::SORT) {
        match sort_arg.as_str() {
            "human-numeric" => mode_flags.human_numeric = true,
            "month" => mode_flags.month = true,
            "general-numeric" => mode_flags.general_numeric = true,
            "numeric" => mode_flags.numeric = true,
            "version" => mode_flags.version = true,
            "random" => mode_flags.random = true,
            _ => {}
        }
    }

    let dictionary_order = matches.get_flag(options::DICTIONARY_ORDER);
    let ignore_non_printing = matches.get_flag(options::IGNORE_NONPRINTING);
    let ignore_case = matches.get_flag(options::IGNORE_CASE);

    if !matches.contains_id(options::KEY)
        && ordering_incompatible(mode_flags, dictionary_order, ignore_non_printing)
    {
        let opts = ordering_opts_string(
            mode_flags,
            dictionary_order,
            ignore_non_printing,
            ignore_case,
        );
        return Err(incompatible_options_error(&opts));
    }

    settings.mode = mode_flags.to_mode();
    if mode_flags.random {
        settings.salt = Some(get_rand_string());
    }

    settings.dictionary_order = dictionary_order;
    settings.ignore_non_printing = ignore_non_printing;
    settings.ignore_case = ignore_case;
    if matches.contains_id(options::PARALLEL) {
        // "0" is default - threads = num of cores
        settings.threads = matches
            .get_one::<String>(options::PARALLEL)
            .map_or_else(|| "0".to_string(), String::from);
        let num_threads = match settings.threads.parse::<usize>() {
            Ok(0) | Err(_) => std::thread::available_parallelism().map_or(1, NonZero::get),
            Ok(n) => n,
        };
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global();
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
                        show_error!("{batch_too_large}");

                        translate!(
                            "sort-maximum-batch-size-rlimit",
                            "rlimit" => {
                                let Some(rlimit) = fd_soft_limit() else {
                                    return Err(UUsageError::new(
                                        2,
                                        translate!("sort-failed-fetch-rlimit"),
                                    ));
                                };
                                rlimit
                            }
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
    if settings.check && matches.get_flag(options::check::CHECK_SILENT) {
        return Err(incompatible_options_error("cC"));
    }
    if matches.get_flag(options::check::CHECK_SILENT)
        || matches!(
            matches
                .get_one::<String>(options::check::CHECK)
                .map(String::as_str),
            Some(options::check::SILENT | options::check::QUIET)
        )
    {
        settings.check_silent = true;
        settings.check = true;
    }

    if matches.contains_id(options::OUTPUT) && settings.check {
        let opts = if settings.check_silent { "Co" } else { "co" };
        return Err(incompatible_options_error(opts));
    }

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

    let needs_random = settings.mode == SortMode::Random
        || settings
            .selectors
            .iter()
            .any(|selector| selector.settings.mode == SortMode::Random);
    if needs_random {
        settings.salt = Some(match settings.random_source.as_deref() {
            Some(path) => salt_from_random_source(path)?,
            None => get_rand_string(),
        });
    }

    // Verify that we can open all input files.
    // It is the correct behavior to close all files afterwards,
    // and to reopen them at a later point. This is different from how the output file is handled,
    // probably to prevent running out of file descriptors.
    for file in &files {
        open(file)?;
    }

    let output = Output::new(matches.get_one::<OsString>(options::OUTPUT))?;

    if settings.debug {
        let global_flags = GlobalOptionFlags::from_matches(&matches);
        emit_debug_warnings(&settings, &global_flags, &legacy_warnings);
    }

    // Initialize locale collation if needed (UTF-8 locales)
    // This MUST happen before init_precomputed() to avoid the performance regression
    #[cfg(feature = "i18n-collator")]
    let needs_locale_collation = i18n::collator::init_locale_collation();

    #[cfg(not(feature = "i18n-collator"))]
    let needs_locale_collation = false;

    settings.init_precomputed(needs_locale_collation);

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
            ])),
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
        Arg::new(options::RANDOM_SOURCE)
            .long(options::RANDOM_SOURCE)
            .help(translate!("sort-help-random-source"))
            .value_name("FILE")
            .value_parser(ValueParser::os_string())
            .value_hint(clap::ValueHint::FilePath),
    )
    .arg(
        Arg::new(options::DICTIONARY_ORDER)
            .short('d')
            .long(options::DICTIONARY_ORDER)
            .help(translate!("sort-help-dictionary-order"))
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
            .help(translate!("sort-help-check")),
    )
    .arg(
        Arg::new(options::check::CHECK_SILENT)
            .short('C')
            .long(options::check::CHECK_SILENT)
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
            SortMode::Default => {
                // Use locale-aware comparison if feature is enabled and no custom flags are set
                #[cfg(feature = "i18n-collator")]
                {
                    if settings.ignore_case
                        || settings.dictionary_order
                        || settings.ignore_non_printing
                    {
                        custom_str_cmp(
                            a_str,
                            b_str,
                            settings.ignore_non_printing,
                            settings.dictionary_order,
                            settings.ignore_case,
                        )
                    } else {
                        locale_cmp(a_str, b_str)
                    }
                }
                #[cfg(not(feature = "i18n-collator"))]
                {
                    custom_str_cmp(
                        a_str,
                        b_str,
                        settings.ignore_non_printing,
                        settings.dictionary_order,
                        settings.ignore_case,
                    )
                }
            }
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
/// We upper each byte on the fly so that binary input (including `NUL`) stays
/// untouched and we avoid locale-sensitive routines such as `strcasecmp`.
fn ascii_case_insensitive_cmp(a: &[u8], b: &[u8]) -> Ordering {
    #[inline]
    fn fold(byte: u8) -> u8 {
        byte.to_ascii_uppercase()
    }

    for (lhs, rhs) in a.iter().copied().zip(b.iter().copied()) {
        let l = fold(lhs);
        let r = fold(rhs);
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
fn get_leading_gen(inp: &[u8], decimal_pt: u8) -> Range<usize> {
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

        if c == decimal_pt && !had_decimal_pt && !had_e_notation {
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
                        char_indices.peek_nth(1),
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
fn general_bd_parse(a: &[u8], decimal_pt: u8) -> GeneralBigDecimalParseResult {
    let parsed_bytes = (decimal_pt != DECIMAL_PT).then(|| {
        a.iter()
            .map(|&b| if b == decimal_pt { DECIMAL_PT } else { b })
            .collect::<Vec<_>>()
    });
    let input = parsed_bytes.as_deref().unwrap_or(a);

    // The string should be valid ASCII to be parsed.
    let Ok(a) = std::str::from_utf8(input) else {
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

/// Generate a 128-bit salt from a uniform RNG distribution.
fn get_rand_string() -> [u8; SALT_LEN] {
    rng().sample(rand::distr::StandardUniform)
}

const SALT_LEN: usize = 16; // 128-bit salt
const MAX_BYTES: usize = 1024 * 1024; // Read cap: 1 MiB
const BUF_LEN: usize = 8192; // 8 KiB read buffer
const U64_LEN: usize = 8;
const RANDOM_SOURCE_TAG: &[u8] = b"uutils-sort-random-source"; // Domain separation tag

/// Create a 128-bit salt by hashing up to 1 MiB from the given file.
fn salt_from_random_source(path: &Path) -> UResult<[u8; SALT_LEN]> {
    let mut reader = open_with_open_failed_error(path)?;
    let mut buf = [0u8; BUF_LEN];
    let mut total = 0usize;
    // freeze seed for --random-source
    let mut hasher = ahash::RandomState::with_seeds(1, 1, 1, 1).build_hasher();

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|error| SortError::ReadFailed {
                path: path.to_owned(),
                error,
            })?;
        if n == 0 {
            break;
        }
        let remaining = MAX_BYTES.saturating_sub(total);
        if remaining == 0 {
            break;
        }
        let take = n.min(remaining);
        hasher.write(&buf[..take]);
        total = total.saturating_add(take);
        if take < n {
            break;
        }
    }

    let first = hasher.finish();
    // freeze seed for --random-source
    let mut second_hasher = ahash::RandomState::with_seeds(2, 2, 2, 2).build_hasher();
    second_hasher.write(RANDOM_SOURCE_TAG);
    second_hasher.write_u64(first);
    let second = second_hasher.finish();

    let mut out = [0u8; SALT_LEN];
    out[..U64_LEN].copy_from_slice(&first.to_le_bytes());
    out[U64_LEN..].copy_from_slice(&second.to_le_bytes());
    Ok(out)
}

fn get_hash<T: Hash>(t: &T) -> u64 {
    // Is reproducibility of get_hash itself needed for --random-source ?
    ahash::RandomState::with_seeds(0, 0, 0, 0).hash_one(t)
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

    match line.get(..3).map(<[u8]>::to_ascii_uppercase).as_deref() {
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
        let precomputed = Precomputed::default();
        tokenize(line, separator, &mut buffer, &precomputed);
        buffer
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
