// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDOs) corasick memchr Roff trunc oset iset CHARCLASS

use std::cmp;
use std::cmp::PartialEq;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write, stdin, stdout};
use std::ops::Range;
use std::path::Path;

use clap::{Arg, ArgAction, Command, value_parser};
use regex::Regex;
use rustc_hash::FxHashSet;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::translate;

/// GNU's regex engine treats a trailing lone backslash as a literal backslash,
/// while the `regex` crate rejects it as an incomplete escape sequence. Double
/// it so that such patterns keep working instead of erroring out.
fn escape_trailing_backslash(pattern: &str) -> String {
    let trailing = pattern.chars().rev().take_while(|&c| c == '\\').count();
    if trailing % 2 == 1 {
        format!("{pattern}\\")
    } else {
        pattern.to_owned()
    }
}

#[derive(Debug, PartialEq)]
enum OutFormat {
    Dumb,
    Roff,
    Tex,
}

#[derive(Debug)]
struct Config {
    format: OutFormat,
    gnu_ext: bool,
    auto_ref: bool,
    input_ref: bool,
    right_ref: bool,
    ignore_case: bool,
    macro_name: String,
    trunc_str: String,
    context_regex: String,
    line_width: usize,
    gap_size: usize,
    sentence_regex: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format: OutFormat::Dumb,
            gnu_ext: true,
            auto_ref: false,
            input_ref: false,
            right_ref: false,
            ignore_case: false,
            macro_name: "xx".to_owned(),
            trunc_str: "/".to_owned(),
            context_regex: "\\w+".to_owned(),
            line_width: 72,
            gap_size: 3,
            sentence_regex: None,
        }
    }
}

fn read_word_filter_file(
    matches: &clap::ArgMatches,
    option: &str,
) -> std::io::Result<FxHashSet<String>> {
    let filename = matches
        .get_one::<OsString>(option)
        .expect("parsing options failed!");
    let reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
        Box::new(stdin())
    } else {
        let file = File::open(Path::new(filename))?;
        Box::new(file)
    });
    let mut words: FxHashSet<String> = FxHashSet::default();
    for word in reader.lines() {
        words.insert(word?);
    }
    Ok(words)
}

/// reads contents of file as unique set of characters to be used with the break-file option
fn read_char_filter_file(
    matches: &clap::ArgMatches,
    option: &str,
) -> std::io::Result<FxHashSet<char>> {
    let filename = matches
        .get_one::<OsString>(option)
        .expect("parsing options failed!");
    let mut reader: Box<dyn Read> = if filename == "-" {
        Box::new(stdin())
    } else {
        let file = File::open(Path::new(filename))?;
        Box::new(file)
    };
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    Ok(buffer.chars().collect())
}

#[derive(Debug)]
struct WordFilter {
    only_specified: bool,
    ignore_specified: bool,
    only_set: FxHashSet<String>,
    ignore_set: FxHashSet<String>,
    word_regex: String,
}

impl WordFilter {
    #[allow(clippy::cognitive_complexity)]
    fn new(matches: &clap::ArgMatches, config: &Config) -> UResult<Self> {
        let (o, oset): (bool, FxHashSet<String>) = if matches.contains_id(options::ONLY_FILE) {
            let words =
                read_word_filter_file(matches, options::ONLY_FILE).map_err_context(String::new)?;
            (true, words)
        } else {
            (false, FxHashSet::default())
        };
        let (i, iset): (bool, FxHashSet<String>) = if matches.contains_id(options::IGNORE_FILE) {
            let words = read_word_filter_file(matches, options::IGNORE_FILE)
                .map_err_context(String::new)?;
            (true, words)
        } else {
            (false, FxHashSet::default())
        };
        let break_set: Option<FxHashSet<char>> = if matches.contains_id(options::BREAK_FILE)
            && !matches.contains_id(options::WORD_REGEXP)
        {
            let mut chars =
                read_char_filter_file(matches, options::BREAK_FILE).map_err_context(String::new)?;
            if !config.gnu_ext {
                // GNU off means at least these are considered
                chars.extend([' ', '\t', '\n']);
            }
            // else only chars found in file
            Some(chars)
        } else {
            // if -W takes precedence or default
            None
        };
        // Ignore empty string regex from cmd-line-args
        let arg_reg: Option<String> = if matches.contains_id(options::WORD_REGEXP) {
            matches
                .get_one::<String>(options::WORD_REGEXP)
                .filter(|v| !v.is_empty())
                .map(|v| escape_trailing_backslash(v))
        } else {
            None
        };
        let reg = match arg_reg {
            Some(arg_reg) => arg_reg,
            None => {
                if let Some(break_set) = break_set {
                    format!(
                        "[^{}]+",
                        regex::escape(&break_set.into_iter().collect::<String>())
                    )
                } else if config.gnu_ext {
                    "\\w+".to_owned()
                } else {
                    "[^ \t\n]+".to_owned()
                }
            }
        };
        Ok(Self {
            only_specified: o,
            ignore_specified: i,
            only_set: oset,
            ignore_set: iset,
            word_regex: reg,
        })
    }
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Ord)]
struct WordRef {
    word: String,
    file_index: usize,
    local_line_nr: usize,
    position: usize,
    position_end: usize,
    char_start: usize,
}

