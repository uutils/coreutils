// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDOs) corasick memchr Roff trunc oset iset CHARCLASS

use std::cmp;
use std::cmp::PartialEq;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write, stdin, stdout};
use std::num::ParseIntError;
use std::path::Path;

use clap::{Arg, ArgAction, Command};
use regex::Regex;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, UUsageError};
use uucore::format_usage;
use uucore::translate;

struct PtxResult {
    words: BTreeSet<WordRef>,
    max_word_len: usize,
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
    word_regex: Option<String>,
    line_width: usize,
    gap_size: usize,
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
            context_regex: r#"[.?!][])"'}]*(?:\t|  |$)[ \t\n]*"#.to_owned(),
            word_regex: None,
            line_width: 72,
            gap_size: 3,
        }
    }
}

fn read_word_filter_file(
    matches: &clap::ArgMatches,
    option: &str,
) -> std::io::Result<HashSet<String>> {
    let filename = matches
        .get_one::<OsString>(option)
        .expect("parsing options failed!");
    let reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
        Box::new(stdin())
    } else {
        let file = File::open(Path::new(filename))?;
        Box::new(file)
    });
    let mut words: HashSet<String> = HashSet::new();
    for word in reader.lines() {
        words.insert(word?);
    }
    Ok(words)
}

/// reads contents of file as unique set of characters to be used with the break-file option
fn read_char_filter_file(
    matches: &clap::ArgMatches,
    option: &str,
) -> std::io::Result<HashSet<char>> {
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
    only_set: HashSet<String>,
    ignore_set: HashSet<String>,
    word_regex: String,
}

impl WordFilter {
    #[allow(clippy::cognitive_complexity)]
    fn new(matches: &clap::ArgMatches, config: &Config) -> UResult<Self> {
        let (o, oset): (bool, HashSet<String>) = if matches.contains_id(options::ONLY_FILE) {
            let words =
                read_word_filter_file(matches, options::ONLY_FILE).map_err_context(String::new)?;
            (true, words)
        } else {
            (false, HashSet::new())
        };
        let (i, iset): (bool, HashSet<String>) = if matches.contains_id(options::IGNORE_FILE) {
            let words = read_word_filter_file(matches, options::IGNORE_FILE)
                .map_err_context(String::new)?;
            (true, words)
        } else {
            (false, HashSet::new())
        };
        let break_set: Option<HashSet<char>> = if matches.contains_id(options::BREAK_FILE)
            && !matches.contains_id(options::WORD_REGEXP)
        {
            let chars =
                read_char_filter_file(matches, options::BREAK_FILE).map_err_context(String::new)?;
            let mut hs: HashSet<char> = if config.gnu_ext {
                HashSet::new() // really only chars found in file
            } else {
                // GNU off means at least these are considered
                [' ', '\t', '\n'].iter().copied().collect()
            };
            hs.extend(chars);
            Some(hs)
        } else {
            // if -W takes precedence or default
            None
        };
        // Ignore empty string regex from cmd-line-args
        let arg_reg: Option<String> = if matches.contains_id(options::WORD_REGEXP) {
            match matches.get_one::<String>(options::WORD_REGEXP) {
                Some(v) => {
                    if v.is_empty() {
                        None
                    } else {
                        Some(v.to_owned())
                    }
                }
                None => None,
            }
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

/// Original, lightweight WordRef for line mode (fast path)
#[derive(Debug, PartialOrd, PartialEq, Eq, Ord)]
struct LineWordRef {
    word: String,
    global_line_nr: usize,
    local_line_nr: usize,
    position: usize,
    position_end: usize,
    filename: OsString,
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Ord)]
struct StreamWordRef {
    word: String,
    absolute_position: usize, // The absolute start position of the word in full_content
    absolute_position_end: usize,
    context_start: usize, // The absolute start position of the context to which this word belongs
    context_end: usize,

    // Mainly used for the -A option and obtaining original lines
    filename: OsString,
    global_line_nr: usize,
    local_line_nr: usize,
}

#[derive(Debug, Eq)]
enum WordRef {
    Line(LineWordRef),
    Stream(StreamWordRef),
}

impl WordRef {
    fn filename(&self) -> &OsString {
        match self {
            Self::Line(r) => &r.filename,
            Self::Stream(r) => &r.filename,
        }
    }
    fn local_line_nr(&self) -> usize {
        match self {
            Self::Line(r) => r.local_line_nr,
            Self::Stream(r) => r.local_line_nr,
        }
    }
    fn word(&self) -> &String {
        match self {
            Self::Line(r) => &r.word,
            Self::Stream(r) => &r.word,
        }
    }

    fn unique_position(&self) -> (usize, usize) {
        match self {
            Self::Line(r) => (r.global_line_nr, r.position),
            Self::Stream(r) => (r.global_line_nr, r.absolute_position),
        }
    }
}

impl PartialEq for WordRef {
    fn eq(&self, other: &Self) -> bool {
        // We consider two elements to be identical only when both their words and positions are the same
        self.word() == other.word() && self.unique_position() == other.unique_position()
    }
}

impl Ord for WordRef {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.word().cmp(other.word()) {
            // If the words are the same, sort them by their order of appearance
            cmp::Ordering::Equal => self.unique_position().cmp(&other.unique_position()),
            other_ordering => other_ordering,
        }
    }
}

impl PartialOrd for WordRef {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
#[derive(Debug, Error)]
enum PtxError {
    #[error("{}", translate!("ptx-error-dumb-format"))]
    DumbFormat,

