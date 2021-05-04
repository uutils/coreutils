//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Boden Garman <bpgarman@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) fpath

#[macro_use]
extern crate uucore;

mod count_bytes;
mod countable;
mod wordcount;
use count_bytes::count_bytes_fast;
use countable::WordCountable;
use wordcount::{TitledWordCount, WordCount};

use clap::{App, Arg, ArgMatches};
use thiserror::Error;

use std::cmp::max;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::str::from_utf8;

#[derive(Error, Debug)]
pub enum WcError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("Expected a file, found directory {0}")]
    IsDirectory(String),
}

type WcResult<T> = Result<T, WcError>;

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
            show_bytes: matches.is_present(options::BYTES),
            show_chars: matches.is_present(options::CHAR),
            show_lines: matches.is_present(options::LINES),
            show_words: matches.is_present(options::WORDS),
            show_max_line_length: matches.is_present(options::MAX_LINE_LENGTH),
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

    fn number_enabled(&self) -> u32 {
        let mut result = 0;
        result += self.show_bytes as u32;
        result += self.show_chars as u32;
        result += self.show_lines as u32;
        result += self.show_max_line_length as u32;
        result += self.show_words as u32;
        result
    }
}

static ABOUT: &str = "Display newline, word, and byte counts for each FILE, and a total line if
more than one FILE is specified.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod options {
    pub static BYTES: &str = "bytes";
    pub static CHAR: &str = "chars";
    pub static LINES: &str = "lines";
    pub static MAX_LINE_LENGTH: &str = "max-line-length";
    pub static WORDS: &str = "words";
}

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
            Arg::with_name(options::BYTES)
                .short("c")
                .long(options::BYTES)
                .help("print the byte counts"),
        )
        .arg(
            Arg::with_name(options::CHAR)
                .short("m")
                .long(options::CHAR)
                .help("print the character counts"),
        )
        .arg(
            Arg::with_name(options::LINES)
                .short("l")
                .long(options::LINES)
                .help("print the newline counts"),
        )
        .arg(
            Arg::with_name(options::MAX_LINE_LENGTH)
                .short("L")
                .long(options::MAX_LINE_LENGTH)
                .help("print the length of the longest line"),
        )
        .arg(
            Arg::with_name(options::WORDS)
                .short("w")
                .long(options::WORDS)
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

    if wc(files, &settings).is_ok() {
        0
    } else {
        1
    }
}

const CR: u8 = b'\r';
const LF: u8 = b'\n';
const SPACE: u8 = b' ';
const TAB: u8 = b'\t';
const SYN: u8 = 0x16_u8;
const FF: u8 = 0x0C_u8;

#[inline(always)]
fn is_word_separator(byte: u8) -> bool {
    byte == SPACE || byte == TAB || byte == CR || byte == SYN || byte == FF
}

fn word_count_from_reader<T: WordCountable>(
    mut reader: T,
    settings: &Settings,
    path: &str,
) -> WcResult<WordCount> {
    let only_count_bytes = settings.show_bytes
        && (!(settings.show_chars
            || settings.show_lines
            || settings.show_max_line_length
            || settings.show_words));
    if only_count_bytes {
        return Ok(WordCount {
            bytes: count_bytes_fast(&mut reader)?,
            ..WordCount::default()
        });
    }

    // we do not need to decode the byte stream if we're only counting bytes/newlines
    let decode_chars = settings.show_chars || settings.show_words || settings.show_max_line_length;

    let mut line_count: usize = 0;
    let mut word_count: usize = 0;
    let mut byte_count: usize = 0;
    let mut char_count: usize = 0;
    let mut longest_line_length: usize = 0;
    let mut ends_lf: bool;

    // reading from a TTY seems to raise a condition on, rather than return Some(0) like a file.
    // hence the option wrapped in a result here
    for line_result in reader.lines() {
        let raw_line = match line_result {
            Ok(l) => l,
            Err(e) => {
                show_warning!("Error while reading {}: {}", path, e);
                continue;
            }
        };

        // GNU 'wc' only counts lines that end in LF as lines
        ends_lf = *raw_line.last().unwrap() == LF;
        line_count += ends_lf as usize;

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
                // -L is a GNU 'wc' extension so same behavior on LF
                longest_line_length = current_char_count - (ends_lf as usize);
            }
        }
    }

    Ok(WordCount {
        bytes: byte_count,
        chars: char_count,
        lines: line_count,
        words: word_count,
        max_line_length: longest_line_length,
    })
}

fn word_count_from_path(path: &str, settings: &Settings) -> WcResult<WordCount> {
    if path == "-" {
        let stdin = io::stdin();
        let stdin_lock = stdin.lock();
        word_count_from_reader(stdin_lock, settings, path)
    } else {
        let path_obj = Path::new(path);
        if path_obj.is_dir() {
            Err(WcError::IsDirectory(path.to_owned()))
        } else {
            let file = File::open(path)?;
            word_count_from_reader(file, settings, path)
        }
    }
}

fn wc(files: Vec<String>, settings: &Settings) -> Result<(), u32> {
    let mut total_word_count = WordCount::default();
    let mut results = vec![];
    let mut max_width: usize = 0;
    let mut error_count = 0;

    let num_files = files.len();

    for path in &files {
        let word_count = word_count_from_path(&path, settings).unwrap_or_else(|err| {
            show_error!("{}", err);
            error_count += 1;
            WordCount::default()
        });
        // Compute the number of digits needed to display the number
        // of bytes in the file. Even if the settings indicate that we
        // won't *display* the number of bytes, we still use the
        // number of digits in the byte count as the width when
        // formatting each count as a string for output.
        max_width = max(max_width, word_count.bytes.to_string().len());
        total_word_count += word_count;
        results.push(word_count.with_title(path));
    }

    for result in &results {
        if let Err(err) = print_stats(settings, &result, max_width) {
            show_warning!("failed to print result for {}: {}", result.title, err);
            error_count += 1;
        }
    }

    if num_files > 1 {
        let total_result = total_word_count.with_title("total");
        if let Err(err) = print_stats(settings, &total_result, max_width) {
            show_warning!("failed to print total: {}", err);
            error_count += 1;
        }
    }

    if error_count == 0 {
        Ok(())
    } else {
        Err(error_count)
    }
}

fn print_stats(
    settings: &Settings,
    result: &TitledWordCount,
    mut min_width: usize,
) -> WcResult<()> {
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    if settings.number_enabled() <= 1 {
        // Prevent a leading space in case we only need to display a single
        // number.
        min_width = 0;
    }

    let mut is_first: bool = true;

    if settings.show_lines {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.lines, min_width)?;
        is_first = false;
    }
    if settings.show_words {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.words, min_width)?;
        is_first = false;
    }
    if settings.show_bytes {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.bytes, min_width)?;
        is_first = false;
    }
    if settings.show_chars {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.chars, min_width)?;
        is_first = false;
    }
    if settings.show_max_line_length {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(
            stdout_lock,
            "{:1$}",
            result.count.max_line_length, min_width
        )?;
    }

    if result.title == "-" {
        writeln!(stdout_lock)?;
    } else {
        writeln!(stdout_lock, " {}", result.title)?;
    }

    Ok(())
}