fn get_config(matches: &mut clap::ArgMatches) -> UResult<Config> {
    let mut config = Config::default();
    let err_msg = "parsing options failed";
    if matches.get_flag(options::TRADITIONAL) {
        config.gnu_ext = false;
        config.format = OutFormat::Roff;
        "[^ \t\n]+".clone_into(&mut config.context_regex);
    }
    if let Some(regex) = matches
        .remove_one::<String>(options::SENTENCE_REGEXP)
        .map(|r| escape_trailing_backslash(&r))
    {
        // TODO: The regex crate used here is not fully compatible with GNU's regex implementation.
        // For example, it does not support backreferences.
        // In the future, we might want to switch to the fancy-regex crate for better compatibility.

        // Verify regex is valid and doesn't match empty string
        let re = Regex::new(&regex).map_err(|error| {
            let clean_msg = error
                .to_string()
                .lines()
                .last()
                .unwrap_or("")
                .trim_start_matches("error: ")
                .to_string();

            USimpleError::new(
                1,
                translate!("ptx-error-invalid-regexp", "error" => clean_msg),
            )
        })?;
        if re.is_match("") {
            return Err(USimpleError::new(1, translate!("ptx-error-empty-regexp")));
        }

        config.sentence_regex = Some(regex);
    }
    config.auto_ref = matches.get_flag(options::AUTO_REFERENCE);
    config.input_ref = matches.get_flag(options::REFERENCES);
    config.right_ref = matches.get_flag(options::RIGHT_SIDE_REFS);
    config.ignore_case = matches.get_flag(options::IGNORE_CASE);
    if matches.contains_id(options::MACRO_NAME) {
        matches
            .get_one::<String>(options::MACRO_NAME)
            .expect(err_msg)
            .clone_into(&mut config.macro_name);
    }
    if matches.contains_id(options::FLAG_TRUNCATION) {
        matches
            .get_one::<String>(options::FLAG_TRUNCATION)
            .expect(err_msg)
            .clone_into(&mut config.trunc_str);
    }
    if matches.contains_id(options::WIDTH) {
        config.line_width = *matches.get_one::<u64>(options::WIDTH).unwrap() as usize;
    } else if matches.get_flag(options::TYPESET_MODE) {
        config.line_width = 100;
    }
    if matches.contains_id(options::GAP_SIZE) {
        config.gap_size = *matches.get_one::<u64>(options::GAP_SIZE).unwrap() as usize;
    }
    if let Some(format) = matches.get_one::<String>(options::FORMAT) {
        config.format = match format.as_str() {
            "roff" => OutFormat::Roff,
            "tex" => OutFormat::Tex,
            _ => unreachable!("should be caught by clap"),
        };
    }
    if matches.get_flag(options::format::ROFF) {
        config.format = OutFormat::Roff;
    }
    if matches.get_flag(options::format::TEX) {
        config.format = OutFormat::Tex;
    }
    Ok(config)
}

struct FileContent {
    lines: Vec<String>,
    chars_lines: Vec<Vec<char>>,
}

type FileMap = Vec<(OsString, FileContent)>;

fn read_input(input_files: &[OsString], config: &Config) -> UResult<FileMap> {
    let mut file_map: FileMap = FileMap::new();

    let sentence_splitter = config
        .sentence_regex
        .as_ref()
        .and_then(|re_str| Regex::new(re_str).ok());

    for filename in input_files {
        let mut reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
            Box::new(stdin())
        } else {
            // Attach the quoted filename to the error context if opening fails
            let file =
                File::open(Path::new(filename)).map_err_context(|| filename.quote().to_string())?;
            Box::new(file)
        });

        // Attach the quoted filename context if reading the contents fails
        let lines = read_lines(sentence_splitter.as_ref(), &mut reader)
            .map_err_context(|| filename.quote().to_string())?;

        // Indexing UTF-8 string requires walking from the beginning, which can hurts performance badly when the line is long.
        // Since we will be jumping around the line a lot, we dump the content into a Vec<char>, which can be indexed in constant time.
        let chars_lines: Vec<Vec<char>> = lines.iter().map(|x| x.chars().collect()).collect();
        file_map.push((filename.clone(), FileContent { lines, chars_lines }));
    }
    Ok(file_map)
}

