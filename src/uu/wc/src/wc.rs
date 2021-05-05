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

    // Sum the WordCount for each line. Show a warning for each line
    // that results in an IO error when trying to read it.
    let total = reader
        .lines()
        .filter_map(|res| match res {
            Ok(line) => Some(line),
            Err(e) => {
                show_warning!("Error while reading {}: {}", path, e);
                None
            }
        })
        .map(|line| WordCount::from_line(&line, decode_chars))
        .sum();
    Ok(total)
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
        max_width = max(max_width, word_count.bytes.to_string().len() + 1);
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

    if settings.show_lines {
        write!(stdout_lock, "{:1$}", result.count.lines, min_width)?;
    }
    if settings.show_words {
        write!(stdout_lock, "{:1$}", result.count.words, min_width)?;
    }
    if settings.show_bytes {
        write!(stdout_lock, "{:1$}", result.count.bytes, min_width)?;
    }
    if settings.show_chars {
        write!(stdout_lock, "{:1$}", result.count.chars, min_width)?;
    }
    if settings.show_max_line_length {
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
