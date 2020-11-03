//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Boden Garman <bpgarman@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) fpath

extern crate clap;
#[macro_use]
extern crate uucore;

use clap::{App, Arg, ArgMatches};

use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::Path;
use std::result::Result as StdResult;
use std::str::from_utf8;

struct Settings {
    show_bytes: bool,
    show_chars: bool,
    show_lines: bool,
    show_words: bool,
    show_max_line_length: bool,
}

impl Settings {
    fn new(matches: &ArgMatches) -> Settings {
        let settings = Settings {
            show_bytes: matches.is_present(OPT_BYTES),
            show_chars: matches.is_present(OPT_CHAR),
            show_lines: matches.is_present(OPT_LINES),
            show_words: matches.is_present(OPT_WORDS),
            show_max_line_length: matches.is_present(OPT_MAX_LINE_LENGTH),
        };

        if settings.show_bytes
            || settings.show_chars
            || settings.show_lines
            || settings.show_words
            || settings.show_max_line_length
        {
            return settings;
        }

        Settings {
            show_bytes: true,
            show_chars: false,
            show_lines: true,
            show_words: true,
            show_max_line_length: false,
        }
    }
}

struct Result {
    title: String,
    bytes: usize,
    chars: usize,
    lines: usize,
    words: usize,
    max_line_length: usize,
}

static ABOUT: &str = "Display newline, word, and byte counts for each FILE, and a total line if
more than one FILE is specified.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_BYTES: &str = "bytes";
static OPT_CHAR: &str = "chars";
static OPT_LINES: &str = "lines";
static OPT_MAX_LINE_LENGTH: &str = "max-line-length";
static OPT_WORDS: &str = "words";

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... [FILE]...
 With no FILE, or when FILE is -, read standard input.",
        executable!()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_BYTES)
                .short("c")
                .long(OPT_BYTES)
                .help("print the byte counts"),
        )
        .arg(
            Arg::with_name(OPT_CHAR)
                .short("m")
                .long(OPT_CHAR)
                .help("print the character counts"),
        )
        .arg(
            Arg::with_name(OPT_LINES)
                .short("l")
                .long(OPT_LINES)
                .help("print the newline counts"),
        )
        .arg(
            Arg::with_name(OPT_MAX_LINE_LENGTH)
                .short("L")
                .long(OPT_MAX_LINE_LENGTH)
                .help("print the length of the longest line"),
        )
        .arg(
            Arg::with_name(OPT_WORDS)
                .short("w")
                .long(OPT_WORDS)
                .help("print the word counts"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
        .get_matches_from(args);

    let mut files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        files.push("-".to_owned());
    }

    let settings = Settings::new(&matches);

    match wc(files, &settings) {
        Ok(()) => ( /* pass */ ),
        Err(e) => return e,
    }

    0
}

const CR: u8 = b'\r';
const LF: u8 = b'\n';
const SPACE: u8 = b' ';
const TAB: u8 = b'\t';
const SYN: u8 = 0x16 as u8;
const FF: u8 = 0x0C as u8;

#[inline(always)]
fn is_word_separator(byte: u8) -> bool {
    byte == SPACE || byte == TAB || byte == CR || byte == SYN || byte == FF
}

fn wc(files: Vec<String>, settings: &Settings) -> StdResult<(), i32> {
    let mut total_line_count: usize = 0;
    let mut total_word_count: usize = 0;
    let mut total_char_count: usize = 0;
    let mut total_byte_count: usize = 0;
    let mut total_longest_line_length: usize = 0;

    let mut results = vec![];
    let mut max_width: usize = 0;

    // we do not need to decode the byte stream if we're only counting bytes/newlines
    let decode_chars = settings.show_chars || settings.show_words || settings.show_max_line_length;

    for path in &files {
        let mut reader = open(&path[..])?;

        let mut line_count: usize = 0;
        let mut word_count: usize = 0;
        let mut byte_count: usize = 0;
        let mut char_count: usize = 0;
        let mut longest_line_length: usize = 0;
        let mut raw_line = Vec::new();

        // reading from a TTY seems to raise a condition on, rather than return Some(0) like a file.
        // hence the option wrapped in a result here
        while match reader.read_until(LF, &mut raw_line) {
            Ok(n) if n > 0 => true,
            Err(ref e) if !raw_line.is_empty() => {
                show_warning!("Error while reading {}: {}", path, e);
                !raw_line.is_empty()
            }
            _ => false,
        } {
            // GNU 'wc' only counts lines that end in LF as lines
            if *raw_line.last().unwrap() == LF {
                line_count += 1;
            }

            byte_count += raw_line.len();

            if decode_chars {
                // try and convert the bytes to UTF-8 first
                let current_char_count;
                match from_utf8(&raw_line[..]) {
                    Ok(line) => {
                        word_count += line.split_whitespace().count();
                        current_char_count = line.chars().count();
                    }
                    Err(..) => {
                        word_count += raw_line.split(|&x| is_word_separator(x)).count();
                        current_char_count = raw_line.iter().filter(|c| c.is_ascii()).count()
                    }
                }
                char_count += current_char_count;

                if current_char_count > longest_line_length {
                    // we subtract one here because `line.len()` includes the LF
                    // matches GNU 'wc' behavior
                    longest_line_length = current_char_count - 1;
                }
            }

            raw_line.truncate(0);
        }

        results.push(Result {
            title: path.clone(),
            bytes: byte_count,
            chars: char_count,
            lines: line_count,
            words: word_count,
            max_line_length: longest_line_length,
        });

        total_line_count += line_count;
        total_word_count += word_count;
        total_char_count += char_count;
        total_byte_count += byte_count;

        if longest_line_length > total_longest_line_length {
            total_longest_line_length = longest_line_length;
        }

        // used for formatting
        max_width = total_byte_count.to_string().len() + 1;
    }

    for result in &results {
        print_stats(settings, &result, max_width);
    }

    if files.len() > 1 {
        let result = Result {
            title: "total".to_owned(),
            bytes: total_byte_count,
            chars: total_char_count,
            lines: total_line_count,
            words: total_word_count,
            max_line_length: total_longest_line_length,
        };
        print_stats(settings, &result, max_width);
    }

    Ok(())
}

fn print_stats(settings: &Settings, result: &Result, max_width: usize) {
    if settings.show_lines {
        print!("{:1$}", result.lines, max_width);
    }
    if settings.show_words {
        print!("{:1$}", result.words, max_width);
    }
    if settings.show_bytes {
        print!("{:1$}", result.bytes, max_width);
    }
    if settings.show_chars {
        print!("{:1$}", result.chars, max_width);
    }
    if settings.show_max_line_length {
        print!("{:1$}", result.max_line_length, max_width);
    }

    if result.title != "-" {
        println!(" {}", result.title);
    } else {
        println!();
    }
}

fn open(path: &str) -> StdResult<BufReader<Box<dyn Read + 'static>>, i32> {
    if "-" == path {
        let reader = Box::new(stdin()) as Box<dyn Read>;
        return Ok(BufReader::new(reader));
    }

    let fpath = Path::new(path);
    if fpath.is_dir() {
        show_info!("{}: is a directory", path);
    }
    match File::open(&fpath) {
        Ok(fd) => {
            let reader = Box::new(fd) as Box<dyn Read>;
            Ok(BufReader::new(reader))
        }
        Err(e) => {
            show_error!("wc: {}: {}", path, e);
            Err(1)
        }
    }
}