fn read_lines(
    sentence_splitter: Option<&Regex>,
    reader: &mut dyn BufRead,
) -> std::io::Result<Vec<String>> {
    // GNU ptx works on bytes, so invalid UTF-8 input must not be an error.
    // Read everything and replace invalid sequences instead of failing.
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let buffer = String::from_utf8_lossy(&bytes);

    if let Some(re) = sentence_splitter {
        Ok(re
            .split(&buffer)
            .map(|s| s.replace('\n', " ")) // ptx behavior: newlines become spaces inside sentences
            .filter(|s| !s.is_empty()) // remove empty sentences
            .collect())
    } else {
        Ok(buffer.lines().map(ToOwned::to_owned).collect())
    }
}

/// Go through every lines in the input files and record each match occurrence as a `WordRef`.
fn create_word_set(config: &Config, filter: &WordFilter, file_map: &FileMap) -> BTreeSet<WordRef> {
    let Some(reg) = Regex::new(&filter.word_regex).ok() else {
        return BTreeSet::new();
    };
    let Some(ref_reg) = Regex::new(&config.context_regex).ok() else {
        return BTreeSet::new();
    };

    let mut word_set: BTreeSet<WordRef> = BTreeSet::new();
    for (file_index, (_, lines)) in file_map.iter().enumerate() {
        let mut count: usize = 0;
        for line in &lines.lines {
            // if -r, exclude reference from word set
            let (ref_beg, ref_end) = match ref_reg.find(line) {
                Some(x) => (x.start(), x.end()),
                None => (0, 0),
            };
            let mut last_counted_byte = 0;
            let mut char_start = 0;
            // match words with given regex
            for mat in reg.find_iter(line) {
                let (mut beg, end) = (mat.start(), mat.end());

                // GNU-compatible default behavior:
                // with default regexp, keyword must start at first alphabetic char.
                if filter.word_regex == Config::default().context_regex {
                    let matched = &line[beg..end];
                    if let Some(pos) = matched.find(|c: char| c.is_alphabetic()) {
                        beg += pos;
                    } else {
                        continue;
                    }
                }

                if config.input_ref && ((beg, end) == (ref_beg, ref_end)) {
                    continue;
                }
                let mut word = line[beg..end].to_owned();
                if filter.only_specified && !filter.only_set.contains(&word) {
                    continue;
                }
                if filter.ignore_specified && filter.ignore_set.contains(&word) {
                    continue;
                }
                if config.ignore_case {
                    word = word.to_uppercase();
                }

                // Count from the previous match to avoid rescanning the line prefix.
                char_start += line[last_counted_byte..beg].chars().count();
                last_counted_byte = beg;
                word_set.insert(WordRef {
                    word,
                    file_index,
                    local_line_nr: count,
                    position: beg,
                    position_end: end,
                    char_start,
                });
            }
            count += 1;
        }
    }
    word_set
}

fn get_reference(
    config: &Config,
    word_ref: &WordRef,
    filename: &OsStr,
    line: &str,
    context_reg: &Regex,
) -> String {
    if config.auto_ref {
        if filename == "-" {
            format!(":{}", word_ref.local_line_nr + 1)
        } else {
            format!("{}:{}", filename.maybe_quote(), word_ref.local_line_nr + 1)
        }
    } else if config.input_ref {
        let (beg, end) = match context_reg.find(line) {
            Some(x) => (x.start(), x.end()),
            None => (0, 0),
        };
        line[beg..end].to_string()
    } else {
        String::new()
    }
}

/// A line of text, addressed by character rather than by byte.
///
/// ptx measures every field width in characters, so the text arrives already
/// decoded and each method below works on character indices. Fields are carved
/// out as ranges rather than strings because the caller needs to know how much
/// text was left over on either side: that, and not the field contents, is what
/// decides whether a truncation mark is printed.
struct Line<'a>(&'a [char]);

