#![crate_name = "uu_wc"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Boden Garman <bpgarman@gmail.com>
 * (c) Árni Dagur <arni@dagur.eu>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;
extern crate getopts;

#[cfg(unix)]
extern crate libc;
#[cfg(unix)]
extern crate nix;

use getopts::{Matches, Options};
use std::fs::{File, OpenOptions};
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::Path;
use std::result::Result as StdResult;
use std::str::from_utf8;

#[cfg(unix)]
use libc::S_IFREG;
#[cfg(unix)]
use nix::sys::stat::fstat;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};

#[cfg(any(target_os = "linux", target_os = "android"))]
use libc::S_IFIFO;
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::fcntl::{splice, SpliceFFlags};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::unistd::pipe;

struct Settings {
    show_bytes: bool,
    show_chars: bool,
    show_lines: bool,
    show_words: bool,
    show_max_line_length: bool,
}

impl Settings {
    fn new(matches: &Matches) -> Settings {
        let settings = Settings {
            show_bytes: matches.opt_present("bytes"),
            show_chars: matches.opt_present("chars"),
            show_lines: matches.opt_present("lines"),
            show_words: matches.opt_present("words"),
            show_max_line_length: matches.opt_present("L"),
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

struct InputHandle {
    #[cfg(unix)]
    file_descriptor: RawFd,
    reader: Box<Read>,
}

static NAME: &str = "wc";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("c", "bytes", "print the byte counts");
    opts.optflag("m", "chars", "print the character counts");
    opts.optflag("l", "lines", "print the newline counts");
    opts.optflag(
        "L",
        "max-line-length",
        "print the length of the longest line",
    );
    opts.optflag("w", "words", "print the word counts");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let mut matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f),
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", NAME);
        println!("");
        println!(
            "{}",
            opts.usage("Print newline, word and byte counts for each FILE")
        );
        println!("With no FILE, or when FILE is -, read standard input.");
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if matches.free.is_empty() {
        matches.free.push("-".to_owned());
    }

    let settings = Settings::new(&matches);

    match wc(matches.free, &settings) {
        Ok(()) => ( /* pass */ ),
        Err(e) => return e,
    }

    0
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

#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_bytes_using_splice(fd: RawFd) -> nix::Result<usize> {
    const BUF_SIZE: usize = 16 * 1024;

    let null_file = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .map_err(|_| nix::Error::last())?;
    let null = null_file.as_raw_fd();
    let (pipe_rd, pipe_wr) = pipe()?;

    let mut byte_count = 0;
    loop {
        let res = splice(fd, None, pipe_wr, None, BUF_SIZE, SpliceFFlags::empty())?;
        if res == 0 {
            break;
        }
        let res = splice(pipe_rd, None, null, None, BUF_SIZE, SpliceFFlags::empty())?;
        byte_count += res;
    }

    Ok(byte_count)
}

/// In the special case where we only need to count the number of
/// bytes. There are several optimizations we can do.
#[inline]
#[cfg(unix)]
fn count_bytes_fast(fd: RawFd) -> Option<usize> {
    match fstat(fd) {
        Ok(stat) => {
            // If the file is regular, then the `st_size` should hold
            // the file's size in bytes.
            if (stat.st_mode & S_IFREG) != 0 {
                return Some(stat.st_size as usize);
            }
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                // Else, if we're on Linux and our file is a FIFO pipe
                // (or stdin), we use splice to count the number of bytes.
                if (stat.st_mode & S_IFIFO) != 0 {
                    if let Ok(n) = count_bytes_using_splice(fd) {
                        return Some(n);
                    }
                }
            }
        }
        _ => {}
    }
    // We couldn't determine the number of bytes using one of our optimizations
    // fall back on read()...
    None
}

fn wc(files: Vec<String>, settings: &Settings) -> StdResult<(), i32> {
    let mut total_line_count: usize = 0;
    let mut total_word_count: usize = 0;
    let mut total_char_count: usize = 0;
    let mut total_byte_count: usize = 0;
    let mut total_longest_line_length: usize = 0;

    let mut results = vec![];
    let mut max_width: usize = 0;

    #[cfg(unix)]
    let only_need_to_count_bytes = settings.show_bytes
        && (!(settings.show_chars
            || settings.show_lines
            || settings.show_max_line_length
            || settings.show_words));

    for path in &files {
        let handle = open(&path[..])?;

        let mut line_count: usize = 0;
        let mut word_count: usize = 0;
        let mut byte_count: usize = 0;
        let mut char_count: usize = 0;
        let mut longest_line_length: usize = 0;
        let mut raw_line = Vec::new();

        #[cfg(unix)]
        {
            if only_need_to_count_bytes {
                match count_bytes_fast(handle.file_descriptor) {
                    Some(bytes) => { byte_count = bytes; }
                    _ => {}
                }
            }
        }

        if byte_count == 0 {
            let mut buffered_reader = BufReader::new(handle.reader);
            // reading from a TTY seems to raise a condition on, rather than return Some(0) like a file.
            // hence the option wrapped in a result here
            while match buffered_reader.read_until(LF, &mut raw_line) {
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
                let current_char_count;
                match from_utf8(&raw_line[..]) {
                    Ok(line) => {
                        word_count += line.split_whitespace().count();
                        current_char_count = line.chars().count();
                    }
                    Err(..) => {
                        word_count += raw_line.split(|&x| is_word_seperator(x)).count();
                        current_char_count = raw_line.iter().filter(|c| c.is_ascii()).count()
                    }
                }
                char_count += current_char_count;

                if current_char_count > longest_line_length {
                    // we subtract one here because `line.len()` includes the LF
                    // matches GNU 'wc' behaviour
                    longest_line_length = current_char_count - 1;
                }

                raw_line.truncate(0);
            }
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
        println!("");
    }
}

fn open(path: &str) -> StdResult<InputHandle, i32> {
    if "-" == path {
        let stdin = stdin();
        return Ok(InputHandle {
            #[cfg(unix)]
            file_descriptor: stdin.as_raw_fd(),
            reader: Box::new(stdin) as Box<Read>,
        });
    }

    let fpath = Path::new(path);
    if fpath.is_dir() {
        show_info!("{}: is a directory", path);
    }
    match File::open(&fpath) {
        Ok(file) => Ok(InputHandle {
            #[cfg(unix)]
            file_descriptor: file.as_raw_fd(),
            reader: Box::new(file) as Box<Read>,
        }),
        Err(e) => {
            show_error!("wc: {}: {}", path, e);
            Err(1)
        }
    }
}
