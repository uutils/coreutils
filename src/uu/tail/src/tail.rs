//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
//  * (c) Alexander Batischev <eual.jp@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//  *

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

mod chunks;
mod platform;
mod ringbuffer;
use chunks::ReverseChunks;
use chunks::BLOCK_SIZE;
use ringbuffer::RingBuffer;

use clap::{App, Arg};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub mod options {
    pub mod verbosity {
        pub static QUIET: &str = "quiet";
        pub static SILENT: &str = "silent";
        pub static VERBOSE: &str = "verbose";
    }
    pub static BYTES: &str = "bytes";
    pub static FOLLOW: &str = "follow";
    pub static LINES: &str = "lines";
    pub static PID: &str = "pid";
    pub static SLEEP_INT: &str = "sleep-interval";
    pub static ZERO_TERM: &str = "zero-terminated";
}

static ARG_FILES: &str = "files";

enum FilterMode {
    Bytes(u64),
    Lines(u64, u8), // (number of lines, delimiter)
}

struct Settings {
    mode: FilterMode,
    sleep_msec: u32,
    beginning: bool,
    follow: bool,
    pid: platform::Pid,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: FilterMode::Lines(10, b'\n'),
            sleep_msec: 1000,
            beginning: false,
            follow: false,
            pid: 0,
        }
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {
    let mut settings: Settings = Default::default();

    let app = App::new(executable!())
        .version(crate_version!())
        .about("output the last part of files")
        .arg(
            Arg::with_name(options::BYTES)
                .short("c")
                .long(options::BYTES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .help("Number of bytes to print"),
        )
        .arg(
            Arg::with_name(options::FOLLOW)
                .short("f")
                .long(options::FOLLOW)
                .help("Print the file as it grows"),
        )
        .arg(
            Arg::with_name(options::LINES)
                .short("n")
                .long(options::LINES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .help("Number of lines to print"),
        )
        .arg(
            Arg::with_name(options::PID)
                .long(options::PID)
                .takes_value(true)
                .help("with -f, terminate after process ID, PID dies"),
        )
        .arg(
            Arg::with_name(options::verbosity::QUIET)
                .short("q")
                .long(options::verbosity::QUIET)
                .help("never output headers giving file names"),
        )
        .arg(
            Arg::with_name(options::verbosity::SILENT)
                .long(options::verbosity::SILENT)
                .help("synonym of --quiet"),
        )
        .arg(
            Arg::with_name(options::SLEEP_INT)
                .short("s")
                .takes_value(true)
                .long(options::SLEEP_INT)
                .help("Number or seconds to sleep between polling the file when running with -f"),
        )
        .arg(
            Arg::with_name(options::verbosity::VERBOSE)
                .short("v")
                .long(options::verbosity::VERBOSE)
                .help("always output headers giving file names"),
        )
        .arg(
            Arg::with_name(options::ZERO_TERM)
                .short("z")
                .long(options::ZERO_TERM)
                .help("Line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        );

    let matches = app.get_matches_from(args);

    settings.follow = matches.is_present(options::FOLLOW);
    if settings.follow {
        if let Some(n) = matches.value_of(options::SLEEP_INT) {
            let parsed: Option<u32> = n.parse().ok();
            if let Some(m) = parsed {
                settings.sleep_msec = m * 1000
            }
        }
    }

    if let Some(pid_str) = matches.value_of(options::PID) {
        if let Ok(pid) = pid_str.parse() {
            settings.pid = pid;
            if pid != 0 {
                if !settings.follow {
                    show_warning!("PID ignored; --pid=PID is useful only when following");
                }

                if !platform::supports_pid_checks(pid) {
                    show_warning!("--pid=PID is not supported on this system");
                    settings.pid = 0;
                }
            }
        }
    }

    match matches.value_of(options::LINES) {
        Some(n) => {
            let mut slice: &str = n;
            if slice.chars().next().unwrap_or('_') == '+' {
                settings.beginning = true;
                slice = &slice[1..];
            }
            match parse_size(slice) {
                Ok(m) => settings.mode = FilterMode::Lines(m, b'\n'),
                Err(e) => {
                    show_error!("{}", e.to_string());
                    return 1;
                }
            }
        }
        None => {
            if let Some(n) = matches.value_of(options::BYTES) {
                let mut slice: &str = n;
                if slice.chars().next().unwrap_or('_') == '+' {
                    settings.beginning = true;
                    slice = &slice[1..];
                }
                match parse_size(slice) {
                    Ok(m) => settings.mode = FilterMode::Bytes(m),
                    Err(e) => {
                        show_error!("{}", e.to_string());
                        return 1;
                    }
                }
            }
        }
    };

    if matches.is_present(options::ZERO_TERM) {
        if let FilterMode::Lines(count, _) = settings.mode {
            settings.mode = FilterMode::Lines(count, 0);
        }
    }

    let verbose = matches.is_present(options::verbosity::VERBOSE);
    let quiet = matches.is_present(options::verbosity::QUIET)
        || matches.is_present(options::verbosity::SILENT);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        let mut buffer = BufReader::new(stdin());
        unbounded_tail(&mut buffer, &settings);
    } else {
        let multiple = files.len() > 1;
        let mut first_header = true;
        let mut readers = Vec::new();

        for filename in &files {
            if (multiple || verbose) && !quiet {
                if !first_header {
                    println!();
                }
                println!("==> {} <==", filename);
            }
            first_header = false;

            let path = Path::new(filename);
            if path.is_dir() {
                continue;
            }
            let mut file = File::open(&path).unwrap();
            if is_seekable(&mut file) {
                bounded_tail(&mut file, &settings);
                if settings.follow {
                    let reader = BufReader::new(file);
                    readers.push(reader);
                }
            } else {
                let mut reader = BufReader::new(file);
                unbounded_tail(&mut reader, &settings);
                if settings.follow {
                    readers.push(reader);
                }
            }
        }

        if settings.follow {
            follow(&mut readers[..], &files[..], &settings);
        }
    }

    0
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseSizeErr {
    ParseFailure(String),
    SizeTooBig(String),
}

impl Error for ParseSizeErr {
    fn description(&self) -> &str {
        match *self {
            ParseSizeErr::ParseFailure(ref s) => &*s,
            ParseSizeErr::SizeTooBig(ref s) => &*s,
        }
    }
}

impl fmt::Display for ParseSizeErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s = match self {
            ParseSizeErr::ParseFailure(s) => s,
            ParseSizeErr::SizeTooBig(s) => s,
        };
        write!(f, "{}", s)
    }
}

impl ParseSizeErr {
    fn parse_failure(s: &str) -> ParseSizeErr {
        ParseSizeErr::ParseFailure(format!("invalid size: '{}'", s))
    }

    fn size_too_big(s: &str) -> ParseSizeErr {
        ParseSizeErr::SizeTooBig(format!(
            "invalid size: '{}': Value too large to be stored in data type",
            s
        ))
    }
}

pub type ParseSizeResult = Result<u64, ParseSizeErr>;

pub fn parse_size(mut size_slice: &str) -> Result<u64, ParseSizeErr> {
    let mut base = if size_slice.chars().last().unwrap_or('_') == 'B' {
        size_slice = &size_slice[..size_slice.len() - 1];
        1000u64
    } else {
        1024u64
    };

    let exponent = if !size_slice.is_empty() {
        let mut has_suffix = true;
        let exp = match size_slice.chars().last().unwrap_or('_') {
            'K' | 'k' => 1u64,
            'M' => 2u64,
            'G' => 3u64,
            'T' => 4u64,
            'P' => 5u64,
            'E' => 6u64,
            'Z' | 'Y' => {
                return Err(ParseSizeErr::size_too_big(size_slice));
            }
            'b' => {
                base = 512u64;
                1u64
            }
            _ => {
                has_suffix = false;
                0u64
            }
        };
        if has_suffix {
            size_slice = &size_slice[..size_slice.len() - 1];
        }
        exp
    } else {
        0u64
    };

    let mut multiplier = 1u64;
    for _ in 0u64..exponent {
        multiplier *= base;
    }
    if base == 1000u64 && exponent == 0u64 {
        // sole B is not a valid suffix
        Err(ParseSizeErr::parse_failure(size_slice))
    } else {
        let value: Option<i64> = size_slice.parse().ok();
        value
            .map(|v| Ok((multiplier as i64 * v.abs()) as u64))
            .unwrap_or_else(|| Err(ParseSizeErr::parse_failure(size_slice)))
    }
}

fn follow<T: Read>(readers: &mut [BufReader<T>], filenames: &[String], settings: &Settings) {
    assert!(settings.follow);
    let mut last = readers.len() - 1;
    let mut read_some = false;
    let mut process = platform::ProcessChecker::new(settings.pid);

    loop {
        sleep(Duration::new(0, settings.sleep_msec * 1000));

        let pid_is_dead = !read_some && settings.pid != 0 && process.is_dead();
        read_some = false;

        for (i, reader) in readers.iter_mut().enumerate() {
            // Print all new content since the last pass
            loop {
                let mut datum = String::new();
                match reader.read_line(&mut datum) {
                    Ok(0) => break,
                    Ok(_) => {
                        read_some = true;
                        if i != last {
                            println!("\n==> {} <==", filenames[i]);
                            last = i;
                        }
                        print!("{}", datum);
                    }
                    Err(err) => panic!("{}", err),
                }
            }
        }

        if pid_is_dead {
            break;
        }
    }
}

/// Iterate over bytes in the file, in reverse, until we find the
/// `num_delimiters` instance of `delimiter`. The `file` is left seek'd to the
/// position just after that delimiter.
fn backwards_thru_file(file: &mut File, num_delimiters: usize, delimiter: u8) {
    // This variable counts the number of delimiters found in the file
    // so far (reading from the end of the file toward the beginning).
    let mut counter = 0;

    for (block_idx, slice) in ReverseChunks::new(file).enumerate() {
        // Iterate over each byte in the slice in reverse order.
        let mut iter = slice.iter().enumerate().rev();

        // Ignore a trailing newline in the last block, if there is one.
        if block_idx == 0 {
            if let Some(c) = slice.last() {
                if *c == delimiter {
                    iter.next();
                }
            }
        }

        // For each byte, increment the count of the number of
        // delimiters found. If we have found more than the specified
        // number of delimiters, terminate the search and seek to the
        // appropriate location in the file.
        for (i, ch) in iter {
            if *ch == delimiter {
                counter += 1;
                if counter >= num_delimiters {
                    // After each iteration of the outer loop, the
                    // cursor in the file is at the *beginning* of the
                    // block, so seeking forward by `i + 1` bytes puts
                    // us right after the found delimiter.
                    file.seek(SeekFrom::Current((i + 1) as i64)).unwrap();
                    return;
                }
            }
        }
    }
}

/// When tail'ing a file, we do not need to read the whole file from start to
/// finish just to find the last n lines or bytes. Instead, we can seek to the
/// end of the file, and then read the file "backwards" in blocks of size
/// `BLOCK_SIZE` until we find the location of the first line/byte. This ends up
/// being a nice performance win for very large files.
fn bounded_tail(file: &mut File, settings: &Settings) {
    let mut buf = vec![0; BLOCK_SIZE as usize];

    // Find the position in the file to start printing from.
    match settings.mode {
        FilterMode::Lines(count, delimiter) => {
            backwards_thru_file(file, count as usize, delimiter);
        }
        FilterMode::Bytes(count) => {
            file.seek(SeekFrom::End(-(count as i64))).unwrap();
        }
    }

    // Print the target section of the file.
    loop {
        let bytes_read = file.read(&mut buf).unwrap();

        let mut stdout = stdout();
        for b in &buf[0..bytes_read] {
            print_byte(&mut stdout, *b);
        }

        if bytes_read == 0 {
            break;
        }
    }
}

/// Collect the last elements of an iterator into a `VecDeque`.
///
/// This function returns a [`VecDeque`] containing either the last
/// `count` elements of `iter`, an [`Iterator`] over [`Result`]
/// instances, or all but the first `count` elements of `iter`. If
/// `beginning` is `true`, then all but the first `count` elements are
/// returned.
///
/// # Panics
///
/// If any element of `iter` is an [`Err`], then this function panics.
fn unbounded_tail_collect<T, E>(
    iter: impl Iterator<Item = Result<T, E>>,
    count: u64,
    beginning: bool,
) -> VecDeque<T>
where
    E: fmt::Debug,
{
    if beginning {
        // GNU `tail` seems to index bytes and lines starting at 1, not
        // at 0. It seems to treat `+0` and `+1` as the same thing.
        let i = count.max(1) - 1;
        iter.skip(i as usize).map(|r| r.unwrap()).collect()
    } else {
        RingBuffer::from_iter(iter.map(|r| r.unwrap()), count as usize).data
    }
}

fn unbounded_tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) {
    // Read through each line/char and store them in a ringbuffer that always
    // contains count lines/chars. When reaching the end of file, output the
    // data in the ringbuf.
    match settings.mode {
        FilterMode::Lines(count, _) => {
            for line in unbounded_tail_collect(reader.lines(), count, settings.beginning) {
                println!("{}", line);
            }
        }
        FilterMode::Bytes(count) => {
            for byte in unbounded_tail_collect(reader.bytes(), count, settings.beginning) {
                let mut stdout = stdout();
                print_byte(&mut stdout, byte);
            }
        }
    }
}

fn is_seekable<T: Seek>(file: &mut T) -> bool {
    file.seek(SeekFrom::Current(0)).is_ok()
}

#[inline]
fn print_byte<T: Write>(stdout: &mut T, ch: u8) {
    if let Err(err) = stdout.write(&[ch]) {
        crash!(1, "{}", err);
    }
}
