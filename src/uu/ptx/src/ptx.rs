//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDOs) corasick memchr Roff trunc oset iset

extern crate aho_corasick;
extern crate getopts;
extern crate memchr;
extern crate regex;
extern crate regex_syntax;

#[macro_use]
extern crate uucore;

use getopts::{Matches, Options};
use regex::Regex;
use std::cmp;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::default::Default;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};

static NAME: &str = "ptx";
static VERSION: &str = env!("CARGO_PKG_VERSION");

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

fn read_word_filter_file(matches: &Matches, option: &str) -> HashSet<String> {
    let filename = matches.opt_str(option).expect("parsing options failed!");
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
    fn new(matches: &Matches, config: &Config) -> WordFilter {
        let (o, oset): (bool, HashSet<String>) = if matches.opt_present("o") {
            (true, read_word_filter_file(matches, "o"))
        } else {
            (false, HashSet::new())
        };
        let (i, iset): (bool, HashSet<String>) = if matches.opt_present("i") {
            (true, read_word_filter_file(matches, "i"))
        } else {
            (false, HashSet::new())
        };
        if matches.opt_present("b") {
            crash!(1, "-b not implemented yet");
        }
        let reg = if matches.opt_present("W") {
            matches.opt_str("W").expect("parsing options failed!")
        } else if config.gnu_ext {
            "\\w+".to_owned()
        } else {
            "[^ \t\n]+".to_owned()
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

fn print_version() {
    println!("{} {}", NAME, VERSION);
}

fn print_usage(opts: &Options) {
    let brief = "Usage: ptx [OPTION]... [INPUT]...   (without -G) or: \
                 ptx -G [OPTION]... [INPUT [OUTPUT]] \n Output a permuted index, \
                 including context, of the words in the input files. \n\n Mandatory \
                 arguments to long options are mandatory for short options too.";
    let explanation = "With no FILE, or when FILE is -, read standard input. \
                        Default is '-F /'.";
    println!("{}\n{}", opts.usage(&brief), explanation);
}

fn get_config(matches: &Matches) -> Config {
    let mut config: Config = Default::default();
    let err_msg = "parsing options failed";
    if matches.opt_present("G") {
        config.gnu_ext = false;
        config.format = OutFormat::Roff;
        config.context_regex = "[^ \t\n]+".to_owned();
    } else {
        crash!(1, "GNU extensions not implemented yet");
    }
    if matches.opt_present("S") {
        crash!(1, "-S not implemented yet");
    }
    config.auto_ref = matches.opt_present("A");
    config.input_ref = matches.opt_present("r");
    config.right_ref &= matches.opt_present("R");
    config.ignore_case = matches.opt_present("f");
    if matches.opt_present("M") {
        config.macro_name = matches.opt_str("M").expect(err_msg);
    }
    if matches.opt_present("F") {
        config.trunc_str = matches.opt_str("F").expect(err_msg);
    }
    if matches.opt_present("w") {
        let width_str = matches.opt_str("w").expect(err_msg);
        config.line_width = crash_if_err!(1, usize::from_str_radix(&width_str, 10));
    }
    if matches.opt_present("g") {
        let gap_str = matches.opt_str("g").expect(err_msg);
        config.gap_size = crash_if_err!(1, usize::from_str_radix(&gap_str, 10));
    }
    if matches.opt_present("O") {
        config.format = OutFormat::Roff;
    }
    if matches.opt_present("T") {
        config.format = OutFormat::Tex;
    }
    config
}

struct FileContent {
    lines: Vec<String>,
    chars_lines: Vec<Vec<char>>,
    offset: usize,
}

type FileMap = HashMap<String, FileContent>;

fn read_input(input_files: &[String], config: &Config) -> FileMap {
    let mut file_map: FileMap = HashMap::new();
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
    let mut offset: usize = 0;
    for filename in files {
        let reader: BufReader<Box<dyn Read>> = BufReader::new(if filename == "-" {
            Box::new(stdin())
        } else {
            let file = crash_if_err!(1, File::open(filename));
            Box::new(file)
        });
        let lines: Vec<String> = reader.lines().map(|x| crash_if_err!(1, x)).collect();
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
        offset += size
    }
    file_map
}

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
        format!("{}:{}", word_ref.filename, word_ref.local_line_nr + 1)
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
    let half_line_size = (config.line_width / 2) as usize;
    let max_before_size = cmp::max(half_line_size as isize - config.gap_size as isize, 0) as usize;
    let max_after_size = cmp::max(
        half_line_size as isize
            - (2 * config.trunc_str.len()) as isize
            - keyword.len() as isize
            - 1,
        0,
    ) as usize;

    let mut head = String::with_capacity(half_line_size);
    let mut before = String::with_capacity(half_line_size);
    let mut after = String::with_capacity(half_line_size);
    let mut tail = String::with_capacity(half_line_size);

    // get before
    let (_, be) = trim_idx(all_before, 0, all_before.len());
    let mut bb_tmp = cmp::max(be as isize - max_before_size as isize, 0) as usize;
    bb_tmp = trim_broken_word_left(all_before, bb_tmp, be);
    let (before_beg, before_end) = trim_idx(all_before, bb_tmp, be);
    let before_str: String = all_before[before_beg..before_end].iter().collect();
    before.push_str(&before_str);
    assert!(max_before_size >= before.len());

    // get after
    let mut ae_tmp = cmp::min(max_after_size, all_after.len());
    ae_tmp = trim_broken_word_right(all_after, 0, ae_tmp);
    let (_, after_end) = trim_idx(all_after, 0, ae_tmp);
    let after_str: String = all_after[0..after_end].iter().collect();
    after.push_str(&after_str);
    assert!(max_after_size >= after.len());

    // get tail
    let max_tail_size = cmp::max(
        max_before_size as isize - before.len() as isize - config.gap_size as isize,
        0,
    ) as usize;
    let (tb, _) = trim_idx(all_after, after_end, all_after.len());
    let mut te_tmp = cmp::min(all_after.len(), tb + max_tail_size) as usize;
    te_tmp = trim_broken_word_right(
        all_after,
        tb,
        cmp::max(te_tmp as isize - 1, tb as isize) as usize,
    );
    let (tail_beg, tail_end) = trim_idx(all_after, tb, te_tmp);
    let tail_str: String = all_after[tail_beg..tail_end].iter().collect();
    tail.push_str(&tail_str);

    // get head
    let max_head_size = cmp::max(
        max_after_size as isize - after.len() as isize - config.gap_size as isize,
        0,
    ) as usize;
    let (_, he) = trim_idx(all_before, 0, before_beg);
    let hb_tmp = trim_broken_word_left(
        all_before,
        cmp::max(he as isize - max_head_size as isize, 0) as usize,
        he,
    );
    let (head_beg, head_end) = trim_idx(all_before, hb_tmp, he);
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
        let before_start_trimoff =
            word_ref.position - before.trim_start_matches(reference).trim_start().len();
        let before_end_index = before.len();
        &chars_line[before_start_trimoff..cmp::max(before_end_index, before_start_trimoff)]
    } else {
        let before_chars_trim_idx = (0, word_ref.position);
        &chars_line[before_chars_trim_idx.0..before_chars_trim_idx.1]
    };
    let keyword = &line[word_ref.position..word_ref.position_end];
    let after_chars_trim_idx = (word_ref.position_end, chars_line.len());
    let all_after = &chars_line[after_chars_trim_idx.0..after_chars_trim_idx.1];
    let (tail, before, after, head) = get_output_chunks(&all_before, &keyword, &all_after, &config);
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
        output.push_str(&format!("{}{}{}", "{", format_tex_field(&reference), "}"));
    }
    output
}

