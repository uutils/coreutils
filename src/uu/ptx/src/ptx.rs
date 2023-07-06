//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDOs) corasick memchr Roff trunc oset iset CHARCLASS

use clap::{crate_version, Arg, ArgAction, Command};
use regex::Regex;
use std::cmp;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::default::Default;
use std::error::Error;
use std::fmt::{Display, Formatter, Write as FmtWrite};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::num::ParseIntError;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult};
use uucore::{format_usage, help_about, help_usage};

const USAGE: &str = help_usage!("ptx.md");
const ABOUT: &str = help_about!("ptx.md");

const REGEX_CHARCLASS: &str = "^-]\\";

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
        .get_one::<String>(option)
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

/// reads contents of file as unique set of characters to be used with the break-file option
fn read_char_filter_file(
    matches: &clap::ArgMatches,
    option: &str,
) -> std::io::Result<HashSet<char>> {
    let filename = matches
        .get_one::<String>(option)
        .expect("parsing options failed!");
    let mut reader = File::open(filename)?;
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
                [' ', '\t', '\n'].iter().cloned().collect()
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
                if break_set.is_some() {
                    format!(
                        "[^{}]+",
                        break_set
                            .unwrap()
                            .into_iter()
                            .map(|c| if REGEX_CHARCLASS.contains(c) {
                                format!("\\{c}")
                            } else {
                                c.to_string()
                            })
                            .collect::<String>()
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
    position: (usize, usize),
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
            Self::DumbFormat => {
                write!(f, "There is no dumb format with GNU extensions disabled")
            }
            Self::NotImplemented(s) => write!(f, "{s} not implemented yet"),
            Self::ParseError(e) => e.fmt(f),
        }
    }
}

