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

// spell-checker:ignore (misc) HFKJFK Mbdfhn

#[macro_use]
extern crate uucore;

pub mod app;
mod check;
mod chunks;
mod custom_str_cmp;
mod ext_sort;
mod merge;
mod numeric_str_cmp;

use app::{get_app, options};
use custom_str_cmp::custom_str_cmp;
use ext_sort::ext_sort;
use fnv::FnvHasher;
use itertools::Itertools;
use numeric_str_cmp::{numeric_str_cmp, NumInfo, NumInfoParseSettings};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use semver::Version;
use std::cmp::Ordering;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;
use unicode_width::UnicodeWidthStr;
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::InvalidEncodingHandling;

const DECIMAL_PT: char = '.';

const NEGATIVE: char = '-';
const POSITIVE: char = '+';

// Choosing a higher buffer size does not result in performance improvements
// (at least not on my machine). TODO: In the future, we should also take the amount of
// available memory into consideration, instead of relying on this constant only.
const DEFAULT_BUF_SIZE: usize = 1_000_000_000; // 1 GB

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
            SortMode::Numeric => Some('n'),
            SortMode::HumanNumeric => Some('h'),
            SortMode::GeneralNumeric => Some('g'),
            SortMode::Month => Some('M'),
            SortMode::Version => Some('V'),
            SortMode::Random => Some('R'),
            SortMode::Default => None,
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
    output_file: Option<String>,
    stable: bool,
    unique: bool,
    check: bool,
    check_silent: bool,
    salt: String,
    selectors: Vec<FieldSelector>,
    separator: Option<char>,
    threads: String,
    zero_terminated: bool,
    buffer_size: usize,
    tmp_dir: PathBuf,
    compress_prog: Option<String>,
    merge_batch_size: usize,
}

impl GlobalSettings {
    /// Parse a SIZE string into a number of bytes.
    /// A size string comprises an integer and an optional unit.
    /// The unit may be k, K, m, M, g, G, t, T, P, E, Z, Y (powers of 1024), or b which is 1.
    /// Default is K.
    fn parse_byte_count(input: &str) -> Result<usize, ParseSizeError> {
        // GNU sort (8.32)   valid: 1b,        k, K, m, M, g, G, t, T, P, E, Z, Y
        // GNU sort (8.32) invalid:  b, B, 1B,                         p, e, z, y
        const ALLOW_LIST: &[char] = &[
            'b', 'k', 'K', 'm', 'M', 'g', 'G', 't', 'T', 'P', 'E', 'Z', 'Y',
        ];
        let mut size_string = input.trim().to_string();

        if size_string.ends_with(|c: char| ALLOW_LIST.contains(&c) || c.is_digit(10)) {
            // b 1, K 1024 (default)
            if size_string.ends_with(|c: char| c.is_digit(10)) {
                size_string.push('K');
            } else if size_string.ends_with('b') {
                size_string.pop();
            }
            parse_size(&size_string)
        } else {
            Err(ParseSizeError::ParseFailure("invalid suffix".to_string()))
        }
    }

    fn out_writer(&self) -> BufWriter<Box<dyn Write>> {
        match self.output_file {
            Some(ref filename) => match File::create(Path::new(&filename)) {
                Ok(f) => BufWriter::new(Box::new(f) as Box<dyn Write>),
                Err(e) => {
                    show_error!("{0}: {1}", filename, e.to_string());
                    panic!("Could not open output file");
                }
            },
            None => BufWriter::new(Box::new(stdout()) as Box<dyn Write>),
        }
    }
}

