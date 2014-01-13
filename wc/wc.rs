#[crate_id(name="wc", vers="1.0.0", author="Boden Garman")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Boden Garman <bpgarman@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::{print, stdin, stderr, File, result};
use std::io::buffered::BufferedReader;
use std::str::from_utf8_opt;
use extra::getopts::{groups, Matches};

struct Result {
    filename: ~str,
    bytes: uint,
    chars: uint,
    lines: uint,
    words: uint,
    max_line_length: uint,
}

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        groups::optflag("c", "bytes", "print the byte counts"),
        groups::optflag("m", "chars", "print the character counts"),
        groups::optflag("l", "lines", "print the newline counts"),
        groups::optflag("L", "max-line-length", "print the length of the longest line"),
        groups::optflag("w", "words", "print the word counts"),
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer,
                   "Invalid options\n{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }
    };

    if matches.opt_present("help") {
        println!("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("");
        print(groups::usage("Print newline, word and byte counts for each FILE", opts));
        println!("");
        println!("With no FILE, or when FILE is -, read standard input.");
        return;
    }

    if (matches.opt_present("version")) {
        println!("wc 1.0.0");
        return;
    }

    let mut files = matches.free.clone();
    if files.is_empty() {
        files = ~[~"-"];
    }

    wc(files, &matches);
}

static CR: u8 = '\r' as u8;
static LF: u8 = '\n' as u8;
static SPACE: u8 = ' ' as u8;
static TAB: u8 = '\t' as u8;
static SYN: u8 = 0x16 as u8;
static FF: u8 = 0x0C as u8;

fn is_word_seperator(byte: u8) -> bool {
    byte == SPACE || byte == TAB || byte == CR || byte == SYN || byte == FF
}

pub fn wc(files: ~[~str], matches: &Matches) {
    let mut total_line_count: uint = 0;
    let mut total_word_count: uint = 0;
    let mut total_char_count: uint = 0;
    let mut total_byte_count: uint = 0;
    let mut total_longest_line_length: uint = 0;

    let mut results: ~[Result] = ~[];
    let mut max_str_len: uint = 0;

    for path in files.iter() {
        let mut reader = match open(path.to_owned()) {
            Some(f) => f,
            None => { continue }
        };

        let mut line_count: uint = 0;
        let mut word_count: uint = 0;
        let mut byte_count: uint = 0;
        let mut char_count: uint = 0;
        let mut current_char_count: uint = 0;
        let mut longest_line_length: uint = 0;

        loop {
            // reading from a TTY seems to raise a condition on, rather than return Some(0) like a file.
            // hence the option wrapped in a result here
            match result(| | reader.read_until(LF)) {
                Ok(Some(raw_line)) => {
                    // GNU 'wc' only counts lines that end in LF as lines
                    if (raw_line.iter().last().unwrap() == &LF) {
                        line_count += 1;
                    }

                    byte_count += raw_line.iter().len();

                    // try and convert the bytes to UTF-8 first
                    match from_utf8_opt(raw_line) {
                        Some(line) => {
                            word_count += line.words().len();
                            current_char_count = line.chars().len();
                            char_count += current_char_count;
                        },
                        None => {
                            word_count += raw_line.split(|&x| is_word_seperator(x)).len();
                            for byte in raw_line.iter() {
                                match byte.is_ascii() {
                                    true => {
                                        current_char_count += 1;
                                    }
                                    false => { }
                                }
                            }
                            char_count += current_char_count;
                        }
                    }

                    if (current_char_count > longest_line_length) {
                        // we subtract one here because `line.iter().len()` includes the LF
                        // matches GNU 'wc' behaviour
                        longest_line_length = current_char_count - 1;
                    }
                },
                _ => break
            }

        }

        results.push(Result {
            filename: path.clone(),
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

        if (longest_line_length > total_longest_line_length) {
            total_longest_line_length = longest_line_length;
        }

        // used for formatting
        max_str_len = total_byte_count.to_str().len();
    }

    for result in results.iter() {
        print_stats(&result.filename, result.lines, result.words, result.chars, result.bytes, result.max_line_length, matches, max_str_len);
    }

    if (files.len() > 1) {
        print_stats(&~"total", total_line_count, total_word_count, total_char_count, total_byte_count, total_longest_line_length, matches, max_str_len);
    }
}

fn print_stats(filename: &~str, line_count: uint, word_count: uint, char_count: uint,
    byte_count: uint, longest_line_length: uint, matches: &Matches, max_str_len: uint) {
    if (matches.opt_present("lines")) {
        print!("{:1$}", line_count, max_str_len);
    }
    if (matches.opt_present("words")) {
        print!("{:1$}", word_count, max_str_len);
    }
    if (matches.opt_present("bytes")) {
        print!("{:1$}", byte_count, max_str_len + 1);
    }
    if (matches.opt_present("chars")) {
        print!("{:1$}", char_count, max_str_len);
    }
    if (matches.opt_present("max-line-length")) {
        print!("{:1$}", longest_line_length, max_str_len);
    }

    // defaults
    if (!matches.opt_present("bytes") && !matches.opt_present("chars") && !matches.opt_present("lines")
        && !matches.opt_present("words") && !matches.opt_present("max-line-length")) {
        print!("{:1$}", line_count, max_str_len);
        print!("{:1$}", word_count, max_str_len + 1);
        print!("{:1$}", byte_count, max_str_len + 1);
    }

    if (*filename != ~"-") {
        println!(" {}", *filename);
    }
    else {
        println!("");
    }
}

fn open(path: ~str) -> Option<BufferedReader<~Reader>> {
    if "-" == path {
        let reader = ~stdin() as ~Reader;
        return Some(BufferedReader::new(reader));
    }

    match result(|| File::open(&std::path::Path::new(path.as_slice()))) {
        Ok(fd) => {
            let reader = ~fd as ~Reader;
            return Some(BufferedReader::new(reader));
        },
        Err(e) => {
            writeln!(&mut stderr() as &mut Writer, "wc: {0:s}: {1:s}", path, e.desc.to_str());
            os::set_exit_status(1);
        }
    }

    None
}
