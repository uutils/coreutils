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
#[macro_use]
extern crate quick_error;
#[cfg(unix)]
extern crate libc;
#[cfg(unix)]
extern crate nix;

use getopts::{Matches, Options};
use std::cmp::max;
use std::convert::From;
use std::fs::{File, OpenOptions};
use std::io::{self, stdin, BufRead, BufReader, ErrorKind, Read, Stdin};
use std::ops::{Add, AddAssign};
use std::path::Path;
use std::result::Result;
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

quick_error! {
    #[derive(Debug)]
    enum WcError {
        /// Wrapper for io::Error with no context
        InputOutput(err: io::Error) {
            display("cat: {0}", err)
            from()
            cause(err)
        }

        /// At least one error was encountered in reading or writing
        EncounteredErrors(count: usize) {
            display("cat: encountered {0} errors", count)
        }

        /// Denotes an error caused by trying to `wc` a directory
        IsDirectory(path: String) {
            display("wc: {}: Is a directory", path)
        }
    }
}

type WcResult<T> = Result<T, WcError>;

#[cfg(unix)]
trait WordCountable: Read + AsRawFd {}
#[cfg(not(unix))]
trait WordCountable: Read {}
impl WordCountable for Stdin {}
impl WordCountable for File {}

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

#[derive(Debug, Default, Copy, Clone)]
struct WordCount {
    bytes: usize,
    chars: usize,
    lines: usize,
    words: usize,
    max_line_length: usize,
}

impl Add for WordCount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            bytes: self.bytes + other.bytes,
            chars: self.chars + other.chars,
            lines: self.lines + other.lines,
            words: self.words + other.words,
            max_line_length: max(self.max_line_length, other.max_line_length),
        }
    }
}

impl AddAssign for WordCount {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other
    }
}

impl WordCount {
    fn with_title(self, title: String) -> TitledWordCount {
        return TitledWordCount {
            title: title,
            count: self,
        };
    }
}

/// This struct supplements the actual word count with a title that is displayed
/// to the user at the end of the program.
/// The reason we don't simply include title in the `WordCount` struct is that
/// it would result in unneccesary copying of `String`.
#[derive(Debug, Default, Clone)]
struct TitledWordCount {
    title: String,
    count: WordCount,
}

static NAME: &str = "wc";
static VERSION: &str = env!("CARGO_PKG_VERSION");

/// How large the buffer used for io operations is
const BUF_SIZE: usize = 16384;

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

    if wc(matches.free, &settings).is_ok() {
        0
    } else {
        1
    }
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

/// This is a Linux-specific function to count the number of bytes using the
/// `splice` system call, which is faster than using `read`.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_bytes_using_splice(fd: RawFd) -> nix::Result<usize> {
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

/// In the special case where we only need to count the number of bytes. There
/// are several optimizations we can do:
///   1. On Unix,  we can simply `stat` the file if it is regular.
///   2. On Linux -- if the above did not work -- we can use splice to count
///      the number of bytes if the file is a FIFO.
///   3. Otherwise, we just read normally, but without the overhead of counting
///      other things such as lines and words.
#[inline]
fn count_bytes_fast<T: WordCountable>(handle: &mut T) -> WcResult<usize> {
    #[cfg(unix)]
    {
        let fd = handle.as_raw_fd();
        match fstat(fd) {
            Ok(stat) => {
                // If the file is regular, then the `st_size` should hold
                // the file's size in bytes.
                if (stat.st_mode & S_IFREG) != 0 {
                    return Ok(stat.st_size as usize);
                }
                #[cfg(any(target_os = "linux", target_os = "android"))]
                {
                    // Else, if we're on Linux and our file is a FIFO pipe
                    // (or stdin), we use splice to count the number of bytes.
                    if (stat.st_mode & S_IFIFO) != 0 {
                        if let Ok(n) = count_bytes_using_splice(fd) {
                            return Ok(n);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Fall back on `read`, but without the overhead of counting words and lines.
    let mut buf = [0 as u8; BUF_SIZE];
    let mut byte_count = 0;
    loop {
        match handle.read(&mut buf) {
            Ok(0) => return Ok(byte_count),
            Ok(n) => {
                byte_count += n;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(WcError::from(e)),
        }
    }
}

fn word_count_from_reader<T: WordCountable>(
    reader: &mut T,
    only_count_bytes: bool,
) -> WcResult<WordCount> {
    if only_count_bytes {
        return Ok(WordCount {
            bytes: count_bytes_fast(reader)?,
            ..WordCount::default()
        });
    }

    let mut line_count: usize = 0;
    let mut word_count: usize = 0;
    let mut byte_count: usize = 0;
    let mut char_count: usize = 0;
    let mut longest_line_length: usize = 0;
    let mut raw_line = Vec::new();

    let mut buffered_reader = BufReader::new(reader);
    // reading from a TTY seems to raise a condition on, rather than return Some(0) like a file.
    // hence the option wrapped in a result here
    while match buffered_reader.read_until(LF, &mut raw_line) {
        Ok(n) if n > 0 => true,
        Err(_) if !raw_line.is_empty() => !raw_line.is_empty(),
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

    Ok(WordCount {
        bytes: byte_count,
        chars: char_count,
        lines: line_count,
        words: word_count,
        max_line_length: longest_line_length,
    })
}

fn word_count_from_path(path: &String, only_count_bytes: bool) -> WcResult<WordCount> {
    if path == "-" {
        let mut reader = stdin();
        return Ok(word_count_from_reader(&mut reader, only_count_bytes)?);
    } else {
        let path_obj = Path::new(path);
        if path_obj.is_dir() {
            return Err(WcError::IsDirectory(path.to_owned()));
        } else {
            let mut reader = File::open(path)?;
            return Ok(word_count_from_reader(&mut reader, only_count_bytes)?);
        }
    }
}

fn wc(files: Vec<String>, settings: &Settings) -> WcResult<()> {
    let mut total_word_count = WordCount::default();
    let mut results = vec![];
    let mut max_width: usize = 0;
    let mut error_count = 0;

    let num_files = files.len();

    let only_need_to_count_bytes = settings.show_bytes
        && (!(settings.show_chars
            || settings.show_lines
            || settings.show_max_line_length
            || settings.show_words));

    for path in files {
        let word_count =
            word_count_from_path(&path, only_need_to_count_bytes).unwrap_or_else(|err| {
                eprintln!("{}", err);
                error_count += 1;
                WordCount::default()
            });
        max_width = max(max_width, word_count.bytes.to_string().len() + 1);
        total_word_count += word_count;
        results.push(word_count.with_title(path));
    }

    for result in &results {
        print_stats(settings, &result, max_width);
    }

    if num_files > 1 {
        let total_result = total_word_count.with_title("total".to_owned());
        print_stats(settings, &total_result, max_width);
    }

    match error_count {
        0 => Ok(()),
        _ => Err(WcError::EncounteredErrors(error_count)),
    }
}

fn print_stats(settings: &Settings, result: &TitledWordCount, max_width: usize) {
    if settings.show_lines {
        print!("{:1$}", result.count.lines, max_width);
    }
    if settings.show_words {
        print!("{:1$}", result.count.words, max_width);
    }
    if settings.show_bytes {
        print!("{:1$}", result.count.bytes, max_width);
    }
    if settings.show_chars {
        print!("{:1$}", result.count.chars, max_width);
    }
    if settings.show_max_line_length {
        print!("{:1$}", result.count.max_line_length, max_width);
    }

    if result.title == "-" {
        println!("");
    } else {
        println!(" {}", result.title);
    }
}
