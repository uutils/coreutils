//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Yin <mikeyin@mikeyin.org>
//  * (c) Robert Swinford <robert.swinford..AT..gmail.com>
//  * (c) Michael Debertol <michael.debertol..AT..gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// Although these links don't always seem to describe reality, check out the POSIX and GNU specs:
// https://pubs.opengroup.org/onlinepubs/9699919799/utilities/sort.html
// https://www.gnu.org/software/coreutils/manual/html_node/sort-invocation.html

// spell-checker:ignore (ToDO) outfile nondictionary
#[macro_use]
extern crate uucore;

mod external_sort;
mod numeric_str_cmp;

use clap::{App, Arg};
use external_sort::{ExternalSorter, ExternallySortable};
use fnv::FnvHasher;
use itertools::Itertools;
use numeric_str_cmp::{numeric_str_cmp, NumInfo, NumInfoParseSettings};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use semver::Version;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Lines, Read, Write};
use std::mem::replace;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;
use unicode_width::UnicodeWidthStr;
use uucore::fs::is_stdin_interactive; // for Iterator::dedup()

static NAME: &str = "sort";
static ABOUT: &str = "Display sorted concatenation of all FILE(s).";
static VERSION: &str = env!("CARGO_PKG_VERSION");

const LONG_HELP_KEYS: &str = "The key format is FIELD[.CHAR][OPTIONS][,FIELD[.CHAR]][OPTIONS].

Fields by default are separated by the first whitespace after a non-whitespace character. Use -t to specify a custom separator.
In the default case, whitespace is appended at the beginning of each field. Custom separators however are not included in fields.

FIELD and CHAR both start at 1 (i.e. they are 1-indexed). If there is no end specified after a comma, the end will be the end of the line.
If CHAR is set 0, it means the end of the field. CHAR defaults to 1 for the start position and to 0 for the end position.

Valid options are: MbdfhnRrV. They override the global options for this key.";

static OPT_HUMAN_NUMERIC_SORT: &str = "human-numeric-sort";
static OPT_MONTH_SORT: &str = "month-sort";
static OPT_NUMERIC_SORT: &str = "numeric-sort";
static OPT_GENERAL_NUMERIC_SORT: &str = "general-numeric-sort";
static OPT_VERSION_SORT: &str = "version-sort";

static OPT_DICTIONARY_ORDER: &str = "dictionary-order";
static OPT_MERGE: &str = "merge";
static OPT_CHECK: &str = "check";
static OPT_CHECK_SILENT: &str = "check-silent";
static OPT_DEBUG: &str = "debug";
static OPT_IGNORE_CASE: &str = "ignore-case";
static OPT_IGNORE_BLANKS: &str = "ignore-blanks";
static OPT_IGNORE_NONPRINTING: &str = "ignore-nonprinting";
static OPT_OUTPUT: &str = "output";
static OPT_REVERSE: &str = "reverse";
static OPT_STABLE: &str = "stable";
static OPT_UNIQUE: &str = "unique";
static OPT_KEY: &str = "key";
static OPT_SEPARATOR: &str = "field-separator";
static OPT_RANDOM: &str = "random-sort";
static OPT_ZERO_TERMINATED: &str = "zero-terminated";
static OPT_PARALLEL: &str = "parallel";
static OPT_FILES0_FROM: &str = "files0-from";
static OPT_BUF_SIZE: &str = "buffer-size";
static OPT_TMP_DIR: &str = "temporary-directory";

static ARG_FILES: &str = "files";

static DECIMAL_PT: char = '.';
static THOUSANDS_SEP: char = ',';

static NEGATIVE: char = '-';
static POSITIVE: char = '+';

static DEFAULT_TMPDIR: &str = r"/tmp";
// 4GB buffer for Vec<Line> before we dump to disk, never used
static DEFAULT_BUF_SIZE: usize = 4000000000;

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
enum SortMode {
    Numeric,
    HumanNumeric,
    GeneralNumeric,
    Month,
    Version,
    Default,
}
#[derive(Clone)]
struct GlobalSettings {
    mode: SortMode,
    debug: bool,
    ignore_blanks: bool,
    ignore_case: bool,
    dictionary_order: bool,
    ignore_non_printing: bool,
    merge: bool,
    reverse: bool,
    outfile: Option<String>,
    stable: bool,
    unique: bool,
    check: bool,
    check_silent: bool,
    random: bool,
    salt: String,
    selectors: Vec<FieldSelector>,
    separator: Option<char>,
    threads: String,
    zero_terminated: bool,
    buffer_size: usize,
    tmp_dir: PathBuf,
}

impl GlobalSettings {
    // It's back to do conversions for command line opts!
    // Probably want to do through numstrcmp somehow now?
    fn human_numeric_convert(a: &str) -> usize {
        let num_str = &a[get_leading_gen(a)];
        let (_, suf_str) = a.split_at(num_str.len());
        let num_usize = num_str
            .parse::<usize>()
            .expect("Error parsing buffer size: ");
        let suf_usize: usize = match suf_str.to_uppercase().as_str() {
            // SI Units
            "B" => 1usize,
            "K" => 1024usize,
            "M" => 1024000usize,
            "G" => 1024000000usize,
            // GNU regards empty human numeric value as 1024 bytes
            _ => 1024usize,
        };
        num_usize * suf_usize
    }
}

impl Default for GlobalSettings {
    fn default() -> GlobalSettings {
        GlobalSettings {
            mode: SortMode::Default,
            debug: false,
            ignore_blanks: false,
            ignore_case: false,
            dictionary_order: false,
            ignore_non_printing: false,
            merge: false,
            reverse: false,
            outfile: None,
            stable: false,
            unique: false,
            check: false,
            check_silent: false,
            random: false,
            salt: String::new(),
            selectors: vec![],
            separator: None,
            threads: String::new(),
            zero_terminated: false,
            buffer_size: DEFAULT_BUF_SIZE,
            tmp_dir: PathBuf::from(DEFAULT_TMPDIR),
        }
    }
}
#[derive(Clone)]
struct KeySettings {
    mode: SortMode,
    ignore_blanks: bool,
    ignore_case: bool,
    dictionary_order: bool,
    ignore_non_printing: bool,
    random: bool,
    reverse: bool,
}