impl Line<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn text(&self, range: Range<usize>) -> String {
        self.0[range].iter().collect()
    }

    fn is_space(&self, index: usize) -> bool {
        self.0[index].is_whitespace()
    }

    /// How many bytes `range` takes once encoded back to UTF-8.
    fn byte_len(&self, range: Range<usize>) -> usize {
        self.0[range].iter().map(|c| c.len_utf8()).sum()
    }

    /// `range` with whitespace dropped from both ends. A range holding nothing
    /// but whitespace collapses to an empty range at `range.start`, so a field
    /// made of whitespace alone contributes nothing and takes up no width.
    fn trim(&self, range: Range<usize>) -> Range<usize> {
        let Range { mut start, mut end } = range;
        while start < end && self.is_space(start) {
            start += 1;
        }
        while range.start < end && self.is_space(end - 1) {
            end -= 1;
        }
        // The two loops cross each other on an all-whitespace range; pull
        // `start` back so the result is an empty range rather than an inverted
        // one, which would panic when used to slice the line.
        start.min(end)..end
    }

    /// Move `range.start` forward past a word it cuts in half, so a field never
    /// opens on a word fragment. A start that already sits on a boundary, or at
    /// the beginning of the line, stays put.
    fn align_start_to_word(&self, range: Range<usize>) -> Range<usize> {
        let Range { mut start, end } = range;
        if start == end || start == 0 || self.is_space(start) || self.is_space(start - 1) {
            return range;
        }
        while start < end && !self.is_space(start) {
            start += 1;
        }
        start..end
    }

    /// The mirror of [`Self::align_start_to_word`]: pull `range.end` back off a
    /// word it cuts in half. An end at the end of the line stays put, since
    /// nothing was cut there.
    fn align_end_to_word(&self, range: Range<usize>) -> Range<usize> {
        let Range { start, mut end } = range;
        if start == end || end == self.len() || self.is_space(end - 1) || self.is_space(end) {
            return range;
        }
        while start < end && !self.is_space(end - 1) {
            end -= 1;
        }
        start..end
    }

    /// At most `width` characters taken from the right-hand end of `range`, on
    /// whole words and without surrounding whitespace. Fields left of the
    /// keyword grow leftwards from a fixed right edge, so this is how they are
    /// filled.
    fn window_ending_at(&self, range: Range<usize>, width: usize) -> Range<usize> {
        let end = self.trim(range).end;
        let start = end.saturating_sub(width);
        self.trim(self.align_start_to_word(start..end))
    }

    /// At most `width` characters taken from the left-hand end of `range`, on
    /// whole words. The start is left exactly where the caller asked for it —
    /// the after field butts against the keyword, so whitespace there is part
    /// of the output and must survive.
    fn window_starting_at(&self, range: Range<usize>, width: usize) -> Range<usize> {
        let start = range.start;
        let end = cmp::min(range.end, start + width);
        let end = self.align_end_to_word(start..end).end;
        start..self.trim(start..end).end
    }
}

/// The keyword of an index entry and the four context fields laid out around
/// it, in the order `tail before KEYWORD after head`: `before` and `after` hold
/// the context adjacent to the keyword, while `tail` and `head` take the text
/// that wraps around the ends of the line when the keyword sits near one of
/// them. Only one of the two wrap-around fields is ever non-empty.
struct Chunks {
    head: String,
    before: String,
    keyword: String,
    after: String,
    tail: String,
}

impl Chunks {
    /// Lay out `keyword` and the text on either side of it within the
    /// configured line width.
    ///
    /// The widths have to agree with GNU's, because where the keyword sits in
    /// the line is part of ptx's output: half the line width for the context on
    /// either side, less the gap between fields, less the truncation marker at
    /// each end and the keyword itself. That layout leaves the arithmetic very
    /// little room to differ.
    fn new(config: &Config, all_before: &[char], keyword: String, all_after: &[char]) -> Self {
        let before_text = Line(all_before);
        let after_text = Line(all_after);

        let half_line_size = config.line_width / 2;
        let max_before_size = half_line_size.saturating_sub(config.gap_size);
        let max_after_size = half_line_size
            .saturating_sub(2 * config.trunc_str.chars().count() + keyword.chars().count() + 1);

        // The two fields next to the keyword, each reaching as far into the
        // context as its half of the line allows.
        let before = before_text.window_ending_at(0..before_text.len(), max_before_size);
        let after = after_text.window_starting_at(0..after_text.len(), max_after_size);

        // Whatever the adjacent field left unused on its half of the line is
        // what the wrap-around field on the opposite side may occupy.
        let max_tail_size = max_before_size
            .saturating_sub(before.len())
            .saturating_sub(config.gap_size);
        let tail_start = after_text.trim(after.end..after_text.len()).start;
        let mut tail = after_text.window_starting_at(tail_start..after_text.len(), max_tail_size);

        // A one-character word at the very end survives the alignment above,
        // because the character before it is a space and so reads as a word
        // boundary. Drop it, so a tail does not trail off mid-phrase.
        if tail.len() > 2 && after_text.is_space(tail.end - 2) && !after_text.is_space(tail.end - 1)
        {
            tail = after_text.trim(tail.start..tail.end - 1);
        }

        // Sizing the head against the after field's *byte* length is not
        // deliberate: every other width here counts characters. The two agree
        // on ASCII input, so the discrepancy only shows on wider characters.
        let max_head_size = max_after_size
            .saturating_sub(after_text.byte_len(after.clone()))
            .saturating_sub(config.gap_size);
        let head = before_text.window_ending_at(0..before.start, max_head_size);

        let mut chunks = Self {
            head: before_text.text(head.clone()),
            before: before_text.text(before.clone()),
            keyword,
            after: after_text.text(after.clone()),
            tail: after_text.text(tail.clone()),
        };

        // TeX output carries no truncation marks.
        if config.format != OutFormat::Tex {
            // A mark goes on the outermost field that actually lost text, so
            // that it appears at the edge of the line rather than in its middle.
            if after.end != after_text.len() {
                if tail.is_empty() {
                    chunks.after.push_str(&config.trunc_str);
                } else if tail.end != after_text.len() {
                    chunks.tail.push_str(&config.trunc_str);
                }
            }
            if before.start != 0 {
                if head.is_empty() {
                    chunks.before.insert_str(0, &config.trunc_str);
                } else if head.start != 0 {
                    chunks.head.insert_str(0, &config.trunc_str);
                }
            }
        }

        chunks
    }
}