    #[error("{}", translate!("ptx-error-not-implemented", "feature" => (*.0)))]
    NotImplemented(&'static str),

    #[error("{0}")]
    ParseError(ParseIntError),
}

impl UError for PtxError {}

fn get_config(matches: &clap::ArgMatches) -> UResult<Config> {
    let mut config = Config::default();
    let err_msg = "parsing options failed";

    config.input_ref = matches.get_flag(options::REFERENCES);

    if matches.get_flag(options::TRADITIONAL) {
        config.gnu_ext = false;
        config.format = OutFormat::Roff;
        "\n".clone_into(&mut config.context_regex);
    } else {
        if config.input_ref {
            "\n".clone_into(&mut config.context_regex);
        }
        return Err(PtxError::NotImplemented("GNU extensions").into());
    }

    config.auto_ref = matches.get_flag(options::AUTO_REFERENCE);
    if matches.contains_id(options::SENTENCE_REGEXP) {
        matches
            .get_one::<String>(options::SENTENCE_REGEXP)
            .expect(err_msg)
            .clone_into(&mut config.context_regex);
    }
    config.word_regex = matches.get_one::<String>(options::WORD_REGEXP).cloned();
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
        config.line_width = matches
            .get_one::<String>(options::WIDTH)
            .expect(err_msg)
            .parse()
            .map_err(PtxError::ParseError)?;
    }
    if matches.contains_id(options::GAP_SIZE) {
        config.gap_size = matches
            .get_one::<String>(options::GAP_SIZE)
            .expect(err_msg)
            .parse()
            .map_err(PtxError::ParseError)?;
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
    offset: usize,
    full_content: String,
}

type FileMap = HashMap<OsString, FileContent>;

fn read_input(input_files: &[OsString]) -> std::io::Result<FileMap> {
    let mut file_map: FileMap = HashMap::new();
    let mut offset: usize = 0;
    for filename in input_files {
        let mut reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
            Box::new(stdin())
        } else {
            let file = File::open(Path::new(filename))?;
            Box::new(file)
        });
        let mut full_content = String::new();
        reader.read_to_string(&mut full_content)?;