fn get_config(matches: &clap::ArgMatches) -> UResult<Config> {
    let mut config = Config::default();
    let err_msg = "parsing options failed";
    if matches.get_flag(options::TRADITIONAL) {
        config.gnu_ext = false;
        config.format = OutFormat::Roff;
        config.context_regex = "[^ \t\n]+".to_owned();
    } else {
        return Err(PtxError::NotImplemented("GNU extensions").into());
    }
    if matches.contains_id(options::SENTENCE_REGEXP) {
        return Err(PtxError::NotImplemented("-S").into());
    }
    config.auto_ref = matches.get_flag(options::AUTO_REFERENCE);
    config.input_ref = matches.get_flag(options::REFERENCES);
    config.right_ref &= matches.get_flag(options::RIGHT_SIDE_REFS);
    config.ignore_case = matches.get_flag(options::IGNORE_CASE);
    if matches.contains_id(options::MACRO_NAME) {
        config.macro_name = matches
            .get_one::<String>(options::MACRO_NAME)
            .expect(err_msg)
            .to_string();
    }
    if matches.contains_id(options::FLAG_TRUNCATION) {
        config.trunc_str = matches
            .get_one::<String>(options::FLAG_TRUNCATION)
            .expect(err_msg)
            .to_string();
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
    if matches.get_flag(options::FORMAT_ROFF) {
        config.format = OutFormat::Roff;
    }
    if matches.get_flag(options::FORMAT_TEX) {
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
fn create_word_set(config: &Config, filter: &WordFilter, file_map: &FileMap) -> (BTreeSet<WordRef>, Vec<String>)  {
    let reg = Regex::new(&filter.word_regex).unwrap();
    let mut word_set: BTreeSet<WordRef> = BTreeSet::new();
    let mut word_lines: Vec<String> = Vec::new();
    for (file, lines) in file_map.iter() {
        let mut count: usize = 0;
        let offs = lines.offset;
        for line in &lines.lines {
            // match words with given regex
            for mat in reg.find_iter(line) {
                let mut word = line[mat.start()..mat.end()].to_owned();
                if config.ignore_case {
                    word = word.to_lowercase();
                }
                word_set.insert( WordRef {
                    word: word.to_string(),
                    filename: file.clone(),
                    global_line_nr: offs + count,
                    local_line_nr: count,
                    position: (mat.start(), mat.end()),
                });
            }
            word_lines.push(line.to_owned());
            count += 1;
        }
    }
    (word_set, word_lines)
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

fn get_trunc_offset_left(s: &str, o: usize) -> usize {
    let wp = Regex::new("\\s+").unwrap();
    for mat in wp.find_iter(s) {
        if mat.end() > o {
            return mat.end();
        }
    }
    0
}

fn get_trunc_offset_right(s: &str, o: usize) -> usize {
    let mut trunc_offset: usize = 0;
    let wp = Regex::new("\\s+").unwrap();
    for mat in wp.find_iter(s) {
        if mat.end() > o {
            return trunc_offset;
        }
        trunc_offset = mat.end();
    }
    trunc_offset
}

fn get_output_chunks(
    all_before: &str,
    keyword: &str,
    all_after: &str,
    config: &Config,
) -> (String, String, String, String) {
    // Chunk size logics are mostly copied from the GNU ptx source.
    // https://github.com/MaiZure/coreutils-8.3/blob/master/src/ptx.c#L1234
    let mut tail = "".to_string();
    let mut head = "".to_string();
    let mut before = all_before.to_owned();
    let mut after = all_after.to_owned();
    let half_line_size = config.line_width / 2;
    let max_before_size = cmp::max(half_line_size as isize - config.gap_size as isize, 0) as usize;
    let max_after_size = cmp::max(
        half_line_size as isize
            - (2 * config.trunc_str.len()) as isize
            - keyword.len() as isize
            - 1,
        0,
    ) as usize;

    // truncate before and put content in head chunk?
    if before.len() > max_before_size {
        let before_offset = get_trunc_offset_left(&before, before.len() - max_before_size);
        head.push_str(&before[..before_offset-1]);
        before.drain(..before_offset);
        before = before.trim().to_string();
    }
    // truncate after and put content in tail chunk?
    if after.len() > max_after_size {
        let after_offset = get_trunc_offset_right(&after, max_after_size);
        tail.push_str(&after[after_offset..]);
        after.truncate(after_offset);
        after = after.trim_end().to_string();
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
    word_line: &String,
    reference: &str,
) -> String {
    let mut output = String::new();
    let mut startbefore : usize = 0;
    let mut endbefore : usize = 0;
    write!(output, "\\{} ", config.macro_name).unwrap();

    if word_ref.position.0 != 0 {
        endbefore = word_ref.position.0-1;
    }

    if config.input_ref {
        startbefore = reference.len();
        if startbefore != endbefore {
            startbefore += 1;
        }
    }

    let all_before = &word_line[startbefore..endbefore].trim();
    let keyword = &word_ref.word;
    let all_after = &word_line[word_ref.position.1..];
    let (tail, before, after, head) = get_output_chunks(all_before, keyword, all_after, config);
    write!(
        output,
        "{{{0}}}{{{1}}}{{{2}}}{{{3}}}{{{4}}}",
        format_tex_field(&tail),
        format_tex_field(&before),
        format_tex_field(keyword),
        format_tex_field(&after),
        format_tex_field(&head),
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
    word_ref: &WordRef,
    word_line: &String,
    reference: &str,
    ) -> String {
    let mut output = String::new();
    let mut startbefore : usize = 0;
    let mut endbefore : usize = 0;
    write!(output, ".{}", config.macro_name).unwrap();

    if word_ref.position.0 != 0 {
        endbefore = word_ref.position.0;
    }

    if config.input_ref {
        startbefore = reference.len();
        if startbefore != endbefore {
            startbefore += 1;
        }
    }

    let all_before = &word_line[startbefore..endbefore].trim();
    let keyword = &word_ref.word;
    let all_after = &word_line[word_ref.position.1..];
    let (tail, before, after, head) = get_output_chunks(all_before, keyword, all_after, config);
    write!(
        output,
        " \"{}\" \"{}\" \"{}{}\" \"{}\"",
        format_roff_field(&tail),
        format_roff_field(&before),
        format_roff_field(keyword),
        format_roff_field(&after),
        format_roff_field(&head)
    )
    .unwrap();
    if config.auto_ref || config.input_ref {
        write!(output, " \"{}\"", format_roff_field(reference)).unwrap();
    }
    output
}

fn write_traditional_output(
    config: &Config,
    file_map: &FileMap,
    words: &BTreeSet<WordRef>,
    word_lines: &Vec<String>,
    filter: &WordFilter,
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
        let reference = get_reference(
            config,
            word_ref,
            &word_lines[word_ref.global_line_nr],
            &context_reg,
        );
        if filter.only_specified && !(filter.only_set.contains(&word_ref.word)) {
            continue;
        }
        if filter.ignore_specified && filter.ignore_set.contains(&word_ref.word) {
            continue;
        }
        if config.input_ref && (word_ref.position.0 == 0) {
            continue;
        }
        let output_line: String = match config.format {
            OutFormat::Tex => format_tex_line(
                    config,
                    word_ref,
                    &word_lines[word_ref.global_line_nr],
                    &reference,
            ),
            OutFormat::Roff => format_roff_line(
                    config,
                    word_ref,
                    &word_lines[word_ref.global_line_nr],
                    &reference,
            ),
            OutFormat::Dumb => {
                    return Err(PtxError::DumbFormat.into());
            }
        };
        writeln!(writer, "{output_line}").map_err_context(String::new)?;
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
    let args = args.collect_ignore();

    let matches = uu_app().try_get_matches_from(args)?;

    let mut input_files: Vec<String> = match &matches.get_many::<String>(options::FILE) {
        Some(v) => v.clone().map(|v| v.to_owned()).collect(),
        None => vec!["-".to_string()],
    };

    let config = get_config(&matches)?;
    let word_filter = WordFilter::new(&matches, &config)?;
    let file_map = read_input(&input_files, &config).map_err_context(String::new)?;
    let (word_set, word_lines) = create_word_set(&config, &word_filter, &file_map);
    let output_file = if !config.gnu_ext && input_files.len() == 2 {
        input_files.pop().unwrap()
    } else {
        "-".to_string()
    };
    write_traditional_output(&config, &file_map, &word_set, &word_lines, &word_filter, &output_file)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::AUTO_REFERENCE)
                .short('A')
                .long(options::AUTO_REFERENCE)
                .help("output automatically generated references")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TRADITIONAL)
                .short('G')
                .long(options::TRADITIONAL)
                .help("behave more like System V 'ptx'")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FLAG_TRUNCATION)
                .short('F')
                .long(options::FLAG_TRUNCATION)
                .help("use STRING for flagging line truncations")
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::MACRO_NAME)
                .short('M')
                .long(options::MACRO_NAME)
                .help("macro name to use instead of 'xx'")
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::FORMAT_ROFF)
                .short('O')
                .long(options::FORMAT_ROFF)
                .help("generate output as roff directives")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RIGHT_SIDE_REFS)
                .short('R')
                .long(options::RIGHT_SIDE_REFS)
                .help("put references at right, not counted in -w")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SENTENCE_REGEXP)
                .short('S')
                .long(options::SENTENCE_REGEXP)
                .help("for end of lines or end of sentences")
                .value_name("REGEXP"),
        )
        .arg(
            Arg::new(options::FORMAT_TEX)
                .short('T')
                .long(options::FORMAT_TEX)
                .help("generate output as TeX directives")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WORD_REGEXP)
                .short('W')
                .long(options::WORD_REGEXP)
                .help("use REGEXP to match each keyword")
                .value_name("REGEXP"),
        )
        .arg(
            Arg::new(options::BREAK_FILE)
                .short('b')
                .long(options::BREAK_FILE)
                .help("word break characters in this FILE")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .short('f')
                .long(options::IGNORE_CASE)
                .help("fold lower case to upper case for sorting")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::GAP_SIZE)
                .short('g')
                .long(options::GAP_SIZE)
                .help("gap size in columns between output fields")
                .value_name("NUMBER"),
        )
        .arg(
            Arg::new(options::IGNORE_FILE)
                .short('i')
                .long(options::IGNORE_FILE)
                .help("read ignore word list from FILE")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ONLY_FILE)
                .short('o')
                .long(options::ONLY_FILE)
                .help("read only word list from this FILE")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::REFERENCES)
                .short('r')
                .long(options::REFERENCES)
                .help("first field of each line is a reference")
                .value_name("FILE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .help("output width in columns, reference excluded")
                .value_name("NUMBER"),
        )
}
