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

#[derive(Debug, PartialOrd, PartialEq, Eq, Ord)]
struct WordRef {
    word: String,
    global_line_nr: usize,
    local_line_nr: usize,
    position: usize,
    position_end: usize,
    filename: OsString,
}

#[derive(Debug, Error)]
enum PtxError {
    #[error("{}", translate!("ptx-error-not-implemented", "feature" => (*.0)))]
    NotImplemented(&'static str),

    #[error("{0}")]
    ParseError(ParseIntError),
}

impl UError for PtxError {}

fn get_config(matches: &clap::ArgMatches) -> UResult<Config> {
    let mut config = Config::default();
    let err_msg = "parsing options failed";
    if matches.get_flag(options::TRADITIONAL) {
        config.gnu_ext = false;
        config.format = OutFormat::Roff;
        "[^ \t\n]+".clone_into(&mut config.context_regex);
    }
    if matches.contains_id(options::SENTENCE_REGEXP) {
        return Err(PtxError::NotImplemented("-S").into());
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
}

type FileMap = HashMap<OsString, FileContent>;

fn read_input(input_files: &[OsString]) -> std::io::Result<FileMap> {
    let mut file_map: FileMap = HashMap::new();
    let mut offset: usize = 0;
    for filename in input_files {
        let reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
            Box::new(stdin())
        } else {
            let file = File::open(Path::new(filename))?;
            Box::new(file)
        });
        let lines: Vec<String> = reader.lines().collect::<std::io::Result<Vec<String>>>()?;

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
            },
        );
        offset += size;
    }
    Ok(file_map)
}

/// Go through every lines in the input files and record each match occurrence as a `WordRef`.
fn create_word_set(config: &Config, filter: &WordFilter, file_map: &FileMap) -> BTreeSet<WordRef> {
    let reg = Regex::new(&filter.word_regex).unwrap();
    let ref_reg = Regex::new(&config.context_regex).unwrap();
    let mut word_set: BTreeSet<WordRef> = BTreeSet::new();
    for (file, lines) in file_map {
        let mut count: usize = 0;
        let offs = lines.offset;
        for line in &lines.lines {
            // if -r, exclude reference from word set
            let (ref_beg, ref_end) = match ref_reg.find(line) {
                Some(x) => (x.start(), x.end()),
                None => (0, 0),
            };
            // match words with given regex
            for mat in reg.find_iter(line) {
                let (beg, end) = (mat.start(), mat.end());
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
                word_set.insert(WordRef {
                    word,
                    filename: file.clone(),
                    global_line_nr: offs + count,
                    local_line_nr: count,
                    position: beg,
                    position_end: end,
                });
            }
            count += 1;
        }
    }
    word_set
}

