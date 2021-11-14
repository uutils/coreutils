//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
//  * (c) Alexander Batischev <eual.jp@gmail.com>
//  * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

mod chunks;
mod parse;
mod platform;
use chunks::ReverseChunks;

use clap::{App, Arg};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{stdin, stdout, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::ringbuffer::RingBuffer;

#[cfg(unix)]
use crate::platform::stdin_is_pipe_or_fifo;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const ABOUT: &str = "\
                     Print the last 10 lines of each FILE to standard output.\n\
                     With more than one FILE, precede each with a header giving the file name.\n\
                     With no FILE, or when FILE is -, read standard input.\n\
                     \n\
                     Mandatory arguments to long flags are mandatory for short flags too.\
                     ";
const USAGE: &str = "tail [FLAG]... [FILE]...";

pub mod options {
    pub mod verbosity {
        pub static QUIET: &str = "quiet";
        pub static VERBOSE: &str = "verbose";
    }
    pub static BYTES: &str = "bytes";
    pub static FOLLOW: &str = "follow";
    pub static LINES: &str = "lines";
    pub static PID: &str = "pid";
    pub static SLEEP_INT: &str = "sleep-interval";
    pub static ZERO_TERM: &str = "zero-terminated";
    pub static ARG_FILES: &str = "files";
}

#[derive(Debug)]
enum FilterMode {
    Bytes(usize),
    Lines(usize, u8), // (number of lines, delimiter)
}

impl Default for FilterMode {
    fn default() -> Self {
        FilterMode::Lines(10, b'\n')
    }
}

#[derive(Debug, Default)]
struct Settings {
    quiet: bool,
    verbose: bool,
    mode: FilterMode,
    sleep_msec: u32,
    beginning: bool,
    follow: bool,
    pid: platform::Pid,
    files: Vec<String>,
}

impl Settings {
    pub fn get_from(args: impl uucore::Args) -> Result<Self, String> {
        let matches = uu_app().get_matches_from(arg_iterate(args)?);

        let mut settings: Settings = Settings {
            sleep_msec: 1000,
            follow: matches.is_present(options::FOLLOW),
            ..Default::default()
        };

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

        let mode_and_beginning = if let Some(arg) = matches.value_of(options::BYTES) {
            match parse_num(arg) {
                Ok((n, beginning)) => (FilterMode::Bytes(n), beginning),
                Err(e) => return Err(format!("invalid number of bytes: {}", e)),
            }
        } else if let Some(arg) = matches.value_of(options::LINES) {
            match parse_num(arg) {
                Ok((n, beginning)) => (FilterMode::Lines(n, b'\n'), beginning),
                Err(e) => return Err(format!("invalid number of lines: {}", e)),
            }
        } else {
            (FilterMode::Lines(10, b'\n'), false)
        };
        settings.mode = mode_and_beginning.0;
        settings.beginning = mode_and_beginning.1;

        if matches.is_present(options::ZERO_TERM) {
            if let FilterMode::Lines(count, _) = settings.mode {
                settings.mode = FilterMode::Lines(count, 0);
            }
        }

        settings.verbose = matches.is_present(options::verbosity::VERBOSE);
        settings.quiet = matches.is_present(options::verbosity::QUIET);

        settings.files = match matches.values_of(options::ARG_FILES) {
            Some(v) => v.map(|s| s.to_owned()).collect(),
            None => vec!["-".to_owned()],
        };

        Ok(settings)
    }
}

#[allow(clippy::cognitive_complexity)]
#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = match Settings::get_from(args) {
        Ok(o) => o,
        Err(s) => {
            return Err(USimpleError::new(1, s));
        }
    };
    uu_tail(&args)
}