/// Escape special characters for TeX.
fn format_tex_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\backslash{}"),
            '$' | '%' | '#' | '&' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '}' | '{' => {
                out.push_str("$\\");
                out.push(c);
                out.push('$');
            }
            _ => out.push(c),
        }
    }
    out
}

fn format_tex_line(
    config: &Config,
    word_ref: &WordRef,
    line: &str,
    chars_line: &[char],
    reference: &str,
) -> String {
    let mut output = String::new();
    write!(output, "\\{} ", config.macro_name).unwrap();
    let chunks = prepare_line_chunks(config, word_ref, line, chars_line, reference);
    write!(
        output,
        "{{{0}}}{{{1}}}{{{2}}}{{{3}}}{{{4}}}",
        format_tex_field(&chunks.tail),
        format_tex_field(&chunks.before),
        format_tex_field(&chunks.keyword),
        format_tex_field(&chunks.after),
        format_tex_field(&chunks.head),
    )
    .unwrap();
    if config.auto_ref || config.input_ref {
        write!(output, "{{{}}}", format_tex_field(reference)).unwrap();
    }
    output
}

/// Put `first` and `second` side by side, separated by a space when neither is
/// empty, so an absent field costs no padding.
fn join_fields(first: String, second: String) -> String {
    match (first.is_empty(), second.is_empty()) {
        (true, _) => second,
        (_, true) => first,
        _ => format!("{first} {second}"),
    }
}

fn format_dumb_line(
    config: &Config,
    word_ref: &WordRef,
    line: &str,
    chars_line: &[char],
    reference: &str,
) -> String {
    let Chunks {
        head,
        before,
        keyword,
        after,
        tail,
    } = prepare_line_chunks(config, word_ref, line, chars_line, reference);

    // Left of the keyword the wrap-around field comes first, right of it last;
    // a space joins them only when both are present.
    let left_part = join_fields(tail, before);
    let right_part = join_fields(after, head);

    // Calculate the width for the left half (before the keyword)
    let half_width = cmp::max(config.line_width / 2, config.gap_size);

    let left_part_len = if left_part.contains(&config.trunc_str) {
        left_part.len() - config.trunc_str.len()
    } else {
        left_part.len()
    };

    // Right-justify the left part within the left half
    let padding = if left_part.len() < half_width {
        half_width - left_part_len
    } else {
        0
    };

    // Build the output line with padding, left part, gap, keyword, and right part
    let mut output = String::new();
    output.push_str(&" ".repeat(padding));
    output.push_str(&left_part);

    // Add gap before keyword
    output.push_str(&" ".repeat(config.gap_size));

    output.push_str(&keyword);
    output.push_str(&right_part);

    // Add reference if needed
    if config.auto_ref || config.input_ref {
        if config.right_ref {
            output.push(' ');
            output.push_str(reference);
        } else {
            output = format!("{reference} {output}");
        }
    }

    output
}

fn format_roff_field(s: &str) -> String {
    s.replace('\"', "\"\"")
}