fn get_reference(config: &Config, word_ref: &WordRef, line: &str, context_reg: &Regex) -> String {
    if config.auto_ref {
        if word_ref.filename == "-" {
            format!(":{}", word_ref.local_line_nr + 1)
        } else {
            format!(
                "{}:{}",
                word_ref.filename.maybe_quote(),
                word_ref.local_line_nr + 1
            )
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

    // the minimum possible begin index of the before_chunk is the end index minus the length.
    let before_beg = cmp::max(before_end as isize - max_before_size as isize, 0) as usize;
    // in case that falls in the middle of a word, trim away the word.
    let before_beg = trim_broken_word_left(all_before, before_beg, before_end);

    // trim away white space.
    let (before_beg, before_end) = trim_idx(all_before, before_beg, before_end);

    // and get the string.
    let before_str: String = all_before[before_beg..before_end].iter().collect();
    before.push_str(&before_str);
    assert!(max_before_size >= before.len());

    // the after chunk

    // must be no longer than the minimum between the max size and the total available string.
    let after_end = cmp::min(max_after_size, all_after.len());
    // in case that falls in the middle of a word, trim away the word.
    let after_end = trim_broken_word_right(all_after, 0, after_end);

    // trim away white space.
    let (_, after_end) = trim_idx(all_after, 0, after_end);

    // and get the string
    let after_str: String = all_after[0..after_end].iter().collect();
    after.push_str(&after_str);
    assert!(max_after_size >= after.len());

    // the tail chunk

    // max size of the tail chunk = max size of left half - space taken by before chunk - gap size.
    let max_tail_size = cmp::max(
        max_before_size as isize - before.len() as isize - config.gap_size as isize,
        0,
    ) as usize;

    // the tail chunk takes text starting from where the after chunk ends (with whitespace trimmed).
    let (tail_beg, _) = trim_idx(all_after, after_end, all_after.len());

    // end = begin + max length
    let tail_end = cmp::min(all_after.len(), tail_beg + max_tail_size);
    // in case that falls in the middle of a word, trim away the word.
    let tail_end = trim_broken_word_right(all_after, tail_beg, tail_end);

    // trim away whitespace again.
    let (tail_beg, mut tail_end) = trim_idx(all_after, tail_beg, tail_end);
    // Fix: Manually trim trailing char (like "a") that are preceded by a space.
    // This handles cases like "is a" which are not correctly trimmed by the
    // preceding functions.
    if tail_end >= 2
        && (tail_end - 2) > tail_beg
        && all_after[tail_end - 2].is_whitespace()
        && !all_after[tail_end - 1].is_whitespace()
    {
        tail_end -= 1;
        (_, tail_end) = trim_idx(all_after, tail_beg, tail_end);
    }

    // and get the string
    let tail_str: String = all_after[tail_beg..tail_end].iter().collect();
    tail.push_str(&tail_str);

    // the head chunk

    // max size of the head chunk = max size of right half - space taken by after chunk - gap size.
    let max_head_size = cmp::max(
        max_after_size as isize - after.len() as isize - config.gap_size as isize,
        0,
    ) as usize;

    // the head chunk takes text from before the before chunk
    let (_, head_end) = trim_idx(all_before, 0, before_beg);

    // begin = end - max length
    let head_beg = cmp::max(head_end as isize - max_head_size as isize, 0) as usize;
    // in case that falls in the middle of a word, trim away the word.
    let head_beg = trim_broken_word_left(all_before, head_beg, head_end);

    // trim away white space again.
    let (head_beg, head_end) = trim_idx(all_before, head_beg, head_end);

    // and get the string.
    let head_str: String = all_before[head_beg..head_end].iter().collect();
    head.push_str(&head_str);
    //The TeX mode does not output truncation characters.
    if config.format != OutFormat::Tex {
        // put right context truncation string if needed
        if after_end != all_after.len() && tail_beg == tail_end {
            after.push_str(&config.trunc_str);
        } else if after_end != all_after.len() && tail_end != all_after.len() {
            tail.push_str(&config.trunc_str);
        }

        // put left context truncation string if needed
        if before_beg != 0 && head_beg == head_end {
            before = format!("{}{before}", config.trunc_str);
        } else if before_beg != 0 && head_beg != 0 {
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
    word_ref: &WordRef,
    line: &str,
    chars_line: &[char],
    reference: &str,
) -> String {
    let mut output = String::new();
    write!(output, "\\{} ", config.macro_name).unwrap();
    let (tail, before, keyword, after, head) =
        prepare_line_chunks(config, word_ref, line, chars_line, reference);
    write!(
        output,
        "{{{0}}}{{{1}}}{{{2}}}{{{3}}}{{{4}}}",
        format_tex_field(&tail),
        format_tex_field(&before),
        format_tex_field(&keyword),
        format_tex_field(&after),
        format_tex_field(&head),
    )
    .unwrap();
    if config.auto_ref || config.input_ref {
        write!(output, "{{{}}}", format_tex_field(reference)).unwrap();
    }
    output
}

fn format_dumb_line(
    config: &Config,
    word_ref: &WordRef,
    line: &str,
    chars_line: &[char],
    reference: &str,
) -> String {
    let (tail, before, keyword, after, head) =
        prepare_line_chunks(config, word_ref, line, chars_line, reference);

    // Calculate the position for the left part
    // The left part consists of tail (if present) + space + before
    let left_part = if tail.is_empty() {
        before
    } else if before.is_empty() {
        tail
    } else {
        format!("{tail} {before}")
    };

    // Calculate the position for the right part
    let right_part = if head.is_empty() {
        after
    } else if after.is_empty() {
        head
    } else {
        format!("{after} {head}")
    };

    // Calculate the width for the left half (before the keyword)
    let half_width = config.line_width / 2;

    // Right-justify the left part within the left half
    let padding = if left_part.len() < half_width {
        half_width - left_part.len()
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
    let (tail, before, keyword, after, head) =
        prepare_line_chunks(config, word_ref, line, chars_line, reference);
    write!(
        output,
        " \"{}\" \"{}\" \"{}{}\" \"{}\"",
        format_roff_field(&tail),
        format_roff_field(&before),
        format_roff_field(&keyword),
        format_roff_field(&after),
        format_roff_field(&head)
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
    word_ref: &WordRef,
    line: &str,
    chars_line: &[char],
    reference: &str,
) -> (String, String, String, String, String) {
    // Convert byte positions to character positions
    let ref_char_position = line[..word_ref.position].chars().count();
    let char_position_end = ref_char_position
        + line[word_ref.position..word_ref.position_end]
            .chars()
            .count();

    // Extract the text before the keyword
    let all_before = if config.input_ref {
        let before = &line[..word_ref.position];
        let before_char_count = before.chars().count();
        let trimmed_char_count = before
            .trim_start_matches(reference)
            .trim_start()
            .chars()
            .count();
        let trim_offset = before_char_count - trimmed_char_count;
        &chars_line[trim_offset..before_char_count]
    } else {
        &chars_line[..ref_char_position]
    };

    // Extract the keyword and text after it
    let keyword = line[word_ref.position..word_ref.position_end].to_string();
    let all_after = &chars_line[char_position_end..];

    // Get formatted output chunks
    let (tail, before, after, head) = get_output_chunks(all_before, &keyword, all_after, config);

    (tail, before, keyword, after, head)
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
                .map_err_context(|| output_filename.to_string_lossy().quote().to_string())?;
            Box::new(file)
        });

    let context_reg = Regex::new(&config.context_regex).unwrap();

    if !config.right_ref {
        let max_ref_len = if config.auto_ref {
            get_auto_max_reference_len(words)
        } else {
            0
        };
        config.line_width -= max_ref_len;
    }

    for word_ref in words {
        let file_map_value: &FileContent = file_map
            .get(&word_ref.filename)
            .expect("Missing file in file map");
        let FileContent {
            ref lines,
            ref chars_lines,
            offset: _,
        } = *(file_map_value);
        let reference = get_reference(
            config,
            word_ref,
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

fn get_auto_max_reference_len(words: &BTreeSet<WordRef>) -> usize {
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
        .filter(|w| w.filename != "-")
        .map(|w| w.filename.maybe_quote().to_string().len())
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
    let word_set = create_word_set(&config, &word_filter, &file_map);
    write_traditional_output(&mut config, &file_map, &word_set, &output_file)
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