fn uu_tail(settings: &Settings) -> UResult<()> {
    let multiple = settings.files.len() > 1;
    let mut first_header = true;
    let mut readers: Vec<(Box<dyn BufRead>, &String)> = Vec::new();

    #[cfg(unix)]
    let stdin_string = String::from("standard input");

    for filename in &settings.files {
        let use_stdin = filename.as_str() == "-";
        if (multiple || settings.verbose) && !settings.quiet {
            if !first_header {
                println!();
            }
            if use_stdin {
                println!("==> standard input <==");
            } else {
                println!("==> {} <==", filename);
            }
        }
        first_header = false;

        if use_stdin {
            let mut reader = BufReader::new(stdin());
            unbounded_tail(&mut reader, settings);

            // Don't follow stdin since there are no checks for pipes/FIFOs
            //
            // FIXME windows has GetFileType which can determine if the file is a pipe/FIFO
            // so this check can also be performed

            #[cfg(unix)]
            {
                /*
                POSIX specification regarding tail -f

                If the input file is a regular file or if the file operand specifies a FIFO, do not
                terminate after the last line of the input file has been copied, but read and copy
                further bytes from the input file when they become available. If no file operand is
                specified and standard input is a pipe or FIFO, the -f option shall be ignored. If
                the input file is not a FIFO, pipe, or regular file, it is unspecified whether or
                not the -f option shall be ignored.
                */

                if settings.follow && !stdin_is_pipe_or_fifo() {
                    readers.push((Box::new(reader), &stdin_string));
                }
            }
        } else {
            let path = Path::new(filename);
            if path.is_dir() {
                continue;
            }
            let mut file = File::open(&path).unwrap();
            let md = file.metadata().unwrap();
            if is_seekable(&mut file) && get_block_size(&md) > 0 {
                bounded_tail(&mut file, settings);
                if settings.follow {
                    let reader = BufReader::new(file);
                    readers.push((Box::new(reader), filename));
                }
            } else {
                let mut reader = BufReader::new(file);
                unbounded_tail(&mut reader, settings);
                if settings.follow {
                    readers.push((Box::new(reader), filename));
                }
            }
        }
    }

    if settings.follow {
        follow(&mut readers[..], settings);
    }

    Ok(())
}

fn arg_iterate<'a>(
    mut args: impl uucore::Args + 'a,
) -> Result<Box<dyn Iterator<Item = OsString> + 'a>, String> {
    // argv[0] is always present
    let first = args.next().unwrap();
    if let Some(second) = args.next() {
        if let Some(s) = second.to_str() {
            match parse::parse_obsolete(s) {
                Some(Ok(iter)) => Ok(Box::new(vec![first].into_iter().chain(iter).chain(args))),
                Some(Err(e)) => match e {
                    parse::ParseError::Syntax => Err(format!("bad argument format: {}", s.quote())),
                    parse::ParseError::Overflow => Err(format!(
                        "invalid argument: {} Value too large for defined datatype",
                        s.quote()
                    )),
                },
                None => Ok(Box::new(vec![first, second].into_iter().chain(args))),
            }
        } else {
            Err("bad argument encoding".to_owned())
        }
    } else {
        Ok(Box::new(vec![first].into_iter()))
    }
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .usage(USAGE)
        .arg(
            Arg::with_name(options::BYTES)
                .short("c")
                .long(options::BYTES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
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
                .overrides_with_all(&[options::BYTES, options::LINES])
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
                .visible_alias("silent")
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("never output headers giving file names"),
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
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("always output headers giving file names"),
        )
        .arg(
            Arg::with_name(options::ZERO_TERM)
                .short("z")
                .long(options::ZERO_TERM)
                .help("Line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::with_name(options::ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
}

fn follow<T: BufRead>(readers: &mut [(T, &String)], settings: &Settings) {
    assert!(settings.follow);
    if readers.is_empty() {
        return;
    }

    let mut last = readers.len() - 1;
    let mut read_some = false;
    let mut process = platform::ProcessChecker::new(settings.pid);

    loop {
        sleep(Duration::new(0, settings.sleep_msec * 1000));

        let pid_is_dead = !read_some && settings.pid != 0 && process.is_dead();
        read_some = false;

        for (i, (reader, filename)) in readers.iter_mut().enumerate() {
            // Print all new content since the last pass
            loop {
                let mut datum = String::new();
                match reader.read_line(&mut datum) {
                    Ok(0) => break,
                    Ok(_) => {
                        read_some = true;
                        if i != last {
                            println!("\n==> {} <==", filename);
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
    let stdout = stdout();
    let mut stdout = stdout.lock();
    std::io::copy(file, &mut stdout).unwrap();
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
    count: usize,
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
        && file.seek(SeekFrom::End(0)).is_ok()
        && file.seek(SeekFrom::Start(0)).is_ok()
}

#[inline]
fn print_byte<T: Write>(stdout: &mut T, ch: u8) {
    if let Err(err) = stdout.write(&[ch]) {
        crash!(1, "{}", err);
    }
}

fn parse_num(src: &str) -> Result<(usize, bool), ParseSizeError> {
    let mut size_string = src.trim();
    let mut starting_with = false;

    if let Some(c) = size_string.chars().next() {
        if c == '+' || c == '-' {
            // tail: '-' is not documented (8.32 man pages)
            size_string = &size_string[1..];
            if c == '+' {
                starting_with = true;
            }
        }
    } else {
        return Err(ParseSizeError::ParseFailure(src.to_string()));
    }

    parse_size(size_string).map(|n| (n, starting_with))
}

fn get_block_size(md: &Metadata) -> u64 {
    #[cfg(unix)]
    {
        md.blocks()
    }
    #[cfg(not(unix))]
    {
        md.len()
    }
}