fn format_roff_line(
    config: &Config,
    word_ref: &WordRef,
    line: &str,
    chars_line: &[char],
    reference: &str,
) -> String {
    let mut output = String::new();
    write!(output, ".{}", config.macro_name).unwrap();
    let chunks = prepare_line_chunks(config, word_ref, line, chars_line, reference);
    write!(
        output,
        " \"{}\" \"{}\" \"{}{}\" \"{}\"",
        format_roff_field(&chunks.tail),
        format_roff_field(&chunks.before),
        format_roff_field(&chunks.keyword),
        format_roff_field(&chunks.after),
        format_roff_field(&chunks.head)
    )
    .unwrap();
    if config.auto_ref || config.input_ref {
        write!(output, " \"{}\"", format_roff_field(reference)).unwrap();
    }
    output
}

/// Split `line` around the keyword `word_ref` points at and lay the pieces out.
fn prepare_line_chunks(
    config: &Config,
    word_ref: &WordRef,
    line: &str,
    chars_line: &[char],
    reference: &str,
) -> Chunks {
    let char_position_end = word_ref.char_start
        + line[word_ref.position..word_ref.position_end]
            .chars()
            .count();

    // Extract the text before the keyword
    let all_before = if config.input_ref {
        let before = &line[..word_ref.position];
        let stripped = before.trim_start_matches(reference).trim_start();
        let trim_offset = before[..before.len() - stripped.len()].chars().count();
        &chars_line[trim_offset..word_ref.char_start]
    } else {
        &chars_line[..word_ref.char_start]
    };

    // Extract the keyword and text after it
    let keyword = line[word_ref.position..word_ref.position_end].to_string();
    let all_after = &chars_line[char_position_end..];

    Chunks::new(config, all_before, keyword, all_after)
}

fn write_traditional_output(
    config: &mut Config,
    file_map: &FileMap,
    words: &BTreeSet<WordRef>,
    output_filename: &OsStr,
) -> UResult<()> {
    let mut writer: BufWriter<Box<dyn Write>> =
        BufWriter::new(if output_filename == OsStr::new("-") {
            Box::new(stdout())
        } else {
            let file = File::create(output_filename)
                .map_err_context(|| output_filename.quote().to_string())?;
            Box::new(file)
        });

    let context_reg = Regex::new(&config.context_regex).unwrap();

    if !config.right_ref {
        let max_ref_len = if config.auto_ref {
            get_auto_max_reference_len(words, file_map)
        } else {
            0
        };

        // Use saturating_sub to prevent panic if the reference is wider than the line width.
        config.line_width = config.line_width.saturating_sub(max_ref_len);
    }

    for word_ref in words {
        let (filename, file_map_value) = &file_map[word_ref.file_index];
        let FileContent { lines, chars_lines } = file_map_value;
        let reference = get_reference(
            config,
            word_ref,
            filename,
            &lines[word_ref.local_line_nr],
            &context_reg,
        );
        let output_line: String = match config.format {
            OutFormat::Tex => format_tex_line(
                config,
                word_ref,
                &lines[word_ref.local_line_nr],
                &chars_lines[word_ref.local_line_nr],
                &reference,
            ),
            OutFormat::Roff => format_roff_line(
                config,
                word_ref,
                &lines[word_ref.local_line_nr],
                &chars_lines[word_ref.local_line_nr],
                &reference,
            ),
            OutFormat::Dumb => format_dumb_line(
                config,
                word_ref,
                &lines[word_ref.local_line_nr],
                &chars_lines[word_ref.local_line_nr],
                &reference,
            ),
        };
        writeln!(writer, "{output_line}")
            .map_err_context(|| translate!("ptx-error-write-failed"))?;
    }

    writer
        .flush()
        .map_err_context(|| translate!("ptx-error-write-failed"))?;

    Ok(())
}

fn get_auto_max_reference_len(words: &BTreeSet<WordRef>, file_map: &FileMap) -> usize {
    //Get the maximum length of the reference field
    let line_num = words
        .iter()
        .map(|w| {
            if w.local_line_nr == 0 {
                1
            } else {
                (w.local_line_nr as f64).log10() as usize + 1
            }
        })
        .max()
        .unwrap_or(0);

    let filename_len = words
        .iter()
        .map(|w| &file_map[w.file_index].0)
        .filter(|filename| *filename != "-")
        .map(|filename| filename.maybe_quote().to_string().len())
        .max()
        .unwrap_or(0);

    // +1 for the colon
    line_num + filename_len + 1
}

mod options {
    pub mod format {
        pub static ROFF: &str = "roff";
        pub static TEX: &str = "tex";
    }