fn format_roff_field(s: &str) -> String {
    s.replace("\"", "\"\"")
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
        let before_start_trimoff =
            word_ref.position - before.trim_start_matches(reference).trim_start().len();
        let before_end_index = before.len();
        &chars_line[before_start_trimoff..cmp::max(before_end_index, before_start_trimoff)]
    } else {
        let before_chars_trim_idx = (0, word_ref.position);
        &chars_line[before_chars_trim_idx.0..before_chars_trim_idx.1]
    };
    let keyword = &line[word_ref.position..word_ref.position_end];
    let after_chars_trim_idx = (word_ref.position_end, chars_line.len());
    let all_after = &chars_line[after_chars_trim_idx.0..after_chars_trim_idx.1];
    let (tail, before, after, head) = get_output_chunks(&all_before, &keyword, &all_after, &config);
    output.push_str(&format!(
        " \"{}\" \"{}\" \"{}{}\" \"{}\"",
        format_roff_field(&tail),
        format_roff_field(&before),
        format_roff_field(keyword),
        format_roff_field(&after),
        format_roff_field(&head)
    ));
    if config.auto_ref || config.input_ref {
        output.push_str(&format!(" \"{}\"", format_roff_field(&reference)));
    }
    output
}

fn write_traditional_output(
    config: &Config,
    file_map: &FileMap,
    words: &BTreeSet<WordRef>,
    output_filename: &str,
) {
    let mut writer: BufWriter<Box<dyn Write>> = BufWriter::new(if output_filename == "-" {
        Box::new(stdout())
    } else {
        let file = crash_if_err!(1, File::create(output_filename));
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
            OutFormat::Dumb => crash!(1, "There is no dumb format with GNU extensions disabled"),
        };
        crash_if_err!(1, writeln!(writer, "{}", output_line));
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let mut opts = Options::new();
    opts.optflag(
        "A",
        "auto-reference",
        "output automatically generated references",
    );
    opts.optflag("G", "traditional", "behave more like System V 'ptx'");
    opts.optopt(
        "F",
        "flag-truncation",
        "use STRING for flagging line truncations",
        "STRING",
    );
    opts.optopt(
        "M",
        "macro-name",
        "macro name to use instead of 'xx'",
        "STRING",
    );
    opts.optflag("O", "format=roff", "generate output as roff directives");
    opts.optflag(
        "R",
        "right-side-refs",
        "put references at right, not counted in -w",
    );
    opts.optopt(
        "S",
        "sentence-regexp",
        "for end of lines or end of sentences",
        "REGEXP",
    );
    opts.optflag("T", "format=tex", "generate output as TeX directives");
    opts.optopt(
        "W",
        "word-regexp",
        "use REGEXP to match each keyword",
        "REGEXP",
    );
    opts.optopt(
        "b",
        "break-file",
        "word break characters in this FILE",
        "FILE",
    );
    opts.optflag(
        "f",
        "ignore-case",
        "fold lower case to upper case for sorting",
    );
    opts.optopt(
        "g",
        "gap-size",
        "gap size in columns between output fields",
        "NUMBER",
    );
    opts.optopt(
        "i",
        "ignore-file",
        "read ignore word list from FILE",
        "FILE",
    );
    opts.optopt(
        "o",
        "only-file",
        "read only word list from this FILE",
        "FILE",
    );
    opts.optflag("r", "references", "first field of each line is a reference");
    opts.optopt(
        "w",
        "width",
        "output width in columns, reference excluded",
        "NUMBER",
    );
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = return_if_err!(1, opts.parse(&args[1..]));

    if matches.opt_present("help") {
        print_usage(&opts);
        return 0;
    }
    if matches.opt_present("version") {
        print_version();
        return 0;
    }
    let config = get_config(&matches);
    let word_filter = WordFilter::new(&matches, &config);
    let file_map = read_input(&matches.free, &config);
    let word_set = create_word_set(&config, &word_filter, &file_map);
    let output_file = if !config.gnu_ext && matches.free.len() == 2 {
        matches.free[1].clone()
    } else {
        "-".to_owned()
    };
    write_traditional_output(&config, &file_map, &word_set, &output_file);
    0
}
