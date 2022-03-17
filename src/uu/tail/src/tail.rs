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

use clap::{Arg, Command};
use std::collections::VecDeque;
use std::convert::TryInto;
use std::ffi::OsString;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{stdin, stdout, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::format_usage;
use uucore::lines::lines;
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
const USAGE: &str = "{} [FLAG]... [FILE]...";

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
    Bytes(u64),
    Lines(u64, u8), // (number of lines, delimiter)
}

impl Default for FilterMode {
    fn default() -> Self {
        Self::Lines(10, b'\n')
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

        let mut settings: Self = Self {
            sleep_msec: 1000,
            follow: matches.is_present(options::FOLLOW),
            ..Default::default()
        };

        if settings.follow {
            if let Some(n) = matches.value_of(options::SLEEP_INT) {
                let parsed: Option<u32> = n.parse().ok();
                if let Some(m) = parsed {
                    settings.sleep_msec = m * 1000;
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
#[uucore::main]
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
            unbounded_tail(&mut reader, settings)?;

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
            let mut file = File::open(&path)
                .map_err_context(|| format!("cannot open {} for reading", filename.quote()))?;
            let md = file.metadata().unwrap();
            if is_seekable(&mut file) && get_block_size(&md) > 0 {
                bounded_tail(&mut file, &settings.mode, settings.beginning);
                if settings.follow {
                    let reader = BufReader::new(file);
                    readers.push((Box::new(reader), filename));
                }
            } else {
                let mut reader = BufReader::new(file);
                unbounded_tail(&mut reader, settings)?;
                if settings.follow {
                    readers.push((Box::new(reader), filename));
                }
            }
        }
    }

    if settings.follow {
        follow(&mut readers[..], settings)?;
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

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .short('c')
                .long(options::BYTES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of bytes to print"),
        )
        .arg(
            Arg::new(options::FOLLOW)
                .short('f')
                .long(options::FOLLOW)
                .help("Print the file as it grows"),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long(options::LINES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of lines to print"),
        )
        .arg(
            Arg::new(options::PID)
                .long(options::PID)
                .takes_value(true)
                .help("with -f, terminate after process ID, PID dies"),
        )
        .arg(
            Arg::new(options::verbosity::QUIET)
                .short('q')
                .long(options::verbosity::QUIET)
                .visible_alias("silent")
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("never output headers giving file names"),
        )
        .arg(
            Arg::new(options::SLEEP_INT)
                .short('s')
                .takes_value(true)
                .long(options::SLEEP_INT)
                .help("Number or seconds to sleep between polling the file when running with -f"),
        )
        .arg(
            Arg::new(options::verbosity::VERBOSE)
                .short('v')
                .long(options::verbosity::VERBOSE)
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("always output headers giving file names"),
        )
        .arg(
            Arg::new(options::ZERO_TERM)
                .short('z')
                .long(options::ZERO_TERM)
                .help("Line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .multiple_occurrences(true)
                .takes_value(true)
                .min_values(1),
        )
}

/// Continually check for new data in the given readers, writing any to stdout.
fn follow<T: BufRead>(readers: &mut [(T, &String)], settings: &Settings) -> UResult<()> {
    if readers.is_empty() || !settings.follow {
        return Ok(());
    }

    let mut last = readers.len() - 1;
    let mut read_some = false;
    let mut process = platform::ProcessChecker::new(settings.pid);
    let mut stdout = stdout();

    loop {
        sleep(Duration::new(0, settings.sleep_msec * 1000));

        let pid_is_dead = !read_some && settings.pid != 0 && process.is_dead();
        read_some = false;

        for (i, (reader, filename)) in readers.iter_mut().enumerate() {
            // Print all new content since the last pass
            loop {
                let mut datum = vec![];
                match reader.read_until(b'\n', &mut datum) {
                    Ok(0) => break,
                    Ok(_) => {
                        read_some = true;
                        if i != last {
                            println!("\n==> {} <==", filename);
                            last = i;
                        }
                        stdout
                            .write_all(&datum)
                            .map_err_context(|| String::from("write error"))?;
                    }
                    Err(err) => return Err(USimpleError::new(1, err.to_string())),
                }
            }
        }

        if pid_is_dead {
            break;
        }
    }
    Ok(())
}

/// Find the index after the given number of instances of a given byte.
///
/// This function reads through a given reader until `num_delimiters`
/// instances of `delimiter` have been seen, returning the index of
/// the byte immediately following that delimiter. If there are fewer
/// than `num_delimiters` instances of `delimiter`, this returns the
/// total number of bytes read from the `reader` until EOF.
///
/// # Errors
///
/// This function returns an error if there is an error during reading
/// from `reader`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\nb\nc\nd\ne\n");
/// let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
/// assert_eq!(i, 4);
/// ```
///
/// If `num_delimiters` is zero, then this function always returns
/// zero:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\n");
/// let i = forwards_thru_file(&mut reader, 0, b'\n').unwrap();
/// assert_eq!(i, 0);
/// ```
///
/// If there are fewer than `num_delimiters` instances of `delimiter`
/// in the reader, then this function returns the total number of
/// bytes read:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\n");
/// let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
/// assert_eq!(i, 2);
/// ```
fn forwards_thru_file<R>(
    reader: &mut R,
    num_delimiters: u64,
    delimiter: u8,
) -> std::io::Result<usize>
where
    R: Read,
{
    let mut reader = BufReader::new(reader);

    let mut buf = vec![];
    let mut total = 0;
    for _ in 0..num_delimiters {
        match reader.read_until(delimiter, &mut buf) {
            Ok(0) => {
                return Ok(total);
            }
            Ok(n) => {
                total += n;
                buf.clear();
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(total)
}

/// Iterate over bytes in the file, in reverse, until we find the
/// `num_delimiters` instance of `delimiter`. The `file` is left seek'd to the
/// position just after that delimiter.
fn backwards_thru_file(file: &mut File, num_delimiters: u64, delimiter: u8) {
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
fn bounded_tail(file: &mut File, mode: &FilterMode, beginning: bool) {
    // Find the position in the file to start printing from.
    match (mode, beginning) {
        (FilterMode::Lines(count, delimiter), false) => {
            backwards_thru_file(file, *count, *delimiter);
        }
        (FilterMode::Lines(count, delimiter), true) => {
            let i = forwards_thru_file(file, (*count).max(1) - 1, *delimiter).unwrap();
            file.seek(SeekFrom::Start(i as u64)).unwrap();
        }
        (FilterMode::Bytes(count), false) => {
            file.seek(SeekFrom::End(-(*count as i64))).unwrap();
        }
        (FilterMode::Bytes(count), true) => {
            // GNU `tail` seems to index bytes and lines starting at 1, not
            // at 0. It seems to treat `+0` and `+1` as the same thing.
            file.seek(SeekFrom::Start(((*count).max(1) - 1) as u64))
                .unwrap();
        }
    }

    // Print the target section of the file.
    let stdout = stdout();
    let mut stdout = stdout.lock();
    std::io::copy(file, &mut stdout).unwrap();
}

/// An alternative to [`Iterator::skip`] with u64 instead of usize. This is
/// necessary because the usize limit doesn't make sense when iterating over
/// something that's not in memory. For example, a very large file. This allows
/// us to skip data larger than 4 GiB even on 32-bit platforms.
fn skip_u64(iter: &mut impl Iterator, num: u64) {
    for _ in 0..num {
        if iter.next().is_none() {
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
    mut iter: impl Iterator<Item = Result<T, E>>,
    count: u64,
    beginning: bool,
) -> UResult<VecDeque<T>>
where
    E: fmt::Debug,
{
    if beginning {
        // GNU `tail` seems to index bytes and lines starting at 1, not
        // at 0. It seems to treat `+0` and `+1` as the same thing.
        let i = count.max(1) - 1;
        skip_u64(&mut iter, i);
        Ok(iter.map(|r| r.unwrap()).collect())
    } else {
        let count: usize = count
            .try_into()
            .map_err(|_| USimpleError::new(1, "Insufficient addressable memory"))?;
        Ok(RingBuffer::from_iter(iter.map(|r| r.unwrap()), count).data)
    }
}

fn unbounded_tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) -> UResult<()> {
    // Read through each line/char and store them in a ringbuffer that always
    // contains count lines/chars. When reaching the end of file, output the
    // data in the ringbuf.
    match settings.mode {
        FilterMode::Lines(count, sep) => {
            let mut stdout = stdout();
            for line in unbounded_tail_collect(lines(reader, sep), count, settings.beginning)? {
                stdout
                    .write_all(&line)
                    .map_err_context(|| String::from("IO error"))?;
            }
        }
        FilterMode::Bytes(count) => {
            for byte in unbounded_tail_collect(reader.bytes(), count, settings.beginning)? {
                if let Err(err) = stdout().write(&[byte]) {
                    return Err(USimpleError::new(1, err.to_string()));
                }
            }
        }
    }
    Ok(())
}

fn is_seekable<T: Seek>(file: &mut T) -> bool {
    file.seek(SeekFrom::Current(0)).is_ok()
        && file.seek(SeekFrom::End(0)).is_ok()
        && file.seek(SeekFrom::Start(0)).is_ok()
}

fn parse_num(src: &str) -> Result<(u64, bool), ParseSizeError> {
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

#[cfg(test)]
mod tests {

    use crate::forwards_thru_file;
    use std::io::Cursor;

    #[test]
    fn test_forwards_thru_file_zero() {
        let mut reader = Cursor::new("a\n");
        let i = forwards_thru_file(&mut reader, 0, b'\n').unwrap();
        assert_eq!(i, 0);
    }

    #[test]
    fn test_forwards_thru_file_basic() {
        //                   01 23 45 67 89
        let mut reader = Cursor::new("a\nb\nc\nd\ne\n");
        let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
        assert_eq!(i, 4);
    }

    #[test]
    fn test_forwards_thru_file_past_end() {
        let mut reader = Cursor::new("x\n");
        let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
        assert_eq!(i, 2);
    }
}