impl From<&GlobalSettings> for KeySettings {
    fn from(settings: &GlobalSettings) -> Self {
        Self {
            mode: settings.mode,
            ignore_blanks: settings.ignore_blanks,
            ignore_case: settings.ignore_case,
            ignore_non_printing: settings.ignore_non_printing,
            random: settings.random,
            reverse: settings.reverse,
            dictionary_order: settings.dictionary_order,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Represents the string selected by a FieldSelector.
enum SelectionRange {
    /// If we had to transform this selection, we have to store a new string.
    String(String),
    /// If there was no transformation, we can store an index into the line.
    ByIndex(Range<usize>),
}

impl SelectionRange {
    /// Gets the actual string slice represented by this Selection.
    fn get_str<'a>(&'a self, line: &'a str) -> &'a str {
        match self {
            SelectionRange::String(string) => string.as_str(),
            SelectionRange::ByIndex(range) => &line[range.to_owned()],
        }
    }

    fn shorten(&mut self, new_range: Range<usize>) {
        match self {
            SelectionRange::String(string) => {
                string.drain(new_range.end..);
                string.drain(..new_range.start);
            }
            SelectionRange::ByIndex(range) => {
                range.end = range.start + new_range.end;
                range.start += new_range.start;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
enum NumCache {
    AsF64(GeneralF64ParseResult),
    WithInfo(NumInfo),
    None,
}

impl NumCache {
    fn as_f64(&self) -> GeneralF64ParseResult {
        match self {
            NumCache::AsF64(n) => *n,
            _ => unreachable!(),
        }
    }
    fn as_num_info(&self) -> &NumInfo {
        match self {
            NumCache::WithInfo(n) => n,
            _ => unreachable!(),
        }
    }
}
#[derive(Serialize, Deserialize, Clone)]
struct Selection {
    range: SelectionRange,
    num_cache: NumCache,
}

impl Selection {
    /// Gets the actual string slice represented by this Selection.
    fn get_str<'a>(&'a self, line: &'a Line) -> &'a str {
        self.range.get_str(&line.line)
    }
}

type Field = Range<usize>;

#[derive(Serialize, Deserialize, Clone)]
struct Line {
    line: String,
    // The common case is not to specify fields. Let's make this fast.
    selections: SmallVec<[Selection; 1]>,
}

impl ExternallySortable for Line {
    fn get_size(&self) -> u64 {
        // Currently 96 bytes, but that could change, so we get that size here
        std::mem::size_of::<Line>() as u64
    }
}

impl Line {
    fn new(line: String, settings: &GlobalSettings) -> Self {
        let fields = if settings
            .selectors
            .iter()
            .any(|selector| selector.needs_tokens())
        {
            // Only tokenize if we will need tokens.
            Some(tokenize(&line, settings.separator))
        } else {
            None
        };

        let selections = settings
            .selectors
            .iter()
            .map(|selector| {
                let range = selector.get_selection(&line, fields.as_deref());
                let mut range = if let Some(transformed) =
                    transform(&line[range.to_owned()], &selector.settings)
                {
                    SelectionRange::String(transformed)
                } else {
                    SelectionRange::ByIndex(range)
                };
                let num_cache = if selector.settings.mode == SortMode::Numeric
                    || selector.settings.mode == SortMode::HumanNumeric
                {
                    let (info, num_range) = NumInfo::parse(
                        range.get_str(&line),
                        NumInfoParseSettings {
                            accept_si_units: selector.settings.mode == SortMode::HumanNumeric,
                            thousands_separator: Some(THOUSANDS_SEP),
                            decimal_pt: Some(DECIMAL_PT),
                        },
                    );
                    range.shorten(num_range);
                    NumCache::WithInfo(info)
                } else if selector.settings.mode == SortMode::GeneralNumeric
                    && settings.buffer_size == DEFAULT_BUF_SIZE
                {
                    let str = range.get_str(&line);
                    NumCache::AsF64(general_f64_parse(&str[get_leading_gen(str)]))
                } else {
                    NumCache::None
                };
                Selection { range, num_cache }
            })
            .collect();
        Self { line, selections }
    }

    /// Writes indicators for the selections this line matched. The original line content is NOT expected
    /// to be already printed.
    fn print_debug(
        &self,
        settings: &GlobalSettings,
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        // We do not consider this function performance critical, as debug output is only useful for small files,
        // which are not a performance problem in any case. Therefore there aren't any special performance
        // optimizations here.

        let line = self.line.replace('\t', ">");
        writeln!(writer, "{}", line)?;

        let fields = tokenize(&self.line, settings.separator);
        for selector in settings.selectors.iter() {
            let mut selection = selector.get_selection(&self.line, Some(&fields));
            match selector.settings.mode {
                SortMode::Numeric | SortMode::HumanNumeric => {
                    // find out which range is used for numeric comparisons
                    let (_, num_range) = NumInfo::parse(
                        &self.line[selection.clone()],
                        NumInfoParseSettings {
                            accept_si_units: selector.settings.mode == SortMode::HumanNumeric,
                            thousands_separator: Some(THOUSANDS_SEP),
                            decimal_pt: Some(DECIMAL_PT),
                        },
                    );
                    let initial_selection = selection.clone();

                    // Shorten selection to num_range.
                    selection.start += num_range.start;
                    selection.end = selection.start + num_range.len();

                    // include a trailing si unit
                    if selector.settings.mode == SortMode::HumanNumeric
                        && self.line[selection.end..initial_selection.end]
                            .starts_with(&['k', 'K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y'][..])
                    {
                        selection.end += 1;
                    }

                    // include leading zeroes, a leading minus or a leading decimal point
                    while self.line[initial_selection.start..selection.start]
                        .ends_with(&['-', '0', '.'][..])
                    {
                        selection.start -= 1;
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

                    let month = if month_parse(initial_selection) == Month::Unknown {
                        // We failed to parse a month, which is equivalent to matching nothing.
                        0..0
                    } else {
                        // We parsed a month. Match the three first non-whitespace characters, which must be the month we parsed.
                        let mut chars = initial_selection
                            .char_indices()
                            .skip_while(|(_, c)| c.is_whitespace());
                        chars.next().unwrap().0
                            ..chars.nth(2).map_or(initial_selection.len(), |(idx, _)| idx)
                    };

                    // Shorten selection to month.
                    selection.start += month.start;
                    selection.end = selection.start + month.len();
                }
                _ => {}
            }

            write!(
                writer,
                "{}",
                " ".repeat(UnicodeWidthStr::width(&line[..selection.start]))
            )?;

            // TODO: Once our minimum supported rust version is at least 1.47, use selection.is_empty() instead.
            #[allow(clippy::len_zero)]
            {
                if selection.len() == 0 {
                    writeln!(writer, "^ no match for key")?;
                } else {
                    writeln!(
                        writer,
                        "{}",
                        "_".repeat(UnicodeWidthStr::width(&line[selection]))
                    )?;
                }
            }
        }
        if !(settings.random
            || settings.stable
            || settings.unique
            || !(settings.dictionary_order
                || settings.ignore_blanks
                || settings.ignore_case
                || settings.ignore_non_printing
                || settings.mode != SortMode::Default))
        {
            // A last resort comparator is in use, underline the whole line.
            if self.line.is_empty() {
                writeln!(writer, "^ no match for key")?;
            } else {
                writeln!(
                    writer,
                    "{}",
                    "_".repeat(UnicodeWidthStr::width(line.as_str()))
                )?;
            }
        }
        Ok(())
    }
}

/// Transform this line. Returns None if there's no need to transform.
fn transform(line: &str, settings: &KeySettings) -> Option<String> {
    let mut transformed = None;
    if settings.ignore_case {
        transformed = Some(line.to_uppercase());
    }
    if settings.ignore_blanks {
        transformed = Some(
            transformed
                .as_deref()
                .unwrap_or(line)
                .trim_start()
                .to_string(),
        );
    }
    if settings.dictionary_order {
        transformed = Some(remove_nondictionary_chars(
            transformed.as_deref().unwrap_or(line),
        ));
    }
    if settings.ignore_non_printing {
        transformed = Some(remove_nonprinting_chars(
            transformed.as_deref().unwrap_or(line),
        ));
    }
    transformed
}

/// Tokenize a line into fields.
fn tokenize(line: &str, separator: Option<char>) -> Vec<Field> {
    if let Some(separator) = separator {
        tokenize_with_separator(line, separator)
    } else {
        tokenize_default(line)
    }
}

/// By default fields are separated by the first whitespace after non-whitespace.
/// Whitespace is included in fields at the start.
fn tokenize_default(line: &str) -> Vec<Field> {
    let mut tokens = vec![0..0];
    // pretend that there was whitespace in front of the line
    let mut previous_was_whitespace = true;
    for (idx, char) in line.char_indices() {
        if char.is_whitespace() {
            if !previous_was_whitespace {
                tokens.last_mut().unwrap().end = idx;
                tokens.push(idx..0);
            }
            previous_was_whitespace = true;
        } else {
            previous_was_whitespace = false;
        }
    }
    tokens.last_mut().unwrap().end = line.len();
    tokens
}

/// Split between separators. These separators are not included in fields.
fn tokenize_with_separator(line: &str, separator: char) -> Vec<Field> {
    let mut tokens = vec![];
    let separator_indices =
        line.char_indices()
            .filter_map(|(i, c)| if c == separator { Some(i) } else { None });
    let mut start = 0;
    for sep_idx in separator_indices {
        tokens.push(start..sep_idx);
        start = sep_idx + 1;
    }
    if start < line.len() {
        tokens.push(start..line.len());
    }
    tokens
}

#[derive(Clone)]
struct KeyPosition {
    /// 1-indexed, 0 is invalid.
    field: usize,
    /// 1-indexed, 0 is end of field.
    char: usize,
    ignore_blanks: bool,
}

impl KeyPosition {
    fn parse(key: &str, default_char_index: usize, settings: &mut KeySettings) -> Self {
        let mut field_and_char = key.split('.');
        let mut field = field_and_char
            .next()
            .unwrap_or_else(|| crash!(1, "invalid key `{}`", key));
        let mut char = field_and_char.next();

        // If there is a char index, we expect options to appear after it. Otherwise we expect them after the field index.
        let value_with_options = char.as_mut().unwrap_or(&mut field);

        let mut ignore_blanks = settings.ignore_blanks;
        if let Some(options_start) = value_with_options.chars().position(char::is_alphabetic) {
            for option in value_with_options[options_start..].chars() {
                // valid options: MbdfghinRrV
                match option {
                    'M' => settings.mode = SortMode::Month,
                    'b' => ignore_blanks = true,
                    'd' => settings.dictionary_order = true,
                    'f' => settings.ignore_case = true,
                    'g' => settings.mode = SortMode::GeneralNumeric,
                    'h' => settings.mode = SortMode::HumanNumeric,
                    'i' => settings.ignore_non_printing = true,
                    'n' => settings.mode = SortMode::Numeric,
                    'R' => settings.random = true,
                    'r' => settings.reverse = true,
                    'V' => settings.mode = SortMode::Version,
                    c => {
                        crash!(1, "invalid option for key: `{}`", c)
                    }
                }
                // All numeric sorts and month sort conflict with dictionary_order and ignore_non_printing.
                // Instad of reporting an error, let them overwrite each other.

                // FIXME: This should only override if the overridden flag is a global flag.
                // If conflicting flags are attached to the key, GNU sort crashes and we should probably too.
                match option {
                    'h' | 'n' | 'g' | 'M' => {
                        settings.dictionary_order = false;
                        settings.ignore_non_printing = false;
                    }
                    'd' | 'i' => {
                        settings.mode = match settings.mode {
                            SortMode::Numeric
                            | SortMode::HumanNumeric
                            | SortMode::GeneralNumeric
                            | SortMode::Month => SortMode::Default,
                            // Only SortMode::Default and SortMode::Version work with dictionary_order and ignore_non_printing
                            m @ SortMode::Default | m @ SortMode::Version => m,
                        }
                    }
                    _ => {}
                }
            }
            // Strip away option characters from the original value so we can parse it later
            *value_with_options = &value_with_options[..options_start];
        }

        let field = field
            .parse()
            .unwrap_or_else(|e| crash!(1, "failed to parse field index for key `{}`: {}", key, e));
        if field == 0 {
            crash!(1, "field index was 0");
        }
        let char = char.map_or(default_char_index, |char| {
            char.parse().unwrap_or_else(|e| {
                crash!(
                    1,
                    "failed to parse character index for key `{}`: {}",
                    key,
                    e
                )
            })
        });
        Self {
            field,
            char,
            ignore_blanks,
        }
    }
}
#[derive(Clone)]
struct FieldSelector {
    from: KeyPosition,
    to: Option<KeyPosition>,
    settings: KeySettings,
}

impl FieldSelector {
    fn needs_tokens(&self) -> bool {
        self.from.field != 1 || self.from.char == 0 || self.to.is_some()
    }

    /// Look up the slice that corresponds to this selector for the given line.
    /// If needs_fields returned false, fields may be None.
    fn get_selection<'a>(&self, line: &'a str, tokens: Option<&[Field]>) -> Range<usize> {
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

        // Get the index for this line given the KeyPosition
        fn resolve_index(
            line: &str,
            tokens: Option<&[Field]>,
            position: &KeyPosition,
        ) -> Resolution {
            if tokens.map_or(false, |fields| fields.len() < position.field) {
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
                idx += line[idx..]
                    .char_indices()
                    .nth(position.char - 1)
                    .map_or(line.len(), |(idx, _)| idx);
                if idx >= line.len() {
                    Resolution::TooHigh
                } else {
                    if position.ignore_blanks {
                        if let Some((not_whitespace, _)) =
                            line[idx..].char_indices().find(|(_, c)| !c.is_whitespace())
                        {
                            idx += not_whitespace;
                        } else {
                            return Resolution::TooHigh;
                        }
                    }
                    Resolution::StartOfChar(idx)
                }
            }
        }

        match resolve_index(line, tokens, &self.from) {
            Resolution::StartOfChar(from) => {
                let to = self.to.as_ref().map(|to| resolve_index(line, tokens, &to));

                match to {
                    Some(Resolution::StartOfChar(mut to)) => {
                        to += line[to..].chars().next().map_or(1, |c| c.len_utf8());
                        from..to
                    }
                    Some(Resolution::EndOfChar(to)) => from..to,
                    // If `to` was not given or the match would be after the end of the line,
                    // match everything until the end of the line.
                    None | Some(Resolution::TooHigh) => from..line.len(),
                    // If `to` is before the start of the line, report no match.
                    // This can happen if the line starts with a separator.
                    Some(Resolution::TooLow) => 0..0,
                }
            }
            Resolution::TooLow | Resolution::EndOfChar(_) => {
                unreachable!("This should only happen if the field start index is 0, but that should already have caused an error.")
            }
            // While for comparisons it's only important that this is an empty slice,
            // to produce accurate debug output we need to match an empty slice at the end of the line.
            Resolution::TooHigh => line.len()..line.len(),
        }
    }
}

struct MergeableFile<'a> {
    lines: Lines<BufReader<Box<dyn Read>>>,
    current_line: Line,
    settings: &'a GlobalSettings,
}

// BinaryHeap depends on `Ord`. Note that we want to pop smallest items
// from the heap first, and BinaryHeap.pop() returns the largest, so we
// trick it into the right order by calling reverse() here.
impl<'a> Ord for MergeableFile<'a> {
    fn cmp(&self, other: &MergeableFile) -> Ordering {
        compare_by(&self.current_line, &other.current_line, self.settings).reverse()
    }
}

impl<'a> PartialOrd for MergeableFile<'a> {
    fn partial_cmp(&self, other: &MergeableFile) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for MergeableFile<'a> {
    fn eq(&self, other: &MergeableFile) -> bool {
        Ordering::Equal == self.cmp(other)
    }
}

impl<'a> Eq for MergeableFile<'a> {}

struct FileMerger<'a> {
    heap: BinaryHeap<MergeableFile<'a>>,
    settings: &'a GlobalSettings,
}

impl<'a> FileMerger<'a> {
    fn new(settings: &'a GlobalSettings) -> FileMerger<'a> {
        FileMerger {
            heap: BinaryHeap::new(),
            settings,
        }
    }
    fn push_file(&mut self, mut lines: Lines<BufReader<Box<dyn Read>>>) {
        if let Some(Ok(next_line)) = lines.next() {
            let mergeable_file = MergeableFile {
                lines,
                current_line: Line::new(next_line, &self.settings),
                settings: &self.settings,
            };
            self.heap.push(mergeable_file);
        }
    }
}

impl<'a> Iterator for FileMerger<'a> {
    type Item = Line;
    fn next(&mut self) -> Option<Line> {
        match self.heap.pop() {
            Some(mut current) => {
                match current.lines.next() {
                    Some(Ok(next_line)) => {
                        let ret = replace(
                            &mut current.current_line,
                            Line::new(next_line, &self.settings),
                        );
                        self.heap.push(current);
                        Some(ret)
                    }
                    _ => {
                        // Don't put it back in the heap (it's empty/erroring)
                        // but its first line is still valid.
                        Some(current.current_line)
                    }
                }
            }
            None => None,
        }
    }
}

fn get_usage() -> String {
    format!(
        "{0} {1}
Usage:
 {0} [OPTION]... [FILE]...
Write the sorted concatenation of all FILE(s) to standard output.
Mandatory arguments for long options are mandatory for short options too.
With no FILE, or when FILE is -, read standard input.",
        NAME, VERSION
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();
    let usage = get_usage();
    let mut settings: GlobalSettings = Default::default();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_HUMAN_NUMERIC_SORT)
                .short("h")
                .long(OPT_HUMAN_NUMERIC_SORT)
                .help("compare according to human readable sizes, eg 1M > 100k"),
        )
        .arg(
            Arg::with_name(OPT_MONTH_SORT)
                .short("M")
                .long(OPT_MONTH_SORT)
                .help("compare according to month name abbreviation"),
        )
        .arg(
            Arg::with_name(OPT_NUMERIC_SORT)
                .short("n")
                .long(OPT_NUMERIC_SORT)
                .help("compare according to string numerical value"),
        )
        .arg(
            Arg::with_name(OPT_GENERAL_NUMERIC_SORT)
                .short("g")
                .long(OPT_GENERAL_NUMERIC_SORT)
                .help("compare according to string general numerical value"),
        )
        .arg(
            Arg::with_name(OPT_VERSION_SORT)
                .short("V")
                .long(OPT_VERSION_SORT)
                .help("Sort by SemVer version number, eg 1.12.2 > 1.1.2"),
        )
        .arg(
            Arg::with_name(OPT_DICTIONARY_ORDER)
                .short("d")
                .long(OPT_DICTIONARY_ORDER)
                .help("consider only blanks and alphanumeric characters")
                .conflicts_with_all(&[OPT_NUMERIC_SORT, OPT_GENERAL_NUMERIC_SORT, OPT_HUMAN_NUMERIC_SORT, OPT_MONTH_SORT]),
        )
        .arg(
            Arg::with_name(OPT_MERGE)
                .short("m")
                .long(OPT_MERGE)
                .help("merge already sorted files; do not sort"),
        )
        .arg(
            Arg::with_name(OPT_CHECK)
                .short("c")
                .long(OPT_CHECK)
                .help("check for sorted input; do not sort"),
        )
        .arg(
            Arg::with_name(OPT_CHECK_SILENT)
                .short("C")
                .long(OPT_CHECK_SILENT)
                .help("exit successfully if the given file is already sorted, and exit with status 1 otherwise."),
        )
        .arg(
            Arg::with_name(OPT_IGNORE_CASE)
                .short("f")
                .long(OPT_IGNORE_CASE)
                .help("fold lower case to upper case characters"),
        )
        .arg(
            Arg::with_name(OPT_IGNORE_NONPRINTING)
                .short("i")
                .long(OPT_IGNORE_NONPRINTING)
                .help("ignore nonprinting characters")
                .conflicts_with_all(&[OPT_NUMERIC_SORT, OPT_GENERAL_NUMERIC_SORT, OPT_HUMAN_NUMERIC_SORT, OPT_MONTH_SORT]),
        )
        .arg(
            Arg::with_name(OPT_IGNORE_BLANKS)
                .short("b")
                .long(OPT_IGNORE_BLANKS)
                .help("ignore leading blanks when finding sort keys in each line"),
        )
        .arg(
            Arg::with_name(OPT_OUTPUT)
                .short("o")
                .long(OPT_OUTPUT)
                .help("write output to FILENAME instead of stdout")
                .takes_value(true)
                .value_name("FILENAME"),
        )
        .arg(
            Arg::with_name(OPT_RANDOM)
                .short("R")
                .long(OPT_RANDOM)
                .help("shuffle in random order"),
        )
        .arg(
            Arg::with_name(OPT_REVERSE)
                .short("r")
                .long(OPT_REVERSE)
                .help("reverse the output"),
        )
        .arg(
            Arg::with_name(OPT_STABLE)
                .short("s")
                .long(OPT_STABLE)
                .help("stabilize sort by disabling last-resort comparison"),
        )
        .arg(
            Arg::with_name(OPT_UNIQUE)
                .short("u")
                .long(OPT_UNIQUE)
                .help("output only the first of an equal run"),
        )
        .arg(
            Arg::with_name(OPT_KEY)
                .short("k")
                .long(OPT_KEY)
                .help("sort by a key")
                .long_help(LONG_HELP_KEYS)
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(OPT_SEPARATOR)
                .short("t")
                .long(OPT_SEPARATOR)
                .help("custom separator for -k")
                .takes_value(true))
            .arg(Arg::with_name(OPT_ZERO_TERMINATED)
                .short("z")
                .long(OPT_ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::with_name(OPT_PARALLEL)
                .long(OPT_PARALLEL)
                .help("change the number of threads running concurrently to NUM_THREADS")
                .takes_value(true)
                .value_name("NUM_THREADS"),
        )
        .arg(
            Arg::with_name(OPT_BUF_SIZE)
                .short("S")
                .long(OPT_BUF_SIZE)
                .help("sets the maximum SIZE of each segment in number of sorted items")
                .takes_value(true)
                .value_name("SIZE"),
        )
        .arg(
            Arg::with_name(OPT_TMP_DIR)
                .short("T")
                .long(OPT_TMP_DIR)
                .help("use DIR for temporaries, not $TMPDIR or /tmp")
                .takes_value(true)
                .value_name("DIR"),
        )
        .arg(
            Arg::with_name(OPT_FILES0_FROM)
                .long(OPT_FILES0_FROM)
                .help("read input from the files specified by NUL-terminated NUL_FILES")
                .takes_value(true)
                .value_name("NUL_FILES")
                .multiple(true),
        )
        .arg(
            Arg::with_name(OPT_DEBUG)
                .long(OPT_DEBUG)
                .help("underline the parts of the line that are actually used for sorting"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
        .get_matches_from(args);

    settings.debug = matches.is_present(OPT_DEBUG);

    // check whether user specified a zero terminated list of files for input, otherwise read files from args
    let mut files: Vec<String> = if matches.is_present(OPT_FILES0_FROM) {
        let files0_from: Vec<String> = matches
            .values_of(OPT_FILES0_FROM)
            .map(|v| v.map(ToString::to_string).collect())
            .unwrap_or_default();

        let mut files = Vec::new();
        for path in &files0_from {
            let (reader, _) = open(path.as_str()).expect("Could not read from file specified.");
            let buf_reader = BufReader::new(reader);
            for line in buf_reader.split(b'\0').flatten() {
                files.push(
                    std::str::from_utf8(&line)
                        .expect("Could not parse string from zero terminated input.")
                        .to_string(),
                );
            }
        }
        files
    } else {
        matches
            .values_of(ARG_FILES)
            .map(|v| v.map(ToString::to_string).collect())
            .unwrap_or_default()
    };

    settings.mode = if matches.is_present(OPT_HUMAN_NUMERIC_SORT) {
        SortMode::HumanNumeric
    } else if matches.is_present(OPT_MONTH_SORT) {
        SortMode::Month
    } else if matches.is_present(OPT_GENERAL_NUMERIC_SORT) {
        SortMode::GeneralNumeric
    } else if matches.is_present(OPT_NUMERIC_SORT) {
        SortMode::Numeric
    } else if matches.is_present(OPT_VERSION_SORT) {
        SortMode::Version
    } else {
        SortMode::Default
    };

    settings.dictionary_order = matches.is_present(OPT_DICTIONARY_ORDER);
    settings.ignore_non_printing = matches.is_present(OPT_IGNORE_NONPRINTING);
    if matches.is_present(OPT_PARALLEL) {
        // "0" is default - threads = num of cores
        settings.threads = matches
            .value_of(OPT_PARALLEL)
            .map(String::from)
            .unwrap_or_else(|| "0".to_string());
        env::set_var("RAYON_NUM_THREADS", &settings.threads);
    }

    if matches.is_present(OPT_BUF_SIZE) {
        settings.buffer_size = {
            let input = matches
                .value_of(OPT_BUF_SIZE)
                .map(String::from)
                .unwrap_or(format!("{}", DEFAULT_BUF_SIZE));

            GlobalSettings::human_numeric_convert(&input)
        }
    }

    if matches.is_present(OPT_TMP_DIR) {
        let result = matches
            .value_of(OPT_TMP_DIR)
            .map(String::from)
            .unwrap_or(DEFAULT_TMPDIR.to_owned());
        settings.tmp_dir = PathBuf::from(result);
    } else {
        for (key, value) in env::vars_os() {
            if key == OsString::from("TMPDIR") {
                settings.tmp_dir = PathBuf::from(value);
                break;
            }
            settings.tmp_dir = PathBuf::from(DEFAULT_TMPDIR);
        }
    }

    settings.zero_terminated = matches.is_present(OPT_ZERO_TERMINATED);
    settings.merge = matches.is_present(OPT_MERGE);

    settings.check = matches.is_present(OPT_CHECK);
    if matches.is_present(OPT_CHECK_SILENT) {
        settings.check_silent = matches.is_present(OPT_CHECK_SILENT);
        settings.check = true;
    };

    settings.ignore_case = matches.is_present(OPT_IGNORE_CASE);

    settings.ignore_blanks = matches.is_present(OPT_IGNORE_BLANKS);

    settings.outfile = matches.value_of(OPT_OUTPUT).map(String::from);
    settings.reverse = matches.is_present(OPT_REVERSE);
    settings.stable = matches.is_present(OPT_STABLE);
    settings.unique = matches.is_present(OPT_UNIQUE);

    if matches.is_present(OPT_RANDOM) {
        settings.random = matches.is_present(OPT_RANDOM);
        settings.salt = get_rand_string();
    }

    if files.is_empty() {
        /* if no file, default to stdin */
        files.push("-".to_owned());
    } else if settings.check && files.len() != 1 {
        crash!(1, "extra operand `{}' not allowed with -c", files[1])
    }

    if let Some(arg) = matches.args.get(OPT_SEPARATOR) {
        let separator = arg.vals[0].to_string_lossy();
        let separator = separator;
        if separator.len() != 1 {
            crash!(1, "separator must be exactly one character long");
        }
        settings.separator = Some(separator.chars().next().unwrap())
    }

    if matches.is_present(OPT_KEY) {
        for key in &matches.args[OPT_KEY].vals {
            let key = key.to_string_lossy();
            let mut from_to = key.split(',');
            let mut key_settings = KeySettings::from(&settings);
            let from = KeyPosition::parse(
                from_to
                    .next()
                    .unwrap_or_else(|| crash!(1, "invalid key `{}`", key)),
                1,
                &mut key_settings,
            );
            if from.char == 0 {
                crash!(
                    1,
                    "invalid character index 0 in `{}` for the start position of a field",
                    key
                )
            }
            let to = from_to
                .next()
                .map(|to| KeyPosition::parse(to, 0, &mut key_settings));
            let field_selector = FieldSelector {
                from,
                to,
                settings: key_settings,
            };
            settings.selectors.push(field_selector);
        }
    }

    if !settings.stable || !matches.is_present(OPT_KEY) {
        // add a default selector matching the whole line
        let key_settings = KeySettings::from(&settings);
        settings.selectors.push(FieldSelector {
            from: KeyPosition {
                field: 1,
                char: 1,
                ignore_blanks: key_settings.ignore_blanks,
            },
            to: None,
            settings: key_settings,
        });
    }

    exec(files, settings)
}

fn exec(files: Vec<String>, settings: GlobalSettings) -> i32 {
    let mut lines = Vec::new();
    let mut file_merger = FileMerger::new(&settings);

    for path in &files {
        let (reader, _) = match open(path) {
            Some(x) => x,
            None => continue,
        };

        let buf_reader = BufReader::new(reader);

        if settings.merge {
            file_merger.push_file(buf_reader.lines());
        } else if settings.zero_terminated {
            for line in buf_reader.split(b'\0').flatten() {
                lines.push(Line::new(
                    std::str::from_utf8(&line)
                        .expect("Could not parse string from zero terminated input.")
                        .to_string(),
                    &settings,
                ));
            }
        } else {
            for line in buf_reader.lines() {
                if let Ok(n) = line {
                    lines.push(Line::new(n, &settings));
                } else {
                    break;
                }
            }
        }
    }

    if settings.check {
        return exec_check_file(&lines, &settings);
    }

    // Only use ext_sorter when we need to.
    // Probably faster that we don't create
    // an owned value each run
    if settings.buffer_size != DEFAULT_BUF_SIZE {
        lines = ext_sort_by(lines, settings.clone());
    } else {
        sort_by(&mut lines, &settings);
    }

    if settings.merge {
        if settings.unique {
            print_sorted(
                file_merger.dedup_by(|a, b| compare_by(a, b, &settings) == Ordering::Equal),
                &settings,
            )
        } else {
            print_sorted(file_merger, &settings)
        }
    } else if settings.unique {
        print_sorted(
            lines
                .into_iter()
                .dedup_by(|a, b| compare_by(a, b, &settings) == Ordering::Equal),
            &settings,
        )
    } else {
        print_sorted(lines.into_iter(), &settings)
    }

    0
}

fn exec_check_file(unwrapped_lines: &[Line], settings: &GlobalSettings) -> i32 {
    // errors yields the line before each disorder,
    // plus the last line (quirk of .coalesce())
    let mut errors =
        unwrapped_lines
            .iter()
            .enumerate()
            .coalesce(|(last_i, last_line), (i, line)| {
                if compare_by(&last_line, &line, &settings) == Ordering::Greater {
                    Err(((last_i, last_line), (i, line)))
                } else {
                    Ok((i, line))
                }
            });
    if let Some((first_error_index, _line)) = errors.next() {
        // Check for a second "error", as .coalesce() always returns the last
        // line, no matter what our merging function does.
        if let Some(_last_line_or_next_error) = errors.next() {
            if !settings.check_silent {
                println!("sort: disorder in line {}", first_error_index);
            };
            1
        } else {
            // first "error" was actually the last line.
            0
        }
    } else {
        // unwrapped_lines was empty. Empty files are defined to be sorted.
        0
    }
}

fn ext_sort_by(unsorted: Vec<Line>, settings: GlobalSettings) -> Vec<Line> {
    let external_sorter = ExternalSorter::new(
        settings.buffer_size as u64,
        Some(settings.tmp_dir.clone()),
        settings.clone(),
    );
    let iter = external_sorter
        .sort_by(unsorted.into_iter(), settings.clone())
        .unwrap()
        .map(|x| x.unwrap())
        .collect::<Vec<Line>>();
    iter
}

fn sort_by(unsorted: &mut Vec<Line>, settings: &GlobalSettings) {
    if settings.stable || settings.unique {
        unsorted.par_sort_by(|a, b| compare_by(a, b, &settings))
    } else {
        unsorted.par_sort_unstable_by(|a, b| compare_by(a, b, &settings))
    }
}

fn compare_by(a: &Line, b: &Line, global_settings: &GlobalSettings) -> Ordering {
    for (idx, selector) in global_settings.selectors.iter().enumerate() {
        let a_selection = &a.selections[idx];
        let b_selection = &b.selections[idx];
        let a_str = a_selection.get_str(a);
        let b_str = b_selection.get_str(b);
        let settings = &selector.settings;

        let cmp: Ordering = if settings.random {
            random_shuffle(a_str, b_str, global_settings.salt.clone())
        } else {
            match settings.mode {
                SortMode::Numeric | SortMode::HumanNumeric => numeric_str_cmp(
                    (a_str, a_selection.num_cache.as_num_info()),
                    (b_str, b_selection.num_cache.as_num_info()),
                ),
                SortMode::GeneralNumeric => {
                         general_numeric_compare(
                             general_f64_parse(&a_str[get_leading_gen(a_str)]),
                             general_f64_parse(&b_str[get_leading_gen(b_str)]),)
                }
                SortMode::Month => month_compare(a_str, b_str),
                SortMode::Version => version_compare(a_str, b_str),
                SortMode::Default => default_compare(a_str, b_str),
            }
        };
        if cmp != Ordering::Equal {
            return if settings.reverse { cmp.reverse() } else { cmp };
        }
    }

    // Call "last resort compare" if all selectors returned Equal
    let cmp = if global_settings.random || global_settings.stable || global_settings.unique {
        Ordering::Equal
    } else {
        default_compare(&a.line, &b.line)
    };

    if global_settings.reverse {
        cmp.reverse()
    } else {
        cmp
    }
}

// Test output against BSDs and GNU with their locale
// env var set to lc_ctype=utf-8 to enjoy the exact same output.
#[inline(always)]
fn default_compare(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

// This function cleans up the initial comparison done by leading_num_common for a general numeric compare.
// In contrast to numeric compare, GNU general numeric/FP sort *should* recognize positive signs and
// scientific notation, so we strip those lines only after the end of the following numeric string.
// For example, 5e10KFD would be 5e10 or 5x10^10 and +10000HFKJFK would become 10000.
fn get_leading_gen(input: &str) -> Range<usize> {
    let trimmed = input.trim_start();
    let leading_whitespace_len = input.len() - trimmed.len();
    for allowed_prefix in &["inf", "-inf", "nan"] {
        if trimmed.is_char_boundary(allowed_prefix.len())
            && trimmed[..allowed_prefix.len()].eq_ignore_ascii_case(allowed_prefix)
        {
            return leading_whitespace_len..(leading_whitespace_len + allowed_prefix.len());
        }
    }
    // Make this iter peekable to see if next char is numeric
    let mut char_indices = trimmed.char_indices().peekable();

    let first = char_indices.peek();

    if first.map_or(false, |&(_, c)| c == NEGATIVE || c == POSITIVE) {
        char_indices.next();
    }

    let mut had_e_notation = false;
    let mut had_decimal_pt = false;
    while let Some((idx, c)) = char_indices.next() {
        if c.is_ascii_digit() {
            continue;
        }
        if c == DECIMAL_PT && !had_decimal_pt {
            had_decimal_pt = true;
            continue;
        }
        let next_char_numeric = char_indices
            .peek()
            .map_or(false, |(_, c)| c.is_ascii_digit());
        if (c == 'e' || c == 'E') && !had_e_notation && next_char_numeric {
            had_e_notation = true;
            continue;
        }
        return leading_whitespace_len..(leading_whitespace_len + idx);
    }
    leading_whitespace_len..input.len()
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, PartialOrd)]
enum GeneralF64ParseResult {
    Invalid,
    NaN,
    NegInfinity,
    Number(f64),
    Infinity,
}

/// Parse the beginning string into an f64, returning -inf instead of NaN on errors.
#[inline(always)]
fn general_f64_parse(a: &str) -> GeneralF64ParseResult {
    // The actual behavior here relies on Rust's implementation of parsing floating points.
    // For example "nan", "inf" (ignoring the case) and "infinity" are only parsed to floats starting from 1.53.
    // TODO: Once our minimum supported Rust version is 1.53 or above, we should add tests for those cases.
    match a.parse::<f64>() {
        Ok(a) if a.is_nan() => GeneralF64ParseResult::NaN,
        Ok(a) if a == std::f64::NEG_INFINITY => GeneralF64ParseResult::NegInfinity,
        Ok(a) if a == std::f64::INFINITY => GeneralF64ParseResult::Infinity,
        Ok(a) => GeneralF64ParseResult::Number(a),
        Err(_) => GeneralF64ParseResult::Invalid,
    }
}

/// Compares two floats, with errors and non-numerics assumed to be -inf.
/// Stops coercing at the first non-numeric char.
/// We explicitly need to convert to f64 in this case.
fn general_numeric_compare(a: GeneralF64ParseResult, b: GeneralF64ParseResult) -> Ordering {
    a.partial_cmp(&b).unwrap()
}

fn get_rand_string() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect::<String>()
}

fn get_hash<T: Hash>(t: &T) -> u64 {
    let mut s: FnvHasher = Default::default();
    t.hash(&mut s);
    s.finish()
}

fn random_shuffle(a: &str, b: &str, x: String) -> Ordering {
    #![allow(clippy::comparison_chain)]
    let salt_slice = x.as_str();

    let da = get_hash(&[a, salt_slice].concat());
    let db = get_hash(&[b, salt_slice].concat());

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

/// Parse the beginning string into a Month, returning Month::Unknown on errors.
fn month_parse(line: &str) -> Month {
    let line = line.trim();

    const MONTHS: [(&str, Month); 12] = [
        ("JAN", Month::January),
        ("FEB", Month::February),
        ("MAR", Month::March),
        ("APR", Month::April),
        ("MAY", Month::May),
        ("JUN", Month::June),
        ("JUL", Month::July),
        ("AUG", Month::August),
        ("SEP", Month::September),
        ("OCT", Month::October),
        ("NOV", Month::November),
        ("DEC", Month::December),
    ];

    for (month_str, month) in &MONTHS {
        if line.is_char_boundary(month_str.len())
            && line[..month_str.len()].eq_ignore_ascii_case(month_str)
        {
            return *month;
        }
    }

    Month::Unknown
}

fn month_compare(a: &str, b: &str) -> Ordering {
    #![allow(clippy::comparison_chain)]
    let ma = month_parse(a);
    let mb = month_parse(b);

    if ma > mb {
        Ordering::Greater
    } else if ma < mb {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn version_parse(a: &str) -> Version {
    let result = Version::parse(a);

    match result {
        Ok(vers_a) => vers_a,
        // Non-version lines parse to 0.0.0
        Err(_e) => Version::parse("0.0.0").unwrap(),
    }
}

fn version_compare(a: &str, b: &str) -> Ordering {
    #![allow(clippy::comparison_chain)]
    let ver_a = version_parse(a);
    let ver_b = version_parse(b);

    // Version::cmp is not implemented; implement comparison directly
    if ver_a > ver_b {
        Ordering::Greater
    } else if ver_a < ver_b {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn remove_nondictionary_chars(s: &str) -> String {
    // According to GNU, dictionary chars are those of ASCII
    // and a blank is a space or a tab
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect::<String>()
}

fn remove_nonprinting_chars(s: &str) -> String {
    // However, GNU says nonprinting chars are more permissive.
    // All of ASCII except control chars ie, escape, newline
    s.chars()
        .filter(|c| c.is_ascii() && !c.is_ascii_control())
        .collect::<String>()
}

fn print_sorted<T: Iterator<Item = Line>>(iter: T, settings: &GlobalSettings) {
    let mut file: Box<dyn Write> = match settings.outfile {
        Some(ref filename) => match File::create(Path::new(&filename)) {
            Ok(f) => Box::new(BufWriter::new(f)) as Box<dyn Write>,
            Err(e) => {
                show_error!("{0}: {1}", filename, e.to_string());
                panic!("Could not open output file");
            }
        },
        None => Box::new(BufWriter::new(stdout())) as Box<dyn Write>,
    };
    if settings.zero_terminated && !settings.debug {
        for line in iter {
            crash_if_err!(1, file.write_all(line.line.as_bytes()));
            crash_if_err!(1, file.write_all("\0".as_bytes()));
        }
    } else {
        for line in iter {
            if !settings.debug {
                crash_if_err!(1, file.write_all(line.line.as_bytes()));
                crash_if_err!(1, file.write_all("\n".as_bytes()));
            } else {
                crash_if_err!(1, line.print_debug(settings, &mut file));
            }
        }
    }
    crash_if_err!(1, file.flush());
}

// from cat.rs
fn open(path: &str) -> Option<(Box<dyn Read>, bool)> {
    if path == "-" {
        let stdin = stdin();
        return Some((Box::new(stdin) as Box<dyn Read>, is_stdin_interactive()));
    }

    match File::open(Path::new(path)) {
        Ok(f) => Some((Box::new(f) as Box<dyn Read>, false)),
        Err(e) => {
            show_error!("{0}: {1}", path, e.to_string());
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_hash() {
        let a = "Ted".to_string();

        assert_eq!(2646829031758483623, get_hash(&a));
    }

    #[test]
    fn test_random_shuffle() {
        let a = "Ted";
        let b = "Ted";
        let c = get_rand_string();

        assert_eq!(Ordering::Equal, random_shuffle(a, b, c));
    }

    #[test]
    fn test_default_compare() {
        let a = "your own";
        let b = "your place";

        assert_eq!(Ordering::Less, default_compare(a, b));
    }

    #[test]
    fn test_month_compare() {
        let a = "JaN";
        let b = "OCt";

        assert_eq!(Ordering::Less, month_compare(a, b));
    }
    #[test]
    fn test_version_compare() {
        let a = "1.2.3-alpha2";
        let b = "1.4.0";

        assert_eq!(Ordering::Less, version_compare(a, b));
    }

    #[test]
    fn test_random_compare() {
        let a = "9";
        let b = "9";
        let c = get_rand_string();

        assert_eq!(Ordering::Equal, random_shuffle(a, b, c));
    }

    #[test]
    fn test_tokenize_fields() {
        let line = "foo bar b    x";
        assert_eq!(tokenize(line, None), vec![0..3, 3..7, 7..9, 9..14,],);
    }

    #[test]
    fn test_tokenize_fields_leading_whitespace() {
        let line = "    foo bar b    x";
        assert_eq!(tokenize(line, None), vec![0..7, 7..11, 11..13, 13..18,]);
    }

    #[test]
    fn test_tokenize_fields_custom_separator() {
        let line = "aaa foo bar b    x";
        assert_eq!(
            tokenize(line, Some('a')),
            vec![0..0, 1..1, 2..2, 3..9, 10..18,]
        );
    }

    #[test]
    fn test_tokenize_fields_trailing_custom_separator() {
        let line = "a";
        assert_eq!(tokenize(line, Some('a')), vec![0..0]);
        let line = "aa";
        assert_eq!(tokenize(line, Some('a')), vec![0..0, 1..1]);
        let line = "..a..a";
        assert_eq!(tokenize(line, Some('a')), vec![0..2, 3..5]);
    }
}
