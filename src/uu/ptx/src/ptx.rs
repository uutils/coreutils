//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDOs) corasick memchr Roff trunc oset iset

use clap::{crate_version, Arg, Command};
use regex::Regex;
use std::cmp;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::default::Default;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::num::ParseIntError;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};
use uucore::{format_usage, InvalidEncodingHandling};

static NAME: &str = "ptx";
const USAGE: &str = "\
    {} [OPTION]... [INPUT]...
    {} -G [OPTION]... [INPUT [OUTPUT]]";

const ABOUT: &str = "\
    Output a permuted index, including context, of the words in the input files. \n\n\
    Mandatory arguments to long options are mandatory for short options too.\n\
    With no FILE, or when FILE is -, read standard input. Default is '-F /'.";

#[derive(Debug)]
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
        .value_of(option)
        .expect("parsing options failed!")
        .to_string();
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut words: HashSet<String> = HashSet::new();
    for word in reader.lines() {
        words.insert(word?);
    }
    Ok(words)
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
    fn new(matches: &clap::ArgMatches, config: &Config) -> UResult<Self> {
        let (o, oset): (bool, HashSet<String>) = if matches.is_present(options::ONLY_FILE) {
            let words =
                read_word_filter_file(matches, options::ONLY_FILE).map_err_context(String::new)?;
            (true, words)
        } else {
            (false, HashSet::new())
        };
        let (i, iset): (bool, HashSet<String>) = if matches.is_present(options::IGNORE_FILE) {
            let words = read_word_filter_file(matches, options::IGNORE_FILE)
                .map_err_context(String::new)?;
            (true, words)
        } else {
            (false, HashSet::new())
        };
        if matches.is_present(options::BREAK_FILE) {
            return Err(PtxError::NotImplemented("-b").into());
        }
        // Ignore empty string regex from cmd-line-args
        let arg_reg: Option<String> = if matches.is_present(options::WORD_REGEXP) {
            match matches.value_of(options::WORD_REGEXP) {
                Some(v) => {
                    if v.is_empty() {
                        None
                    } else {
                        Some(v.to_string())
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
                if config.gnu_ext {
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
    filename: String,
}

#[derive(Debug)]
enum PtxError {
    DumbFormat,
    NotImplemented(&'static str),
    ParseError(ParseIntError),
}

impl Error for PtxError {}
impl UError for PtxError {}

impl Display for PtxError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            PtxError::DumbFormat => {
                write!(f, "There is no dumb format with GNU extensions disabled")
            }
            PtxError::NotImplemented(s) => write!(f, "{} not implemented yet", s),
            PtxError::ParseError(e) => e.fmt(f),
        }
    }
}

fn get_config(matches: &clap::ArgMatches) -> UResult<Config> {
    let mut config: Config = Default::default();
    let err_msg = "parsing options failed";
    if matches.is_present(options::TRADITIONAL) {
        config.gnu_ext = false;
        config.format = OutFormat::Roff;
        config.context_regex = "[^ \t\n]+".to_owned();
    } else {
        return Err(PtxError::NotImplemented("GNU extensions").into());
    }
    if matches.is_present(options::SENTENCE_REGEXP) {
        return Err(PtxError::NotImplemented("-S").into());
    }
    config.auto_ref = matches.is_present(options::AUTO_REFERENCE);
    config.input_ref = matches.is_present(options::REFERENCES);
    config.right_ref &= matches.is_present(options::RIGHT_SIDE_REFS);
    config.ignore_case = matches.is_present(options::IGNORE_CASE);
    if matches.is_present(options::MACRO_NAME) {
        config.macro_name = matches
            .value_of(options::MACRO_NAME)
            .expect(err_msg)
            .to_string();
    }
    if matches.is_present(options::FLAG_TRUNCATION) {
        config.trunc_str = matches
            .value_of(options::FLAG_TRUNCATION)
            .expect(err_msg)
            .to_string();
    }
    if matches.is_present(options::WIDTH) {
        config.line_width = matches
            .value_of(options::WIDTH)
            .expect(err_msg)
            .parse()
            .map_err(PtxError::ParseError)?;
    }
    if matches.is_present(options::GAP_SIZE) {
        config.gap_size = matches
            .value_of(options::GAP_SIZE)
            .expect(err_msg)
            .parse()
            .map_err(PtxError::ParseError)?;
    }
    if matches.is_present(options::FORMAT_ROFF) {
        config.format = OutFormat::Roff;
    }
    if matches.is_present(options::FORMAT_TEX) {
        config.format = OutFormat::Tex;
    }
    Ok(config)
}

struct FileContent {
    lines: Vec<String>,
    chars_lines: Vec<Vec<char>>,
    offset: usize,
}

type FileMap = HashMap<String, FileContent>;

fn read_input(input_files: &[String], config: &Config) -> std::io::Result<FileMap> {
    let mut file_map: FileMap = HashMap::new();
    let mut files = Vec::new();
    if input_files.is_empty() {
        files.push("-");
    } else if config.gnu_ext {
        for file in input_files {
            files.push(file);
        }
    } else {
        files.push(&input_files[0]);
    }
    let mut offset: usize = 0;
    for filename in files {
        let reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
            Box::new(stdin())
        } else {
            let file = File::open(filename)?;
            Box::new(file)
        });
        let lines: Vec<String> = reader.lines().collect::<std::io::Result<Vec<String>>>()?;

        // Indexing UTF-8 string requires walking from the beginning, which can hurts performance badly when the line is long.
        // Since we will be jumping around the line a lot, we dump the content into a Vec<char>, which can be indexed in constant time.
        let chars_lines: Vec<Vec<char>> = lines.iter().map(|x| x.chars().collect()).collect();
        let size = lines.len();
        file_map.insert(
            filename.to_owned(),
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
    for (file, lines) in file_map.iter() {
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
                if filter.only_specified && !(filter.only_set.contains(&word)) {
                    continue;
                }
                if filter.ignore_specified && filter.ignore_set.contains(&word) {
                    continue;
                }
                if config.ignore_case {
                    word = word.to_lowercase();
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
        format!(
            "{}:{}",
            word_ref.filename.maybe_quote(),
            word_ref.local_line_nr + 1
        )
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
    while b < e && s[e - 1].is_whitespace() {
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
    let half_line_size = (config.line_width / 2) as usize;
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
    let tail_end = cmp::min(all_after.len(), tail_beg + max_tail_size) as usize;
    // in case that falls in the middle of a word, trim away the word.
    let tail_end = trim_broken_word_right(all_after, tail_beg, tail_end);

    // trim away whitespace again.
    let (tail_beg, tail_end) = trim_idx(all_after, tail_beg, tail_end);

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

    // put right context truncation string if needed
    if after_end != all_after.len() && tail_beg == tail_end {
        after.push_str(&config.trunc_str);
    } else if after_end != all_after.len() && tail_end != all_after.len() {
        tail.push_str(&config.trunc_str);
    }

    // put left context truncation string if needed
    if before_beg != 0 && head_beg == head_end {
        before = format!("{}{}", config.trunc_str, before);
    } else if before_beg != 0 && head_beg != 0 {
        head = format!("{}{}", config.trunc_str, head);
    }

    (tail, before, after, head)
}

fn tex_mapper(x: char) -> String {
    match x {
        '\\' => "\\backslash{}".to_owned(),
        '$' | '%' | '#' | '&' | '_' => format!("\\{}", x),
        '}' | '{' => format!("$\\{}$", x),
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
    output.push_str(&format!("\\{} ", config.macro_name));
    let all_before = if config.input_ref {
        let before = &line[0..word_ref.position];
        let before_start_trim_offset =
            word_ref.position - before.trim_start_matches(reference).trim_start().len();
        let before_end_index = before.len();
        &chars_line[before_start_trim_offset..cmp::max(before_end_index, before_start_trim_offset)]
    } else {
        let before_chars_trim_idx = (0, word_ref.position);
        &chars_line[before_chars_trim_idx.0..before_chars_trim_idx.1]
    };
    let keyword = &line[word_ref.position..word_ref.position_end];
    let after_chars_trim_idx = (word_ref.position_end, chars_line.len());
    let all_after = &chars_line[after_chars_trim_idx.0..after_chars_trim_idx.1];
    let (tail, before, after, head) = get_output_chunks(all_before, keyword, all_after, config);
    output.push_str(&format!(
        "{5}{0}{6}{5}{1}{6}{5}{2}{6}{5}{3}{6}{5}{4}{6}",
        format_tex_field(&tail),
        format_tex_field(&before),
        format_tex_field(keyword),
        format_tex_field(&after),
        format_tex_field(&head),
        "{",
        "}"
    ));
    if config.auto_ref || config.input_ref {
        output.push_str(&format!("{}{}{}", "{", format_tex_field(reference), "}"));
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
    output.push_str(&format!(".{}", config.macro_name));
    let all_before = if config.input_ref {
        let before = &line[0..word_ref.position];
        let before_start_trim_offset =
            word_ref.position - before.trim_start_matches(reference).trim_start().len();
        let before_end_index = before.len();
        &chars_line[before_start_trim_offset..cmp::max(before_end_index, before_start_trim_offset)]
    } else {
        let before_chars_trim_idx = (0, word_ref.position);
        &chars_line[before_chars_trim_idx.0..before_chars_trim_idx.1]
    };
    let keyword = &line[word_ref.position..word_ref.position_end];
    let after_chars_trim_idx = (word_ref.position_end, chars_line.len());
    let all_after = &chars_line[after_chars_trim_idx.0..after_chars_trim_idx.1];
    let (tail, before, after, head) = get_output_chunks(all_before, keyword, all_after, config);
    output.push_str(&format!(
        " \"{}\" \"{}\" \"{}{}\" \"{}\"",
        format_roff_field(&tail),
        format_roff_field(&before),
        format_roff_field(keyword),
        format_roff_field(&after),
        format_roff_field(&head)
    ));
    if config.auto_ref || config.input_ref {
        output.push_str(&format!(" \"{}\"", format_roff_field(reference)));
    }
    output
}

fn write_traditional_output(
    config: &Config,
    file_map: &FileMap,
    words: &BTreeSet<WordRef>,
    output_filename: &str,
) -> UResult<()> {
    let mut writer: BufWriter<Box<dyn Write>> = BufWriter::new(if output_filename == "-" {
        Box::new(stdout())
    } else {
        let file = File::create(output_filename).map_err_context(String::new)?;
        Box::new(file)
    });

    let context_reg = Regex::new(&config.context_regex).unwrap();

    for word_ref in words.iter() {
        let file_map_value: &FileContent = file_map
            .get(&(word_ref.filename))
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
            OutFormat::Dumb => {
                return Err(PtxError::DumbFormat.into());
            }
        };
        writeln!(writer, "{}", output_line).map_err_context(String::new)?;
    }
    Ok(())
}

mod options {
    pub static FILE: &str = "file";
    pub static AUTO_REFERENCE: &str = "auto-reference";
    pub static TRADITIONAL: &str = "traditional";
    pub static FLAG_TRUNCATION: &str = "flag-truncation";
    pub static MACRO_NAME: &str = "macro-name";
    pub static FORMAT_ROFF: &str = "format=roff";
    pub static RIGHT_SIDE_REFS: &str = "right-side-refs";
    pub static SENTENCE_REGEXP: &str = "sentence-regexp";
    pub static FORMAT_TEX: &str = "format=tex";
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
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    // let mut opts = Options::new();
    let matches = uu_app().get_matches_from(args);

    let mut input_files: Vec<String> = match &matches.values_of(options::FILE) {
        Some(v) => v.clone().map(|v| v.to_owned()).collect(),
        None => vec!["-".to_string()],
    };

    let config = get_config(&matches)?;
    let word_filter = WordFilter::new(&matches, &config)?;
    let file_map = read_input(&input_files, &config).map_err_context(String::new)?;
    let word_set = create_word_set(&config, &word_filter, &file_map);
    let output_file = if !config.gnu_ext && input_files.len() == 2 {
        input_files.pop().unwrap()
    } else {
        "-".to_string()
    };
    write_traditional_output(&config, &file_map, &word_set, &output_file)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .about(ABOUT)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .multiple_occurrences(true),
        )
        .arg(
            Arg::new(options::AUTO_REFERENCE)
                .short('A')
                .long(options::AUTO_REFERENCE)
                .help("output automatically generated references")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::TRADITIONAL)
                .short('G')
                .long(options::TRADITIONAL)
                .help("behave more like System V 'ptx'"),
        )
        .arg(
            Arg::new(options::FLAG_TRUNCATION)
                .short('F')
                .long(options::FLAG_TRUNCATION)
                .help("use STRING for flagging line truncations")
                .value_name("STRING")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::MACRO_NAME)
                .short('M')
                .long(options::MACRO_NAME)
                .help("macro name to use instead of 'xx'")
                .value_name("STRING")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::FORMAT_ROFF)
                .short('O')
                .long(options::FORMAT_ROFF)
                .help("generate output as roff directives"),
        )
        .arg(
            Arg::new(options::RIGHT_SIDE_REFS)
                .short('R')
                .long(options::RIGHT_SIDE_REFS)
                .help("put references at right, not counted in -w")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::SENTENCE_REGEXP)
                .short('S')
                .long(options::SENTENCE_REGEXP)
                .help("for end of lines or end of sentences")
                .value_name("REGEXP")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::FORMAT_TEX)
                .short('T')
                .long(options::FORMAT_TEX)
                .help("generate output as TeX directives"),
        )
        .arg(
            Arg::new(options::WORD_REGEXP)
                .short('W')
                .long(options::WORD_REGEXP)
                .help("use REGEXP to match each keyword")
                .value_name("REGEXP")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::BREAK_FILE)
                .short('b')
                .long(options::BREAK_FILE)
                .help("word break characters in this FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .short('f')
                .long(options::IGNORE_CASE)
                .help("fold lower case to upper case for sorting")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::GAP_SIZE)
                .short('g')
                .long(options::GAP_SIZE)
                .help("gap size in columns between output fields")
                .value_name("NUMBER")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::IGNORE_FILE)
                .short('i')
                .long(options::IGNORE_FILE)
                .help("read ignore word list from FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::ONLY_FILE)
                .short('o')
                .long(options::ONLY_FILE)
                .help("read only word list from this FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::REFERENCES)
                .short('r')
                .long(options::REFERENCES)
                .help("first field of each line is a reference")
                .value_name("FILE")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .help("output width in columns, reference excluded")
                .value_name("NUMBER")
                .takes_value(true),
        )
}
