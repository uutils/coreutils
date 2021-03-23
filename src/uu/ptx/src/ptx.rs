//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDOs) corasick memchr Roff trunc oset iset

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use regex::Regex;
use std::cmp;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::default::Default;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};

static NAME: &str = "ptx";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static BRIEF: &str = "Usage: ptx [OPTION]... [INPUT]...   (without -G) or: \
                 ptx -G [OPTION]... [INPUT [OUTPUT]] \n Output a permuted index, \
                 including context, of the words in the input files. \n\n Mandatory \
                 arguments to long options are mandatory for short options too.\n
                 With no FILE, or when FILE is -, read standard input. \
                Default is '-F /'.";

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
    fn default() -> Config {
        Config {
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

fn read_word_filter_file(matches: &clap::ArgMatches, option: &str) -> HashSet<String> {
    let filename = matches
        .value_of(option)
        .expect("parsing options failed!")
        .to_string();
    let reader = BufReader::new(crash_if_err!(1, File::open(filename)));
    let mut words: HashSet<String> = HashSet::new();
    for word in reader.lines() {
        words.insert(crash_if_err!(1, word));
    }
    words
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
    fn new(matches: &clap::ArgMatches, config: &Config) -> WordFilter {
        let (o, oset): (bool, HashSet<String>) = if matches.is_present(options::ONLY_FILE) {
            (true, read_word_filter_file(matches, options::ONLY_FILE))
        } else {
            (false, HashSet::new())
        };
        let (i, iset): (bool, HashSet<String>) = if matches.is_present(options::IGNORE_FILE) {
            (true, read_word_filter_file(matches, options::IGNORE_FILE))
        } else {
            (false, HashSet::new())
        };
        if matches.is_present(options::BREAK_FILE) {
            crash!(1, "-b not implemented yet");
        }
        // Ignore empty string regex from cmd-line-args
        let arg_reg: Option<String> = if matches.is_present(options::WORD_REGEXP) {
            match matches.value_of(options::WORD_REGEXP) {
                Some(v) => match v.is_empty() {
                    true => None,
                    false => Some(v.to_string()),
                },
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
        WordFilter {
            only_specified: o,
            ignore_specified: i,
            only_set: oset,
            ignore_set: iset,
            word_regex: reg,
        }
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

fn get_config(matches: &clap::ArgMatches) -> Config {
    let mut config: Config = Default::default();
    let err_msg = "parsing options failed";
    if matches.is_present(options::TRADITIONAL) {
        config.gnu_ext = false;
        config.format = OutFormat::Roff;
        config.context_regex = "[^ \t\n]+".to_owned();
    } else {
        crash!(1, "GNU extensions not implemented yet");
    }
    if matches.is_present(options::SENTENCE_REGEXP) {
        crash!(1, "-S not implemented yet");
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
    if matches.is_present(options::IGNORE_CASE) {
        config.trunc_str = matches
            .value_of(options::IGNORE_CASE)
            .expect(err_msg)
            .to_string();
    }
    if matches.is_present(options::WIDTH) {
        let width_str = matches.value_of(options::WIDTH).expect(err_msg).to_string();
        config.line_width = crash_if_err!(1, usize::from_str_radix(&width_str, 10));
    }
    if matches.is_present(options::GAP_SIZE) {
        let gap_str = matches
            .value_of(options::GAP_SIZE)
            .expect(err_msg)
            .to_string();
        config.gap_size = crash_if_err!(1, usize::from_str_radix(&gap_str, 10));
    }
    if matches.is_present(options::FORMAT_ROFF) {
        config.format = OutFormat::Roff;
    }
    if matches.is_present(options::FORMAT_TEX) {
        config.format = OutFormat::Tex;
    }
    config
}

fn read_input(input_files: &[String], config: &Config) -> HashMap<String, (Vec<String>, usize)> {
    let mut file_map: HashMap<String, (Vec<String>, usize)> = HashMap::new();
    let mut files = Vec::new();
    if input_files.is_empty() {
        files.push("-");
    } else if config.gnu_ext {
        for file in input_files {
            files.push(&file);
        }
    } else {
        files.push(&input_files[0]);
    }
    let mut lines_so_far: usize = 0;
    for filename in files {
        let reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
            Box::new(stdin())
        } else {
            let file = crash_if_err!(1, File::open(filename));
            Box::new(file)
        });
        let lines: Vec<String> = reader.lines().map(|x| crash_if_err!(1, x)).collect();
        let size = lines.len();
        file_map.insert(filename.to_owned(), (lines, lines_so_far));
        lines_so_far += size
    }
    file_map
}

fn create_word_set(
    config: &Config,
    filter: &WordFilter,
    file_map: &HashMap<String, (Vec<String>, usize)>,
) -> BTreeSet<WordRef> {
    let reg = Regex::new(&filter.word_regex).unwrap();
    let ref_reg = Regex::new(&config.context_regex).unwrap();
    let mut word_set: BTreeSet<WordRef> = BTreeSet::new();
    for (file, lines) in file_map.iter() {
        let mut count: usize = 0;
        let offs = lines.1;
        for line in &lines.0 {
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

fn get_reference(config: &Config, word_ref: &WordRef, line: &str) -> String {
    if config.auto_ref {
        format!("{}:{}", word_ref.filename, word_ref.local_line_nr + 1)
    } else if config.input_ref {
        let reg = Regex::new(&config.context_regex).unwrap();
        let (beg, end) = match reg.find(line) {
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
    all_before: &str,
    keyword: &str,
    all_after: &str,
    config: &Config,
) -> (String, String, String, String) {
    assert_eq!(all_before.trim(), all_before);
    assert_eq!(keyword.trim(), keyword);
    assert_eq!(all_after.trim(), all_after);
    let mut head = String::new();
    let mut before = String::new();
    let mut after = String::new();
    let mut tail = String::new();

    let half_line_size = cmp::max(
        (config.line_width / 2) as isize - (2 * config.trunc_str.len()) as isize,
        0,
    ) as usize;
    let max_after_size = cmp::max(half_line_size as isize - keyword.len() as isize - 1, 0) as usize;
    let max_before_size = half_line_size;
    let all_before_vec: Vec<char> = all_before.chars().collect();
    let all_after_vec: Vec<char> = all_after.chars().collect();

    // get before
    let mut bb_tmp = cmp::max(all_before.len() as isize - max_before_size as isize, 0) as usize;
    bb_tmp = trim_broken_word_left(&all_before_vec, bb_tmp, all_before.len());
    let (before_beg, before_end) = trim_idx(&all_before_vec, bb_tmp, all_before.len());
    before.push_str(&all_before[before_beg..before_end]);
    assert!(max_before_size >= before.len());

    // get after
    let mut ae_tmp = cmp::min(max_after_size, all_after.len());
    ae_tmp = trim_broken_word_right(&all_after_vec, 0, ae_tmp);
    let (after_beg, after_end) = trim_idx(&all_after_vec, 0, ae_tmp);
    after.push_str(&all_after[after_beg..after_end]);
    assert!(max_after_size >= after.len());

    // get tail
    let max_tail_size = max_before_size - before.len();
    let (tb, _) = trim_idx(&all_after_vec, after_end, all_after.len());
    let mut te_tmp = cmp::min(tb + max_tail_size, all_after.len());
    te_tmp = trim_broken_word_right(&all_after_vec, tb, te_tmp);
    let (tail_beg, tail_end) = trim_idx(&all_after_vec, tb, te_tmp);
    tail.push_str(&all_after[tail_beg..tail_end]);

    // get head
    let max_head_size = max_after_size - after.len();
    let (_, he) = trim_idx(&all_before_vec, 0, before_beg);
    let mut hb_tmp = cmp::max(he as isize - max_head_size as isize, 0) as usize;
    hb_tmp = trim_broken_word_left(&all_before_vec, hb_tmp, he);
    let (head_beg, head_end) = trim_idx(&all_before_vec, hb_tmp, he);
    head.push_str(&all_before[head_beg..head_end]);

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

    // add space before "after" if needed
    if !after.is_empty() {
        after = format!(" {}", after);
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

fn adjust_tex_str(context: &str) -> String {
    let ws_reg = Regex::new(r"[\t\n\v\f\r ]").unwrap();
    let mut fix: String = ws_reg.replace_all(context, " ").trim().to_owned();
    let mapped_chunks: Vec<String> = fix.chars().map(tex_mapper).collect();
    fix = mapped_chunks.join("");
    fix
}

fn format_tex_line(config: &Config, word_ref: &WordRef, line: &str, reference: &str) -> String {
    let mut output = String::new();
    output.push_str(&format!("\\{} ", config.macro_name));
    let all_before = if config.input_ref {
        let before = &line[0..word_ref.position];
        adjust_tex_str(before.trim().trim_start_matches(reference))
    } else {
        adjust_tex_str(&line[0..word_ref.position])
    };
    let keyword = adjust_tex_str(&line[word_ref.position..word_ref.position_end]);
    let all_after = adjust_tex_str(&line[word_ref.position_end..line.len()]);
    let (tail, before, after, head) = get_output_chunks(&all_before, &keyword, &all_after, &config);
    output.push_str(&format!(
        "{5}{0}{6}{5}{1}{6}{5}{2}{6}{5}{3}{6}{5}{4}{6}",
        tail, before, keyword, after, head, "{", "}"
    ));
    if config.auto_ref || config.input_ref {
        output.push_str(&format!("{}{}{}", "{", adjust_tex_str(&reference), "}"));
    }
    output
}

fn adjust_roff_str(context: &str) -> String {
    let ws_reg = Regex::new(r"[\t\n\v\f\r]").unwrap();
    ws_reg
        .replace_all(context, " ")
        .replace("\"", "\"\"")
        .trim()
        .to_owned()
}

fn format_roff_line(config: &Config, word_ref: &WordRef, line: &str, reference: &str) -> String {
    let mut output = String::new();
    output.push_str(&format!(".{}", config.macro_name));
    let all_before = if config.input_ref {
        let before = &line[0..word_ref.position];
        adjust_roff_str(before.trim().trim_start_matches(reference))
    } else {
        adjust_roff_str(&line[0..word_ref.position])
    };
    let keyword = adjust_roff_str(&line[word_ref.position..word_ref.position_end]);
    let all_after = adjust_roff_str(&line[word_ref.position_end..line.len()]);
    let (tail, before, after, head) = get_output_chunks(&all_before, &keyword, &all_after, &config);
    output.push_str(&format!(
        " \"{}\" \"{}\" \"{}{}\" \"{}\"",
        tail, before, keyword, after, head
    ));
    if config.auto_ref || config.input_ref {
        output.push_str(&format!(" \"{}\"", adjust_roff_str(&reference)));
    }
    output
}

fn write_traditional_output(
    config: &Config,
    file_map: &HashMap<String, (Vec<String>, usize)>,
    words: &BTreeSet<WordRef>,
    output_filename: &str,
) {
    let mut writer: BufWriter<Box<dyn Write>> = BufWriter::new(if output_filename == "-" {
        Box::new(stdout())
    } else {
        let file = crash_if_err!(1, File::create(output_filename));
        Box::new(file)
    });
    for word_ref in words.iter() {
        let file_map_value: &(Vec<String>, usize) = file_map
            .get(&(word_ref.filename))
            .expect("Missing file in file map");
        let (ref lines, _) = *(file_map_value);
        let reference = get_reference(config, word_ref, &lines[word_ref.local_line_nr]);
        let output_line: String = match config.format {
            OutFormat::Tex => {
                format_tex_line(config, word_ref, &lines[word_ref.local_line_nr], &reference)
            }
            OutFormat::Roff => {
                format_roff_line(config, word_ref, &lines[word_ref.local_line_nr], &reference)
            }
            OutFormat::Dumb => crash!(1, "There is no dumb format with GNU extensions disabled"),
        };
        crash_if_err!(1, writeln!(writer, "{}", output_line));
    }
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

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    // let mut opts = Options::new();
    let matches = App::new(executable!())
        .name(NAME)
        .version(VERSION)
        .usage(BRIEF)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
        .arg(
            Arg::with_name(options::AUTO_REFERENCE)
                .short("A")
                .long(options::AUTO_REFERENCE)
                .help("output automatically generated references")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::TRADITIONAL)
                .short("G")
                .long(options::TRADITIONAL)
                .help("behave more like System V 'ptx'"),
        )
        .arg(
            Arg::with_name(options::FLAG_TRUNCATION)
                .short("F")
                .long(options::FLAG_TRUNCATION)
                .help("use STRING for flagging line truncations")
                .value_name("STRING")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::MACRO_NAME)
                .short("M")
                .long(options::MACRO_NAME)
                .help("macro name to use instead of 'xx'")
                .value_name("STRING")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::FORMAT_ROFF)
                .short("O")
                .long(options::FORMAT_ROFF)
                .help("generate output as roff directives"),
        )
        .arg(
            Arg::with_name(options::RIGHT_SIDE_REFS)
                .short("R")
                .long(options::RIGHT_SIDE_REFS)
                .help("put references at right, not counted in -w")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::SENTENCE_REGEXP)
                .short("S")
                .long(options::SENTENCE_REGEXP)
                .help("for end of lines or end of sentences")
                .value_name("REGEXP")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::FORMAT_TEX)
                .short("T")
                .long(options::FORMAT_TEX)
                .help("generate output as TeX directives"),
        )
        .arg(
            Arg::with_name(options::WORD_REGEXP)
                .short("W")
                .long(options::WORD_REGEXP)
                .help("use REGEXP to match each keyword")
                .value_name("REGEXP")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::BREAK_FILE)
                .short("b")
                .long(options::BREAK_FILE)
                .help("word break characters in this FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::IGNORE_CASE)
                .short("f")
                .long(options::IGNORE_CASE)
                .help("fold lower case to upper case for sorting")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::GAP_SIZE)
                .short("g")
                .long(options::GAP_SIZE)
                .help("gap size in columns between output fields")
                .value_name("NUMBER")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::IGNORE_FILE)
                .short("i")
                .long(options::IGNORE_FILE)
                .help("read ignore word list from FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::ONLY_FILE)
                .short("o")
                .long(options::ONLY_FILE)
                .help("read only word list from this FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::REFERENCES)
                .short("r")
                .long(options::REFERENCES)
                .help("first field of each line is a reference")
                .value_name("FILE")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::WIDTH)
                .short("w")
                .long(options::WIDTH)
                .help("output width in columns, reference excluded")
                .value_name("NUMBER")
                .takes_value(true),
        )
        .get_matches_from(args);

    let input_files: Vec<String> = match &matches.values_of(options::FILE) {
        Some(v) => v.clone().map(|v| v.to_owned()).collect(),
        None => vec!["-".to_string()],
    };

    let config = get_config(&matches);
    let word_filter = WordFilter::new(&matches, &config);
    let file_map = read_input(&input_files, &config);
    let word_set = create_word_set(&config, &word_filter, &file_map);
    let output_file = if !config.gnu_ext && matches.args.len() == 2 {
        matches.value_of(options::FILE).unwrap_or("-").to_string()
    } else {
        "-".to_owned()
    };
    write_traditional_output(&config, &file_map, &word_set, &output_file);
    0
}