        let lines: Vec<String> = full_content.lines().map(String::from).collect();
        // Indexing UTF-8 string requires walking from the beginning, which can hurts performance badly when the line is long.
        // Since we will be jumping around the line a lot, we dump the content into a Vec<char>, which can be indexed in constant time.
        let chars_lines: Vec<Vec<char>> = lines.iter().map(|x| x.chars().collect()).collect();
        let size = lines.len();
        file_map.insert(
            filename.clone(),
            FileContent {
                lines,
                chars_lines,
                offset,
                full_content,
            },
        );
        offset += size;
    }
    Ok(file_map)
}

/// Scans input files to find and record every keyword occurrence as a `WordRef`.
///
/// Depending on the configuration, this function operates in one of two modes:
/// 1.  **Line Mode**: Processes each physical line of the input individually.
/// 2.  **Stream Mode**: Treats the entire file as a single stream and chunks it by context boundaries (e.g., sentences).
///     Processes input files to identify and record every keyword occurrence as a `WordRef`.
///
/// This function is the core of the keyword indexing logic and operates in one of two modes,
/// determined by the active `context_regex`:
///
/// * **Line Mode** (when `context_regex` is `\n`): The function iterates through each physical
///   line of the input, finding all keywords within that line. It uses a lightweight `LineWordRef`.
///
/// * **Stream Mode** (when `context_regex` is not `\n`): The function treats the entire file
///   content as a single stream, splitting it into logical contexts (e.g., sentences).
///   It uses a more detailed `StreamWordRef`.
fn create_word_set(config: &Config, filter: &WordFilter, file_map: &FileMap) -> PtxResult {
    let word_reg = Regex::new(&filter.word_regex).unwrap();
    let mut word_set: BTreeSet<WordRef> = BTreeSet::new();
    let mut max_word_len = 0;

    let is_stream_mode = config.context_regex != "\n";

    if is_stream_mode {
        let context_reg = Regex::new(&config.context_regex).unwrap();
        let mut global_line_offset = 0;

        for (file, file_content) in file_map {
            let full_content_str = &file_content.full_content;
            // Precompute the starting index of each row to facilitate quick location of line numbers later.
            let line_starts: Vec<usize> = {
                let mut starts = vec![0];
                starts.extend(full_content_str.match_indices('\n').map(|(i, _)| i + 1));
                starts
            };

            let mut current_pos = 0;

            while current_pos < full_content_str.len() {
                let search_slice = &full_content_str[current_pos..];
                let next_boundary = context_reg.find(search_slice);
                let context_end =
                    next_boundary.map_or(full_content_str.len(), |m| current_pos + m.end());

                let mut context_start_offset = current_pos;
                let mut context_slice = &full_content_str[current_pos..context_end];

                //Context contraction
                let is_start_of_line = line_starts.binary_search(&context_start_offset).is_ok();
                if config.input_ref && is_start_of_line {
                    if let Some(ref_end_relative) = context_slice.find(char::is_whitespace) {
                        let content_start_relative = context_slice[ref_end_relative..]
                            .find(|c: char| !c.is_whitespace())
                            .map_or(context_slice.len(), |i| ref_end_relative + i);
                        context_slice = &context_slice[content_start_relative..];
                        context_start_offset += content_start_relative;
                    } else {
                        context_slice = "";
                    }
                }

                for word_match in word_reg.find_iter(context_slice) {
                    let word_str = word_match.as_str();
                    max_word_len = cmp::max(max_word_len, word_str.len());
                    if filter.only_specified && !filter.only_set.contains(word_str) {
                        continue;
                    }
                    if filter.ignore_specified && filter.ignore_set.contains(word_str) {
                        continue;
                    }
                    let final_word = if config.ignore_case {
                        word_str.to_uppercase()
                    } else {
                        word_str.to_owned()
                    };

                    let absolute_word_pos = context_start_offset + word_match.start();

                    // Keyword filtering: if the keyword is within the reference area, skip it
                    // Must be used in conjunction with the context contraction above; omitting either of the two processing logics will cause an error
                    if config.input_ref {
                        let line_nr = line_starts
                            .binary_search(&absolute_word_pos)
                            .unwrap_or_else(|i| i - 1);

                        let physical_line = file_content.lines[line_nr].as_str();
                        let ref_end = physical_line
                            .find(char::is_whitespace)
                            .unwrap_or(physical_line.len());
                        // Calculate the relative position of the word within the physical line.
                        let word_pos_in_line = absolute_word_pos - line_starts[line_nr];
                        if word_pos_in_line < ref_end {
                            // Skip keywords within the reference area
                            continue;
                        }
                    }

                    let local_line_nr = line_starts
                        .binary_search(&absolute_word_pos)
                        .unwrap_or_else(|i| i - 1);

                    word_set.insert(WordRef::Stream(StreamWordRef {
                        word: final_word,
                        absolute_position: absolute_word_pos,
                        absolute_position_end: context_start_offset + word_match.end(),
                        context_start: context_start_offset,
                        context_end,
                        filename: file.clone(),
                        global_line_nr: global_line_offset + local_line_nr,
                        local_line_nr,
                    }));
                }
                current_pos = context_end;
            }
            global_line_offset += file_content.lines.len();
        }
    } else {
        for (file, file_content) in file_map {
            for (count, line) in file_content.lines.iter().enumerate() {
                let (ref_beg, ref_end) = if config.input_ref {
                    let end = line.find(char::is_whitespace).unwrap_or(line.len());
                    (0, end)
                } else {
                    (0, 0)
                };

                for mat in word_reg.find_iter(line) {
                    let (beg, end) = (mat.start(), mat.end());
                    if config.input_ref && (beg, end) == (ref_beg, ref_end) {
                        continue;
                    }

                    let word_str = &line[beg..end];
                    max_word_len = cmp::max(max_word_len, word_str.len());
                    if filter.only_specified && !filter.only_set.contains(word_str) {
                        continue;
                    }
                    if filter.ignore_specified && filter.ignore_set.contains(word_str) {
                        continue;
                    }
                    let final_word = if config.ignore_case {
                        word_str.to_uppercase()
                    } else {
                        word_str.to_owned()
                    };

                    word_set.insert(WordRef::Line(LineWordRef {
                        word: final_word,
                        filename: file.clone(),
                        global_line_nr: file_content.offset + count,
                        local_line_nr: count,
                        position: beg,
                        position_end: end,
                    }));
                }
            }
        }
    }

    PtxResult {
        words: word_set,
        max_word_len,
    }
}

fn get_reference(config: &Config, word_ref: &WordRef, line: &str) -> String {
    if config.auto_ref {
        if word_ref.filename() == "-" {
            format!(":{}", word_ref.local_line_nr() + 1)
        } else {
            format!(
                "{}:{}",
                word_ref.filename().maybe_quote(),
                word_ref.local_line_nr() + 1
            )
        }
    } else if config.input_ref {
        let end = line.find(char::is_whitespace).unwrap_or(line.len());
        line[..end].to_string()
    } else {
        String::new()
    }
}

fn assert_str_integrity(s: &[char], beg: usize, end: usize) {
    assert!(beg <= end);
    assert!(end <= s.len());
}

fn trim_broken_word_left(s: &[char], beg: usize, end: usize) -> usize {
    assert_str_integrity(s, beg, end);
    if beg == end || beg == 0 || s[beg].is_whitespace() || s[beg - 1].is_whitespace() {
        return beg;
    }
    let mut b = beg;
    while b < end && !s[b].is_whitespace() {
        b += 1;
    }
    b
}

fn trim_broken_word_right(s: &[char], beg: usize, end: usize) -> usize {
    assert_str_integrity(s, beg, end);
    if beg == end || end == s.len() || s[end - 1].is_whitespace() || s[end].is_whitespace() {
        return end;
    }
    let mut e = end;
    while beg < e && !s[e - 1].is_whitespace() {
        e -= 1;
    }
    e
}

fn trim_idx(s: &[char], beg: usize, end: usize) -> (usize, usize) {
    assert_str_integrity(s, beg, end);
    let mut b = beg;
    let mut e = end;
    while b < e && s[b].is_whitespace() {
        b += 1;
    }
    while beg < e && s[e - 1].is_whitespace() {
        e -= 1;
    }
    (b, e)
}

fn get_output_chunks(
    all_before: &[char],
    keyword: &str,
    all_after: &[char],
    head_has_truncated: bool,
    config: &Config,
) -> (String, String, String, String) {
    // Chunk size logics are mostly copied from the GNU ptx source.
    // https://github.com/MaiZure/coreutils-8.3/blob/master/src/ptx.c#L1234
    let half_line_size = config.line_width / 2;
    let max_before_size = cmp::max(half_line_size as isize - config.gap_size as isize, 0) as usize;
    let max_after_size = cmp::max(
        half_line_size as isize
            - (2 * config.trunc_str.len()) as isize
            - keyword.len() as isize
            - 1,
        0,
    ) as usize;

    // Allocate plenty space for all the chunks.
    let mut head = String::with_capacity(half_line_size);
    let mut before = String::with_capacity(half_line_size);
    let mut after = String::with_capacity(half_line_size);
    let mut tail = String::with_capacity(half_line_size);

    // the before chunk

    // trim whitespace away from all_before to get the index where the before chunk should end.
    let (_, before_end) = trim_idx(all_before, 0, all_before.len());

    // First calculate a theoretical starting point
    let initial_before_beg = cmp::max(before_end as isize - max_before_size as isize, 0) as usize;

    let before_is_truncated = initial_before_beg > 0
        && all_before[..initial_before_beg]
            .iter()
            .any(|c| !c.is_whitespace());

    let before_beg = trim_broken_word_left(all_before, initial_before_beg, before_end);

    // trim away white space.
    let (before_beg, before_end) = trim_idx(all_before, before_beg, before_end);

    // and get the string.
    // Replace all whitespace with a single space.
    let before_str: String = all_before[before_beg..before_end]
        .iter()
        .map(|&c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    before.push_str(&before_str);
    assert!(max_before_size >= before.len());

    // the after chunk

    // must be no longer than the minimum between the max size and the total available string.
    let after_end = cmp::min(max_after_size, all_after.len());
    // in case that falls in the middle of a word, trim away the word.
    let after_end = trim_broken_word_right(all_after, 0, after_end);
    let after_is_truncated = after_end != all_after.len();
    // trim away white space.
    let (_, after_end) = trim_idx(all_after, 0, after_end);

    // and get the string
    let after_str: String = all_after[0..after_end]
        .iter()
        .map(|&c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    after.push_str(&after_str);
    assert!(max_after_size >= after.len());

    // the tail chunk

    let before_len_for_calc = if before.is_empty() {
        // If the `before` chunk is empty, its effective length for the formula
        // depends on whether the character that was originally just before the keyword was whitespace.
        // all_before ends just before the keyword. If it's not empty and its last char is space,
        // then the C code quirk would result in a -1 length.
        if !all_before.is_empty() && all_before[all_before.len() - 1].is_whitespace() {
            -1
        } else {
            0
        }
    } else {
        before.len() as isize
    };

    let max_tail_size = cmp::max(
        max_before_size as isize - before_len_for_calc - config.gap_size as isize,
        0,
    ) as usize;

    let (tail_beg_untrimmed, _) = trim_idx(all_after, after_end, all_after.len());
    let available_tail_slice = &all_after[tail_beg_untrimmed..];

    let final_tail_end = if max_tail_size > available_tail_slice.len() {
        all_after.len()
    } else {
        // Within the available space, search backward from the end to find the boundary (whitespace character) of the last word.
        let fitting_slice = &available_tail_slice[..max_tail_size];

        match fitting_slice.iter().rposition(|&c| c.is_whitespace()) {
            // If a whitespace is found, it indicates that the last word is complete, and the boundary is here.
            Some(last_space_idx) => tail_beg_untrimmed + last_space_idx,
            // If no whitespace is found (e.g., it is an excessively long word), then not a single word can be kept.
            None => tail_beg_untrimmed,
        }
    };

    let tail_is_truncated = final_tail_end != all_after.len();

    // trim away whitespace again.
    let (tail_beg, tail_end) = trim_idx(all_after, tail_beg_untrimmed, final_tail_end);

    // and get the string
    let tail_str: String = all_after[tail_beg..tail_end]
        .iter()
        .map(|&c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    tail.push_str(&tail_str);

    // the head chunk

    // max size of the head chunk = max size of right half - space taken by after chunk - gap size.
    let max_head_size = cmp::max(
        max_after_size as isize - after.len() as isize - config.gap_size as isize,
        0,
    ) as usize;

    // the head chunk takes text from before the before chunk
    let (_, head_end) = trim_idx(all_before, 0, before_beg);

    // Calculate the theoretical start point for the head.
    let initial_head_beg = cmp::max(head_end as isize - max_head_size as isize, 0) as usize;

    // Determine if actual content was truncated from the very beginning of `all_before`.
    let head_is_truncated = (initial_head_beg > 0
        && all_before[..initial_head_beg]
            .iter()
            .any(|c| !c.is_whitespace()))
        || head_has_truncated;

    // Adjust for broken words based on the theoretical start point.
    let head_beg = trim_broken_word_left(all_before, initial_head_beg, head_end);

    // trim away white space again.
    let (head_beg, head_end) = trim_idx(all_before, head_beg, head_end);

    // and get the string.
    let head_str: String = all_before[head_beg..head_end]
        .iter()
        .map(|&c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    head.push_str(&head_str);
    //The TeX mode does not output truncation characters.
    if config.format != OutFormat::Tex {
        // put right context truncation string if needed
        if after_is_truncated && tail_beg == tail_end {
            after.push_str(&config.trunc_str);
        } else if after_is_truncated && tail_is_truncated {
            tail.push_str(&config.trunc_str);
        }

        // put left context truncation string if needed
        if before_is_truncated && head_beg == head_end {
            before = format!("{}{before}", config.trunc_str);
        // } else if head_is_truncated && head_beg != 0 {
        } else if head_is_truncated {
            head = format!("{}{head}", config.trunc_str);
        }
    }

    (tail, before, after, head)
}

fn tex_mapper(x: char) -> String {
    match x {
        '\\' => "\\backslash{}".to_owned(),
        '$' | '%' | '#' | '&' | '_' => format!("\\{x}"),
        '}' | '{' => format!("$\\{x}$"),
        _ => x.to_string(),
    }
}

/// Escape special characters for TeX.
fn format_tex_field(s: &str) -> String {
    let mapped_chunks: Vec<String> = s.chars().map(tex_mapper).collect();
    mapped_chunks.join("")
}

fn format_tex_line(
    config: &Config,
    tail: &str,
    before: &str,
    keyword: &str,
    after: &str,
    head: &str,
    reference: &str,
) -> String {
    let mut output = String::new();
    write!(output, "\\{} ", config.macro_name).unwrap();
    write!(
        output,
        "{{{0}}}{{{1}}}{{{2}}}{{{3}}}{{{4}}}",
        format_tex_field(tail),
        format_tex_field(before),
        format_tex_field(keyword),
        format_tex_field(after),
        format_tex_field(head),
    )
    .unwrap();
    if config.auto_ref || config.input_ref {
        write!(output, "{{{}}}", format_tex_field(reference)).unwrap();
    }
    output
}

fn format_roff_field(s: &str) -> String {
    s.replace('\"', "\"\"")
}

fn format_roff_line(
    config: &Config,
    tail: &str,
    before: &str,
    keyword: &str,
    after: &str,
    head: &str,
    reference: &str,
) -> String {
    let mut output = String::new();
    write!(output, ".{}", config.macro_name).unwrap();
    write!(
        output,
        " \"{}\" \"{}\" \"{}{}\" \"{}\"",
        format_roff_field(tail),
        format_roff_field(before),
        format_roff_field(keyword),
        format_roff_field(after),
        format_roff_field(head)
    )
    .unwrap();
    if config.auto_ref || config.input_ref {
        write!(output, " \"{}\"", format_roff_field(reference)).unwrap();
    }
    output
}

/// Extract and prepare text chunks for formatting in both TeX and roff output
fn prepare_line_chunks(
    config: &Config,
    word_ref: &LineWordRef,
    line: &str,
    chars_line: &[char],
    max_word_len: usize,
) -> (String, String, String, String, String) {
    let half_line_width = config.line_width / 2;

    // First, determine the start of the "pure" context by skipping the reference in -r mode.
    // This position is relative to the start of the `line`.
    let pure_context_start_pos = if config.input_ref {
        let ref_end = line.find(char::is_whitespace).unwrap_or(line.len());
        line[ref_end..]
            .find(|c: char| !c.is_whitespace())
            .map_or(line.len(), |i| ref_end + i)
    } else {
        0
    };

    // The length of the left context is the distance from the keyword back to the start of the pure context.
    let left_context_len = word_ref.position.saturating_sub(pure_context_start_pos);

    // This is the calculated start position for `all_before`, as a BYTE index into `line`.
    //Optimize the logic: if the left context is too long, skip a part to be compatible with the GNU version.
    let (left_field_start_rel_bytes, head_has_truncated) =
        if left_context_len > half_line_width + max_word_len {
            // Optimization triggered: Jump back from the keyword.
            let mut jump_back_pos = word_ref
                .position
                .saturating_sub(half_line_width + max_word_len);

            //handle utf-8 boundary
            while !line.is_char_boundary(jump_back_pos) {
                jump_back_pos -= 1;
            }
            let slice_after_jump = &line[jump_back_pos..];
            // From the jump-back position, skip one "chunk" (word or whitespace) forward
            let chunk_len =
                find_first_chunk_end(slice_after_jump, config.word_regex.as_ref(), config.gnu_ext);
            (jump_back_pos + chunk_len, true)
        } else {
            // Optimization not triggered: Start from the beginning of the pure context.
            (pure_context_start_pos, false)
        };

    // We need the character count for the keyword's start position (end of `all_before`).
    let ref_char_position = line[..word_ref.position].chars().count();

    //Convert our calculated BYTE start position to a CHARACTER start position for slicing `chars_line`.
    let all_before_start_char = line[..left_field_start_rel_bytes].chars().count();

    let all_before = &chars_line[all_before_start_char..ref_char_position];

    let char_position_end = ref_char_position
        + line[word_ref.position..word_ref.position_end]
            .chars()
            .count();

    let keyword = line[word_ref.position..word_ref.position_end].to_string();
    let all_after = &chars_line[char_position_end..];
    let (tail, before, after, head) =
        get_output_chunks(all_before, &keyword, all_after, head_has_truncated, config);

    (tail, before, keyword, after, head)
}

/// Calculates the byte length of the first logical "chunk" at the beginning of the slice.
/// The/// gic follows three paths:
/// 1. If a `-W` regex is provided, it returns the length of a match at the start of the slice,
///    or the length of the first character if no match occurs.
/// 2. If at a default "word character", it returns the length of the entire contiguous word.
/// 3. If at a non-word character (e.g., space), it returns the length of that single character.
fn find_first_chunk_end(slice: &str, word_reg: Option<&String>, gnu_ext: bool) -> usize {
    if slice.is_empty() {
        return 0;
    }

    if let Some(reg_str) = word_reg {
        let reg = Regex::new(reg_str).unwrap();
        return match reg.find(slice) {
            Some(mat) if mat.start() == 0 => {
                // The regex matched at the very beginning of the slice.
                // The chunk is the entire match.
                mat.end()
            }
            _ => {
                // The regex did not match at the beginning, or did not match at all.
                // In this case, advance by one char.
                slice.chars().next().map_or(0, |c| c.len_utf8())
            }
        };
    }

    // No `-W` option, use default behavior.
    let first_char = slice.chars().next().unwrap();

    // Determine if the first character is part of a "default word".
    let is_word_char = if gnu_ext {
        first_char.is_alphabetic()
    } else {
        !first_char.is_whitespace()
    };

    if is_word_char {
        // We are at the start of a default word. Consume the whole word.
        if gnu_ext {
            slice
                .find(|c: char| !c.is_alphabetic())
                .unwrap_or(slice.len())
        } else {
            slice.find(char::is_whitespace).unwrap_or(slice.len())
        }
    } else {
        // We are at a non-word character (e.g., space, punctuation in GNU mode).
        //So we advance by one UTF-8 char.
        first_char.len_utf8()
    }
}
fn prepare_stream_chunks(
    config: &Config,
    word_ref: &StreamWordRef,
    full_content: &str,
    max_word_len: usize,
) -> (String, String, String, String, String) {
    let half_line_width = config.line_width / 2;
    let left_context_len = word_ref.absolute_position - word_ref.context_start;

    let (left_field_start_abs, head_has_truncated) =
        if left_context_len > half_line_width + max_word_len {
            // Jump back from the keyword.
            let jump_back_pos = word_ref
                .absolute_position
                .saturating_sub(half_line_width + max_word_len);

            //  From the jump-back position, skip one "chunk" (word or whitespace) forward
            //  to ensure we don't start in the middle of a word.
            let slice_after_jump = &full_content[jump_back_pos..];
            let chunk_len =
                find_first_chunk_end(slice_after_jump, config.word_regex.as_ref(), config.gnu_ext);

            (jump_back_pos + chunk_len, true)
        } else {
            // If the left context is not long, we start from the beginning of the context.
            (word_ref.context_start, false)
        };

    let keyword = &full_content[word_ref.absolute_position..word_ref.absolute_position_end];

    // `all_before` now correctly starts from the calculated `left_field_start_abs`.
    let all_before_str = &full_content[left_field_start_abs..word_ref.absolute_position];
    // `all_after` is calculated based on the original context end.
    let all_after_str = &full_content[word_ref.absolute_position_end..word_ref.context_end];

    let all_before_chars: Vec<char> = all_before_str.chars().collect();
    let all_after_chars: Vec<char> = all_after_str.chars().collect();
    let (tail, before, after, head) = get_output_chunks(
        &all_before_chars,
        keyword,
        &all_after_chars,
        head_has_truncated,
        config,
    );

    (tail, before, keyword.to_string(), after, head)
}

fn write_traditional_output(
    config: &mut Config,
    file_map: &FileMap,
    ptx_result: &PtxResult,
    output_filename: &OsStr,
) -> UResult<()> {
    let mut writer: BufWriter<Box<dyn Write>> =
        BufWriter::new(if output_filename == OsStr::new("-") {
            Box::new(stdout())
        } else {
            let file = File::create(output_filename)
                .map_err_context(|| output_filename.to_string_lossy().quote().to_string())?;
            Box::new(file)
        });

    if !config.right_ref {
        let max_ref_len = if config.auto_ref {
            get_auto_max_reference_len(&ptx_result.words) + config.gap_size
        } else if config.input_ref {
            get_input_max_reference_len(file_map) + config.gap_size
        } else {
            0
        };
        config.line_width -= max_ref_len;
    }

    for word_ref_enum in &ptx_result.words {
        let file_content: &FileContent = file_map
            .get(word_ref_enum.filename())
            .expect("Missing file in file map");

        let physical_line = &file_content.lines[word_ref_enum.local_line_nr()];
        let reference = get_reference(config, word_ref_enum, physical_line);

        let (tail, before, keyword, after, head) = match word_ref_enum {
            WordRef::Stream(word_ref) => prepare_stream_chunks(
                config,
                word_ref,
                &file_content.full_content,
                ptx_result.max_word_len,
            ),
            WordRef::Line(word_ref) => {
                let line = &file_content.lines[word_ref.local_line_nr];
                let chars_line = &file_content.chars_lines[word_ref.local_line_nr];
                prepare_line_chunks(config, word_ref, line, chars_line, ptx_result.max_word_len)
            }
        };

        let output_line: String = match config.format {
            OutFormat::Tex => {
                format_tex_line(config, &tail, &before, &keyword, &after, &head, &reference)
            }
            OutFormat::Roff => {
                format_roff_line(config, &tail, &before, &keyword, &after, &head, &reference)
            }
            OutFormat::Dumb => {
                return Err(PtxError::DumbFormat.into());
            }
        };
        writeln!(writer, "{output_line}")
            .map_err_context(|| translate!("ptx-error-write-failed"))?;
    }

    writer
        .flush()
        .map_err_context(|| translate!("ptx-error-write-failed"))?;

    Ok(())
}

fn get_input_max_reference_len(file_map: &FileMap) -> usize {
    let mut input_max_reference_len = 0;
    for file_content in file_map.values() {
        for line in &file_content.lines {
            let ref_len = line.find(char::is_whitespace).unwrap_or(line.len());

            // 更新我们目前找到的最大长度
            input_max_reference_len = cmp::max(input_max_reference_len, ref_len);
        }
    }
    input_max_reference_len
}

fn get_auto_max_reference_len(words: &BTreeSet<WordRef>) -> usize {
    //Get the maximum length of the reference field
    let line_num = words
        .iter()
        .map(|w| {
            if w.local_line_nr() == 0 {
                1
            } else {
                (w.local_line_nr() as f64).log10() as usize + 1
            }
        })
        .max()
        .unwrap_or(0);

    let filename_len = words
        .iter()
        .filter(|w| w.filename() != "-")
        .map(|w| w.filename().maybe_quote().to_string().len())
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
    pub static WIDTH: &str = "width";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let mut config = get_config(&matches)?;

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
                translate!("ptx-error-extra-operand", "operand" => file.to_string_lossy().quote()),
            ));
        }
    }

    let word_filter = WordFilter::new(&matches, &config)?;
    let file_map = read_input(&input_files).map_err_context(String::new)?;
    let ptx_result = create_word_set(&config, &word_filter, &file_map);
    write_traditional_output(&mut config, &file_map, &ptx_result, &output_file)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(translate!("ptx-about"))
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
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
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .help(translate!("ptx-help-width"))
                .value_name("NUMBER"),
        )
}