    pub static FILE: &str = "file";
    pub static AUTO_REFERENCE: &str = "auto-reference";
    pub static TRADITIONAL: &str = "traditional";
    pub static FLAG_TRUNCATION: &str = "flag-truncation";
    pub static MACRO_NAME: &str = "macro-name";
    pub static FORMAT: &str = "format";
    pub static RIGHT_SIDE_REFS: &str = "right-side-refs";
    pub static SENTENCE_REGEXP: &str = "sentence-regexp";
    pub static WORD_REGEXP: &str = "word-regexp";
    pub static BREAK_FILE: &str = "break-file";
    pub static IGNORE_CASE: &str = "ignore-case";
    pub static GAP_SIZE: &str = "gap-size";
    pub static IGNORE_FILE: &str = "ignore-file";
    pub static ONLY_FILE: &str = "only-file";
    pub static REFERENCES: &str = "references";
    pub static TYPESET_MODE: &str = "typeset-mode";
    pub static WIDTH: &str = "width";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let mut config = get_config(&mut matches)?;

    let input_files;
    let output_file: OsString;

    let mut files = matches
        .get_many::<OsString>(options::FILE)
        .into_iter()
        .flatten()
        .cloned();

    if config.gnu_ext {
        input_files = {
            let mut files = files.collect::<Vec<_>>();
            if files.is_empty() {
                files.push(OsString::from("-"));
            }
            files
        };
        output_file = OsString::from("-");
    } else {
        input_files = vec![files.next().unwrap_or(OsString::from("-"))];
        output_file = files.next().unwrap_or(OsString::from("-"));
        if let Some(file) = files.next() {
            return Err(UUsageError::new(
                1,
                translate!("ptx-error-extra-operand", "operand" => file.quote()),
            ));
        }
    }

    let word_filter = WordFilter::new(&matches, &config)?;
    let file_map = read_input(&input_files, &config)?;
    let word_set = create_word_set(&config, &word_filter, &file_map);
    write_traditional_output(&mut config, &file_map, &word_set, &output_file)
}