impl Default for GlobalSettings {
    fn default() -> GlobalSettings {
        GlobalSettings {
            mode: SortMode::Default,
            debug: false,
            ignore_leading_blanks: false,
            ignore_case: false,
            dictionary_order: false,
            ignore_non_printing: false,
            merge: false,
            reverse: false,
            output_file: None,
            stable: false,
            unique: false,
            check: false,
            check_silent: false,
            salt: String::new(),
            selectors: vec![],
            separator: None,
            threads: String::new(),
            zero_terminated: false,
            buffer_size: DEFAULT_BUF_SIZE,
            tmp_dir: PathBuf::new(),
            compress_prog: None,
            merge_batch_size: 16,
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
                return Err(format!(
                    "options '-{}{}' are incompatible",
                    'd',
                    mode.get_short_name().unwrap()
                ));
            } else if ignore_non_printing {
                return Err(format!(
                    "options '-{}{}' are incompatible",
                    'i',
                    mode.get_short_name().unwrap()
                ));
            }
        }
        Ok(())
    }

    fn set_sort_mode(&mut self, mode: SortMode) -> Result<(), String> {
        if self.mode != SortMode::Default {
            return Err(format!(
                "options '-{}{}' are incompatible",
                self.mode.get_short_name().unwrap(),
                mode.get_short_name().unwrap()
            ));
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

#[derive(Clone, Debug)]
enum NumCache {
    AsF64(GeneralF64ParseResult),
    WithInfo(NumInfo),
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

#[derive(Clone, Debug)]
struct Selection<'a> {
    slice: &'a str,
    num_cache: Option<Box<NumCache>>,
}

type Field = Range<usize>;

#[derive(Clone, Debug)]
pub struct Line<'a> {
    line: &'a str,
    selections: Box<[Selection<'a>]>,
}

impl<'a> Line<'a> {
    fn create(string: &'a str, settings: &GlobalSettings) -> Self {
        let fields = if settings
            .selectors
            .iter()
            .any(|selector| selector.needs_tokens)
        {
            // Only tokenize if we will need tokens.
            Some(tokenize(string, settings.separator))
        } else {
            None
        };

        Line {
            line: string,
            selections: settings
                .selectors
                .iter()
                .filter(|selector| !selector.is_default_selection)
                .map(|selector| selector.get_selection(string, fields.as_deref()))
                .collect(),
        }
    }

    fn print(&self, writer: &mut impl Write, settings: &GlobalSettings) {
        if settings.zero_terminated && !settings.debug {
            crash_if_err!(1, writer.write_all(self.line.as_bytes()));
            crash_if_err!(1, writer.write_all(b"\0"));
        } else if !settings.debug {
            crash_if_err!(1, writer.write_all(self.line.as_bytes()));
            crash_if_err!(1, writer.write_all(b"\n"));
        } else {
            crash_if_err!(1, self.print_debug(settings, writer));
        }
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

        let line = self.line.replace('\t', ">");
        writeln!(writer, "{}", line)?;

        let fields = tokenize(self.line, settings.separator);
        for selector in settings.selectors.iter() {
            let mut selection = selector.get_range(self.line, Some(&fields));
            match selector.settings.mode {
                SortMode::Numeric | SortMode::HumanNumeric => {
                    // find out which range is used for numeric comparisons
                    let (_, num_range) = NumInfo::parse(
                        &self.line[selection.clone()],
                        NumInfoParseSettings {
                            accept_si_units: selector.settings.mode == SortMode::HumanNumeric,
                            ..Default::default()
                        },
                    );
                    let initial_selection = selection.clone();

                    // Shorten selection to num_range.
                    selection.start += num_range.start;
                    selection.end = selection.start + num_range.len();

                    if num_range != (0..0) {
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
                    } else {
                        // This was not a valid number.
                        // Report no match at the first non-whitespace character.
                        let leading_whitespace = self.line[selection.clone()]
                            .find(|c: char| !c.is_whitespace())
                            .unwrap_or(0);
                        selection.start += leading_whitespace;
                        selection.end += leading_whitespace;
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
                        .char_indices()
                        .skip_while(|(_, c)| c.is_whitespace());

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
                    .map_or(true, |selector| selector != &Default::default()))
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
            .ok_or_else(|| format!("invalid key `{}`", key))?;
        let char = field_and_char.next();

        let field = field
            .parse()
            .map_err(|e| format!("failed to parse field index `{}`: {}", field, e))?;
        if field == 0 {
            return Err("field index can not be 0".to_string());
        }

        let char = char.map_or(Ok(default_char_index), |char| {
            char.parse()
                .map_err(|e| format!("failed to parse character index `{}`: {}", char, e))
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
        KeyPosition {
            field: 1,
            char: 1,
            ignore_blanks: false,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
struct FieldSelector {
    from: KeyPosition,
    to: Option<KeyPosition>,
    settings: KeySettings,
    needs_tokens: bool,
    // Whether the selection for each line is going to be the whole line with no NumCache
    is_default_selection: bool,
}

impl Default for FieldSelector {
    fn default() -> Self {
        Self {
            from: Default::default(),
            to: None,
            settings: Default::default(),
            needs_tokens: false,
            is_default_selection: true,
        }
    }
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

    fn parse(key: &str, global_settings: &GlobalSettings) -> Self {
        let mut from_to = key.split(',');
        let (from, from_options) = Self::split_key_options(from_to.next().unwrap());
        let to = from_to.next().map(|to| Self::split_key_options(to));
        let options_are_empty = from_options.is_empty() && matches!(to, None | Some((_, "")));
        crash_if_err!(
            2,
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
            .map_err(|e| format!("failed to parse key `{}`: {}", key, e))
        )
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
                    c => return Err(format!("invalid option: `{}`", c)),
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
            Err("invalid character index 0 for the start position of a field".to_string())
        } else {
            Ok(Self {
                is_default_selection: from.field == 1
                    && from.char == 1
                    && to.is_none()
                    && !matches!(
                        settings.mode,
                        SortMode::Numeric | SortMode::GeneralNumeric | SortMode::HumanNumeric
                    )
                    && !from.ignore_blanks,
                needs_tokens: from.field != 1 || from.char == 0 || to.is_some(),
                from,
                to,
                settings,
            })
        }
    }

    /// Get the selection that corresponds to this selector for the line.
    /// If needs_fields returned false, tokens may be None.
    fn get_selection<'a>(&self, line: &'a str, tokens: Option<&[Field]>) -> Selection<'a> {
        let mut range = &line[self.get_range(line, tokens)];
        let num_cache = if self.settings.mode == SortMode::Numeric
            || self.settings.mode == SortMode::HumanNumeric
        {
            // Parse NumInfo for this number.
            let (info, num_range) = NumInfo::parse(
                range,
                NumInfoParseSettings {
                    accept_si_units: self.settings.mode == SortMode::HumanNumeric,
                    ..Default::default()
                },
            );
            // Shorten the range to what we need to pass to numeric_str_cmp later.
            range = &range[num_range];
            Some(Box::new(NumCache::WithInfo(info)))
        } else if self.settings.mode == SortMode::GeneralNumeric {
            // Parse this number as f64, as this is the requirement for general numeric sorting.
            Some(Box::new(NumCache::AsF64(general_f64_parse(
                &range[get_leading_gen(range)],
            ))))
        } else {
            // This is not a numeric sort, so we don't need a NumCache.
            None
        };
        Selection {
            slice: range,
            num_cache,
        }
    }

    /// Look up the range in the line that corresponds to this selector.
    /// If needs_fields returned false, tokens may be None.
    fn get_range<'a>(&self, line: &'a str, tokens: Option<&[Field]>) -> Range<usize> {
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
                        .char_indices()
                        .find(|(_, c)| !c.is_whitespace())
                        .map_or(line[idx..].len(), |(idx, _)| idx);
                }
                // apply the character index
                idx += line[idx..]
                    .char_indices()
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
                };
                if range.start > range.end {
                    range.end = range.start;
                }
                range
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

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... [FILE]...
Write the sorted concatenation of all FILE(s) to standard output.
Mandatory arguments for long options are mandatory for short options too.
With no FILE, or when FILE is -, read standard input.",
        executable!()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();
    let usage = get_usage();
    let mut settings: GlobalSettings = Default::default();

    let matches = get_app(executable!())
        .usage(usage.as_str())
        .get_matches_from(args);

    settings.debug = matches.is_present(options::DEBUG);

    // check whether user specified a zero terminated list of files for input, otherwise read files from args
    let mut files: Vec<String> = if matches.is_present(options::FILES0_FROM) {
        let files0_from: Vec<String> = matches
            .values_of(options::FILES0_FROM)
            .map(|v| v.map(ToString::to_string).collect())
            .unwrap_or_default();

        let mut files = Vec::new();
        for path in &files0_from {
            let reader = open(path.as_str());
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
            .values_of(options::FILES)
            .map(|v| v.map(ToString::to_string).collect())
            .unwrap_or_default()
    };

    settings.mode = if matches.is_present(options::modes::HUMAN_NUMERIC)
        || matches.value_of(options::modes::SORT) == Some("human-numeric")
    {
        SortMode::HumanNumeric
    } else if matches.is_present(options::modes::MONTH)
        || matches.value_of(options::modes::SORT) == Some("month")
    {
        SortMode::Month
    } else if matches.is_present(options::modes::GENERAL_NUMERIC)
        || matches.value_of(options::modes::SORT) == Some("general-numeric")
    {
        SortMode::GeneralNumeric
    } else if matches.is_present(options::modes::NUMERIC)
        || matches.value_of(options::modes::SORT) == Some("numeric")
    {
        SortMode::Numeric
    } else if matches.is_present(options::modes::VERSION)
        || matches.value_of(options::modes::SORT) == Some("version")
    {
        SortMode::Version
    } else if matches.is_present(options::modes::RANDOM)
        || matches.value_of(options::modes::SORT) == Some("random")
    {
        settings.salt = get_rand_string();
        SortMode::Random
    } else {
        SortMode::Default
    };

    settings.dictionary_order = matches.is_present(options::DICTIONARY_ORDER);
    settings.ignore_non_printing = matches.is_present(options::IGNORE_NONPRINTING);
    if matches.is_present(options::PARALLEL) {
        // "0" is default - threads = num of cores
        settings.threads = matches
            .value_of(options::PARALLEL)
            .map(String::from)
            .unwrap_or_else(|| "0".to_string());
        env::set_var("RAYON_NUM_THREADS", &settings.threads);
    }

    settings.buffer_size = matches
        .value_of(options::BUF_SIZE)
        .map_or(DEFAULT_BUF_SIZE, |s| {
            GlobalSettings::parse_byte_count(s)
                .unwrap_or_else(|e| crash!(2, "{}", format_error_message(e, s, options::BUF_SIZE)))
        });

    settings.tmp_dir = matches
        .value_of(options::TMP_DIR)
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir);

    settings.compress_prog = matches.value_of(options::COMPRESS_PROG).map(String::from);

    if let Some(n_merge) = matches.value_of(options::BATCH_SIZE) {
        settings.merge_batch_size = n_merge
            .parse()
            .unwrap_or_else(|_| crash!(2, "invalid --batch-size argument '{}'", n_merge));
    }

    settings.zero_terminated = matches.is_present(options::ZERO_TERMINATED);
    settings.merge = matches.is_present(options::MERGE);

    settings.check = matches.is_present(options::check::CHECK);
    if matches.is_present(options::check::CHECK_SILENT)
        || matches!(
            matches.value_of(options::check::CHECK),
            Some(options::check::SILENT) | Some(options::check::QUIET)
        )
    {
        settings.check_silent = true;
        settings.check = true;
    };

    settings.ignore_case = matches.is_present(options::IGNORE_CASE);

    settings.ignore_leading_blanks = matches.is_present(options::IGNORE_LEADING_BLANKS);

    settings.output_file = matches.value_of(options::OUTPUT).map(String::from);
    settings.reverse = matches.is_present(options::REVERSE);
    settings.stable = matches.is_present(options::STABLE);
    settings.unique = matches.is_present(options::UNIQUE);

    if files.is_empty() {
        /* if no file, default to stdin */
        files.push("-".to_owned());
    } else if settings.check && files.len() != 1 {
        crash!(1, "extra operand `{}' not allowed with -c", files[1])
    }

    if let Some(arg) = matches.args.get(options::SEPARATOR) {
        let separator = arg.vals[0].to_string_lossy();
        let separator = separator;
        if separator.len() != 1 {
            crash!(1, "separator must be exactly one character long");
        }
        settings.separator = Some(separator.chars().next().unwrap())
    }

    if matches.is_present(options::KEY) {
        for key in &matches.args[options::KEY].vals {
            settings
                .selectors
                .push(FieldSelector::parse(&key.to_string_lossy(), &settings));
        }
    }

    if !matches.is_present(options::KEY) {
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

    exec(&files, &settings)
}

fn output_sorted_lines<'a>(iter: impl Iterator<Item = &'a Line<'a>>, settings: &GlobalSettings) {
    if settings.unique {
        print_sorted(
            iter.dedup_by(|a, b| compare_by(a, b, settings) == Ordering::Equal),
            settings,
        );
    } else {
        print_sorted(iter, settings);
    }
}

fn exec(files: &[String], settings: &GlobalSettings) -> i32 {
    if settings.merge {
        let mut file_merger = merge::merge_with_file_limit(files.iter().map(open), settings);
        file_merger.write_all(settings);
    } else if settings.check {
        if files.len() > 1 {
            crash!(1, "only one file allowed with -c");
        }
        return check::check(files.first().unwrap(), settings);
    } else {
        let mut lines = files.iter().map(open);

        ext_sort(&mut lines, settings);
    }
    0
}

fn sort_by<'a>(unsorted: &mut Vec<Line<'a>>, settings: &GlobalSettings) {
    if settings.stable || settings.unique {
        unsorted.par_sort_by(|a, b| compare_by(a, b, settings))
    } else {
        unsorted.par_sort_unstable_by(|a, b| compare_by(a, b, settings))
    }
}

fn compare_by<'a>(a: &Line<'a>, b: &Line<'a>, global_settings: &GlobalSettings) -> Ordering {
    let mut idx = 0;
    for selector in &global_settings.selectors {
        let mut _selections = None;
        let (a_selection, b_selection) = if selector.is_default_selection {
            // We can select the whole line.
            // We have to store the selections outside of the if-block so that they live long enough.
            _selections = Some((
                Selection {
                    slice: a.line,
                    num_cache: None,
                },
                Selection {
                    slice: b.line,
                    num_cache: None,
                },
            ));
            // Unwrap the selections again, and return references to them.
            (
                &_selections.as_ref().unwrap().0,
                &_selections.as_ref().unwrap().1,
            )
        } else {
            let selections = (&a.selections[idx], &b.selections[idx]);
            idx += 1;
            selections
        };
        let a_str = a_selection.slice;
        let b_str = b_selection.slice;
        let settings = &selector.settings;

        let cmp: Ordering = match settings.mode {
            SortMode::Random => random_shuffle(a_str, b_str, &global_settings.salt),
            SortMode::Numeric | SortMode::HumanNumeric => numeric_str_cmp(
                (a_str, a_selection.num_cache.as_ref().unwrap().as_num_info()),
                (b_str, b_selection.num_cache.as_ref().unwrap().as_num_info()),
            ),
            SortMode::GeneralNumeric => general_numeric_compare(
                a_selection.num_cache.as_ref().unwrap().as_f64(),
                b_selection.num_cache.as_ref().unwrap().as_f64(),
            ),
            SortMode::Month => month_compare(a_str, b_str),
            SortMode::Version => version_compare(a_str, b_str),
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

// This function cleans up the initial comparison done by leading_num_common for a general numeric compare.
// In contrast to numeric compare, GNU general numeric/FP sort *should* recognize positive signs and
// scientific notation, so we strip those lines only after the end of the following numeric string.
// For example, 5e10KFD would be 5e10 or 5x10^10 and +10000HFKJFK would become 10000.
fn get_leading_gen(input: &str) -> Range<usize> {
    let trimmed = input.trim_start();
    let leading_whitespace_len = input.len() - trimmed.len();

    // check for inf, -inf and nan
    for allowed_prefix in &["inf", "-inf", "nan"] {
        if trimmed.is_char_boundary(allowed_prefix.len())
            && trimmed[..allowed_prefix.len()].eq_ignore_ascii_case(allowed_prefix)
        {
            return leading_whitespace_len..(leading_whitespace_len + allowed_prefix.len());
        }
    }
    // Make this iter peekable to see if next char is numeric
    let mut char_indices = itertools::peek_nth(trimmed.char_indices());

    let first = char_indices.peek();

    if matches!(first, Some((_, NEGATIVE)) | Some((_, POSITIVE))) {
        char_indices.next();
    }

    let mut had_e_notation = false;
    let mut had_decimal_pt = false;
    while let Some((idx, c)) = char_indices.next() {
        if c.is_ascii_digit() {
            continue;
        }
        if c == DECIMAL_PT && !had_decimal_pt && !had_e_notation {
            had_decimal_pt = true;
            continue;
        }
        if (c == 'e' || c == 'E') && !had_e_notation {
            // we can only consume the 'e' if what follow is either a digit, or a sign followed by a digit.
            if let Some(&(_, next_char)) = char_indices.peek() {
                if (next_char == '+' || next_char == '-')
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
    leading_whitespace_len..input.len()
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
enum GeneralF64ParseResult {
    Invalid,
    NaN,
    NegInfinity,
    Number(f64),
    Infinity,
}

/// Parse the beginning string into a GeneralF64ParseResult.
/// Using a GeneralF64ParseResult instead of f64 is necessary to correctly order floats.
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

fn random_shuffle(a: &str, b: &str, salt: &str) -> Ordering {
    #![allow(clippy::comparison_chain)]

    let da = get_hash(&[a, salt].concat());
    let db = get_hash(&[b, salt].concat());

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

fn print_sorted<'a, T: Iterator<Item = &'a Line<'a>>>(iter: T, settings: &GlobalSettings) {
    let mut writer = settings.out_writer();
    for line in iter {
        line.print(&mut writer, settings);
    }
}

// from cat.rs
fn open(path: impl AsRef<OsStr>) -> Box<dyn Read + Send> {
    let path = path.as_ref();
    if path == "-" {
        let stdin = stdin();
        return Box::new(stdin) as Box<dyn Read + Send>;
    }

    match File::open(Path::new(path)) {
        Ok(f) => Box::new(f) as Box<dyn Read + Send>,
        Err(e) => {
            crash!(2, "cannot read: {0:?}: {1}", path, e);
        }
    }
}

fn format_error_message(error: ParseSizeError, s: &str, option: &str) -> String {
    // NOTE:
    // GNU's sort echos affected flag, -S or --buffer-size, depending user's selection
    // GNU's sort does distinguish between "invalid (suffix in) argument"
    match error {
        ParseSizeError::ParseFailure(_) => format!("invalid --{} argument '{}'", option, s),
        ParseSizeError::SizeTooBig(_) => format!("--{} argument '{}' too large", option, s),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_hash() {
        let a = "Ted".to_string();

        assert_eq!(2_646_829_031_758_483_623, get_hash(&a));
    }

    #[test]
    fn test_random_shuffle() {
        let a = "Ted";
        let b = "Ted";
        let c = get_rand_string();

        assert_eq!(Ordering::Equal, random_shuffle(a, b, &c));
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

        assert_eq!(Ordering::Equal, random_shuffle(a, b, &c));
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

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_line_size() {
        // We should make sure to not regress the size of the Line struct because
        // it is unconditional overhead for every line we sort.
        assert_eq!(std::mem::size_of::<Line>(), 32);
        // These are the fields of Line:
        assert_eq!(std::mem::size_of::<&str>(), 16);
        assert_eq!(std::mem::size_of::<Box<[Selection]>>(), 16);

        // How big is a selection? Constant cost all lines pay when we need selections.
        assert_eq!(std::mem::size_of::<Selection>(), 24);
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
            #[cfg(not(target_pointer_width = "128"))]
            assert!(GlobalSettings::parse_byte_count(input).is_err());
        }

        // ParseFailure
        let invalid_input = ["nonsense", "1B", "B", "b", "p", "e", "z", "y"];
        for input in &invalid_input {
            assert!(GlobalSettings::parse_byte_count(input).is_err());
        }
    }
}
