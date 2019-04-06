#![crate_name = "uu_wc"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Boden Garman <bpgarman@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use itertools::Itertools;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use std::path::Path;
use std::result::Result as StdResult;
use std::str::from_utf8;
use structopt::*;
use uucore::{executable, show_error, show_info, show_warning};

#[derive(StructOpt)]
struct Settings {
    #[structopt(short = "c", long)]
    bytes: bool,
    #[structopt(short = "m", long)]
    chars: bool,
    #[structopt(short = "l", long)]
    lines: bool,
    #[structopt(short = "w", long)]
    words: bool,
    #[structopt(short = "L")]
    max_line_length: bool,
    files: Vec<String>,
}

struct Outcome<'a> {
    title: &'a str,
    bytes: usize,
    chars: usize,
    lines: usize,
    words: usize,
    max_line_length: usize,
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut settings = Settings::from_iter(args.into_iter());

    // If no options are passed, we need to set bytes, lines, and words.
    if !(settings.bytes
        || settings.chars
        || settings.lines
        || settings.words
        || settings.max_line_length)
    {
        settings.bytes = true;
        settings.lines = true;
        settings.words = true;
    }

    if settings.files.is_empty() {
        settings.files.push("-".to_owned());
    }

    wc(settings).err().unwrap_or(0)
}

const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;
const SPACE: u8 = ' ' as u8;
const TAB: u8 = '\t' as u8;
const SYN: u8 = 0x16 as u8;
const FF: u8 = 0x0C as u8;

#[inline(always)]
fn is_word_seperator(byte: u8) -> bool {
    byte == SPACE || byte == TAB || byte == CR || byte == SYN || byte == FF
}

fn wc_reader<'a>(path: &'a str, mut reader: impl BufRead) -> Outcome<'a> {
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

        // try and convert the bytes to UTF-8 first
        let current_char_count = match from_utf8(&raw_line[..]) {
            Ok(line) => {
                word_count += line.split_whitespace().count();
                line.chars().count()
            }
            Err(..) => {
                word_count += raw_line.split(|&x| is_word_seperator(x)).count();
                raw_line.iter().filter(|c| c.is_ascii()).count()
            }
        };
        char_count += current_char_count;

        if current_char_count > longest_line_length {
            // We subtract one here because `line.len()` includes the LF.
            // This matches GNU 'wc' behaviour.
            longest_line_length = current_char_count - 1;
        }

        raw_line.truncate(0);
    }

    Outcome {
        title: path,
        bytes: byte_count,
        chars: char_count,
        lines: line_count,
        words: word_count,
        max_line_length: longest_line_length,
    }
}

fn wc(settings: Settings) -> StdResult<(), i32> {
    let (outcomes, total) = settings
        .files
        .iter()
        .map(|path| {
            if path == "-" {
                Ok(wc_reader(path, stdin().lock()))
            } else {
                open(&path).map(|reader| wc_reader(path, reader))
            }
        })
        .fold_results(
            (
                vec![],
                Outcome {
                    title: "total",
                    bytes: 0,
                    chars: 0,
                    lines: 0,
                    words: 0,
                    max_line_length: 0,
                },
            ),
            |(mut outcomes, mut total), item| {
                total.bytes += item.bytes;
                total.chars += item.chars;
                total.lines += item.lines;
                total.words += item.words;
                total.max_line_length = std::cmp::max(total.max_line_length, item.max_line_length);
                outcomes.push(item);
                (outcomes, total)
            },
        )?;

    let max_width = total.bytes.to_string().len() + 1;

    for outcome in &outcomes {
        print_stats(&settings, &outcome, max_width);
    }

    if settings.files.len() > 1 {
        print_stats(&settings, &total, max_width);
    }

    Ok(())
}

fn print_stats(settings: &Settings, outcome: &Outcome, max_width: usize) {
    if settings.lines {
        print!("{:1$}", outcome.lines, max_width);
    }
    if settings.words {
        print!("{:1$}", outcome.words, max_width);
    }
    if settings.bytes {
        print!("{:1$}", outcome.bytes, max_width);
    }
    if settings.chars {
        print!("{:1$}", outcome.chars, max_width);
    }
    if settings.max_line_length {
        print!("{:1$}", outcome.max_line_length, max_width);
    }

    if outcome.title != "-" {
        println!(" {}", outcome.title);
    } else {
        println!("");
    }
}

fn open(path: &str) -> StdResult<BufReader<File>, i32> {
    let fpath = Path::new(path);
    if fpath.is_dir() {
        show_info!("{}: is a directory", path);
    }
    File::open(&fpath).map(BufReader::new).map_err(|e| {
        show_error!("wc: {}: {}", path, e);
        1
    })
}