pub fn uu_app() -> Command {
    Command::new("ptx")
        .about(translate!("ptx-about"))
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("ptx"))
        .override_usage(format_usage(&translate!("ptx-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::AUTO_REFERENCE)
                .short('A')
                .long(options::AUTO_REFERENCE)
                .help(translate!("ptx-help-auto-reference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TRADITIONAL)
                .short('G')
                .long(options::TRADITIONAL)
                .help(translate!("ptx-help-traditional"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FLAG_TRUNCATION)
                .short('F')
                .long(options::FLAG_TRUNCATION)
                .help(translate!("ptx-help-flag-truncation"))
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::MACRO_NAME)
                .short('M')
                .long(options::MACRO_NAME)
                .help(translate!("ptx-help-macro-name"))
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::FORMAT)
                .long(options::FORMAT)
                .hide(true)
                .value_parser(["roff", "tex"])
                .overrides_with_all([options::FORMAT, options::format::ROFF, options::format::TEX]),
        )
        .arg(
            Arg::new(options::format::ROFF)
                .short('O')
                .help(translate!("ptx-help-roff"))
                .overrides_with_all([options::FORMAT, options::format::ROFF, options::format::TEX])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::TEX)
                .short('T')
                .help(translate!("ptx-help-tex"))
                .overrides_with_all([options::FORMAT, options::format::ROFF, options::format::TEX])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RIGHT_SIDE_REFS)
                .short('R')
                .long(options::RIGHT_SIDE_REFS)
                .help(translate!("ptx-help-right-side-refs"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SENTENCE_REGEXP)
                .short('S')
                .long(options::SENTENCE_REGEXP)
                .help(translate!("ptx-help-sentence-regexp"))
                .value_name("REGEXP"),
        )
        .arg(
            Arg::new(options::WORD_REGEXP)
                .short('W')
                .long(options::WORD_REGEXP)
                .help(translate!("ptx-help-word-regexp"))
                .value_name("REGEXP"),
        )
        .arg(
            Arg::new(options::BREAK_FILE)
                .short('b')
                .long(options::BREAK_FILE)
                .help(translate!("ptx-help-break-file"))
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .short('f')
                .long(options::IGNORE_CASE)
                .help(translate!("ptx-help-ignore-case"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::GAP_SIZE)
                .short('g')
                .long(options::GAP_SIZE)
                .value_parser(value_parser!(u64).range(1..))
                .help(translate!("ptx-help-gap-size"))
                .value_name("NUMBER"),
        )
        .arg(
            Arg::new(options::IGNORE_FILE)
                .short('i')
                .long(options::IGNORE_FILE)
                .help(translate!("ptx-help-ignore-file"))
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::ONLY_FILE)
                .short('o')
                .long(options::ONLY_FILE)
                .help(translate!("ptx-help-only-file"))
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::REFERENCES)
                .short('r')
                .long(options::REFERENCES)
                .help(translate!("ptx-help-references"))
                .value_name("FILE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TYPESET_MODE)
                .short('t')
                .long(options::TYPESET_MODE)
                .help(translate!("ptx-help-typeset-mode"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .value_parser(value_parser!(u64).range(1..))
                .help(translate!("ptx-help-width"))
                .value_name("NUMBER"),
        )
}

#[cfg(test)]
mod tests {
    use super::{Chunks, Config, Line, OutFormat};

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn trim_drops_only_the_outer_whitespace() {
        let text = chars("  pear  plum  ");
        let line = Line(&text);
        assert_eq!(line.trim(0..14), 2..12);
        // Trimming a window already free of whitespace leaves it alone.
        assert_eq!(line.trim(2..12), 2..12);
        // A window holding nothing but whitespace collapses onto its own
        // start, rather than inverting into a range that cannot be sliced.
        assert_eq!(line.trim(12..14), 12..12);
        assert_eq!(line.trim(0..2), 0..0);
    }

    #[test]
    fn align_start_skips_a_word_it_would_split() {
        let text = chars("pear plum quince");
        let line = Line(&text);
        // Index 2 sits inside "pear", so the window opens at the space that
        // ends it; a later trim moves it onto "plum".
        assert_eq!(line.align_start_to_word(2..16), 4..16);
        // A start already on a boundary, or at the very beginning, stays put.
        assert_eq!(line.align_start_to_word(5..16), 5..16);
        assert_eq!(line.align_start_to_word(0..16), 0..16);
    }

    #[test]
    fn align_end_drops_a_word_it_would_split() {
        let text = chars("pear plum quince");
        let line = Line(&text);
        // Index 12 sits inside "quince", so the window closes on the space
        // that precedes it.
        assert_eq!(line.align_end_to_word(0..12), 0..10);
        // An end at the end of the line cut nothing and is kept.
        assert_eq!(line.align_end_to_word(0..16), 0..16);
    }

    #[test]
    fn windows_take_whole_words_from_the_requested_side() {
        let text = chars("pear plum quince ");
        let line = Line(&text);
        // Seven characters from the right end reach back into "plum", which is
        // therefore dropped; the trailing space goes with it.
        assert_eq!(line.text(line.window_ending_at(0..17, 7)), "quince");
        // Ten from the left end reach into "quince", so it is dropped too.
        assert_eq!(line.text(line.window_starting_at(0..17, 10)), "pear plum");
        // A width wide enough for everything returns the whole trimmed line.
        assert_eq!(
            line.text(line.window_ending_at(0..17, 99)),
            "pear plum quince"
        );
    }

    #[test]
    fn window_starting_at_keeps_leading_whitespace() {
        let text = chars(" pear plum");
        let line = Line(&text);
        // The after field butts against the keyword, so the space that
        // separates them belongs to the field.
        assert_eq!(line.text(line.window_starting_at(0..10, 5)), " pear");
    }

    #[test]
    fn a_wide_line_needs_no_truncation_marks() {
        let config = Config {
            line_width: 60,
            ..Config::default()
        };
        let chunks = Chunks::new(
            &config,
            &chars("pear plum "),
            "nut".to_owned(),
            &chars(" cake tart pie"),
        );
        assert_eq!(chunks.before, "pear plum");
        assert_eq!(chunks.keyword, "nut");
        assert_eq!(chunks.after, " cake tart pie");
        // Nothing wrapped around the ends of the line.
        assert!(chunks.head.is_empty());
        assert!(chunks.tail.is_empty());
    }

    #[test]
    fn a_narrow_line_marks_the_text_it_dropped() {
        let config = Config {
            line_width: 20,
            ..Config::default()
        };
        let chunks = Chunks::new(
            &config,
            &chars("pear plum "),
            "nut".to_owned(),
            &chars(" cake tart pie"),
        );
        // "pear" did not fit, so the before field opens with the mark.
        assert_eq!(chunks.before, "/plum");
        // No word at all fit to the right of the keyword.
        assert_eq!(chunks.after, "/");
        assert!(chunks.head.is_empty());
        assert!(chunks.tail.is_empty());
    }

    #[test]
    fn tex_output_carries_no_truncation_marks() {
        let config = Config {
            line_width: 20,
            format: OutFormat::Tex,
            ..Config::default()
        };
        let chunks = Chunks::new(
            &config,
            &chars("pear plum "),
            "nut".to_owned(),
            &chars(" cake tart pie"),
        );
        assert_eq!(chunks.before, "plum");
        assert!(chunks.after.is_empty());
    }
}
