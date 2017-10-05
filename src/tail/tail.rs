#![crate_name = "uu_tail"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
 * (c) Alexander Batischev <eual.jp@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

mod platform;

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, stdin, stdout, Write};
use std::path::Path;
use std::str::from_utf8;
use std::thread::sleep;
use std::time::Duration;

static NAME: &'static str = "tail";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

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
            mode: FilterMode::Lines(10, '\n' as u8),
            sleep_msec: 1000,
            beginning: false,
            follow: false,
            pid: 0,
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut settings: Settings = Default::default();

    // handle obsolete -number syntax
    let options = match obsolete(&args[1..]) {
        (args, Some(n)) => { settings.mode = FilterMode::Lines(n, '\n' as u8); args },
        (args, None) => args
    };

    let args = options;

    let mut opts = getopts::Options::new();

    opts.optopt("c", "bytes", "Number of bytes to print", "k");
    opts.optopt("n", "lines", "Number of lines to print", "k");
    opts.optflag("f", "follow", "Print the file as it grows");
    opts.optopt("s", "sleep-interval", "Number or seconds to sleep between polling the file when running with -f", "n");
    opts.optopt("", "pid", "with -f, terminate after process ID, PID dies", "PID");
    opts.optflag("z", "zero-terminated", "Line delimiter is NUL, not newline");
    opts.optflag("h", "help", "help");
    opts.optflag("V", "version", "version");
    opts.optflag("v", "verbose", "always output headers giving file names");
    // TODO: --silent flag as alias to --quiet
    opts.optflag("q", "quiet", "never output headers giving file names");

    let given_options = match opts.parse(&args) {
        Ok (m) => { m }
        Err(_) => {
            println!("{}", opts.usage(""));
            return 1;
        }
    };

    if given_options.opt_present("h") {
        println!("{}", opts.usage(""));
        return 0;
    }
    if given_options.opt_present("V") { version(); return 0 }

    settings.follow = given_options.opt_present("f");
    if settings.follow {
        match given_options.opt_str("s") {
            Some(n) => {
                let parsed: Option<u32> = n.parse().ok();
                match parsed {
                    Some(m) => { settings.sleep_msec = m * 1000 }
                    None => {}
                }
            }
            None => {}
        };
    }

    if let Some(pid_str) = given_options.opt_str("pid") {
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

    match given_options.opt_str("n") {
        Some(n) => {
            let mut slice: &str = n.as_ref();
            if slice.chars().next().unwrap_or('_') == '+' {
                settings.beginning = true;
                slice = &slice[1..];
            }
            match parse_size(slice) {
                Ok(m) => settings.mode = FilterMode::Lines(m, '\n' as u8),
                Err(e) => {
                    show_error!("{}", e.description());
                    return 1;
                }
            }
        }
        None => match given_options.opt_str("c") {
            Some(n) => {
                let mut slice: &str = n.as_ref();
                if slice.chars().next().unwrap_or('_') == '+' {
                    settings.beginning = true;
                    slice = &slice[1..];
                }
                match parse_size(slice) {
                    Ok(m) => settings.mode = FilterMode::Bytes(m),
                    Err(e) => {
                        show_error!("{}", e.description());
                        return 1;
                    }
                }
            }
            None => { }
        }
    };

    if given_options.opt_present("z") {
        if let FilterMode::Lines(count, _) = settings.mode {
            settings.mode = FilterMode::Lines(count, 0);
        }
    }

    let verbose = given_options.opt_present("v");
    let quiet = given_options.opt_present("q");

    let files = given_options.free;

    if files.is_empty() {
        let mut buffer = BufReader::new(stdin());
        unbounded_tail(&mut buffer, &settings);
    } else {
        let multiple = files.len() > 1;
        let mut first_header = true;
        let mut readers = Vec::new();

        for filename in &files {
            if (multiple || verbose) && !quiet {
                if !first_header { println!(""); }
                println!("==> {} <==", filename);
            }
            first_header = false;

            let path = Path::new(filename);
            if path.is_dir() {
                continue;
            }
            let mut file = File::open(&path).unwrap();
            if is_seekable(&mut file) {
                bounded_tail(&file, &settings);
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
        write!(f, "{}", Error::description(self))
    }
}

impl ParseSizeErr {
    fn parse_failure(s: &str) -> ParseSizeErr {
        ParseSizeErr::ParseFailure(format!("invalid size: '{}'", s))
    }

    fn size_too_big(s: &str) -> ParseSizeErr {
        ParseSizeErr::SizeTooBig(
            format!("invalid size: '{}': Value too large to be stored in data type", s))
    }
}

pub type ParseSizeResult = Result<u64, ParseSizeErr>;

pub fn parse_size(mut size_slice: &str) -> Result<u64, ParseSizeErr> {
    let mut base =
        if size_slice.chars().last().unwrap_or('_') == 'B' {
            size_slice = &size_slice[..size_slice.len() - 1];
            1000u64
        } else {
            1024u64
        };

    let exponent =
        if size_slice.len() > 0 {
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
                },
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
    for _ in 0u64 .. exponent {
        multiplier *= base;
    }
    if base == 1000u64 && exponent == 0u64 {
        // sole B is not a valid suffix
        Err(ParseSizeErr::parse_failure(size_slice))
    } else {
        let value: Option<u64> = size_slice.parse().ok();
        value.map(|v| Ok(multiplier * v))
             .unwrap_or(Err(ParseSizeErr::parse_failure(size_slice)))
    }
}

// It searches for an option in the form of -123123
//
// In case is found, the options vector will get rid of that object so that
// getopts works correctly.
fn obsolete(options: &[String]) -> (Vec<String>, Option<u64>) {
    let mut options: Vec<String> = options.to_vec();
    let mut a = 0;
    let b = options.len();

    while a < b {
        let current = options[a].clone();
        let current = current.as_bytes();

        if current.len() > 1 && current[0] == '-' as u8 {
            let len = current.len();
            for pos in 1 .. len {
                // Ensure that the argument is only made out of digits
                if !(current[pos] as char).is_numeric() { break; }

                // If this is the last number
                if pos == len - 1 {
                    options.remove(a);
                    let number: Option<u64> = from_utf8(&current[1..len]).unwrap().parse().ok();
                    return (options, Some(number.unwrap()));
                }
            }
        }

        a += 1;
    };

    (options, None)
}

/// When reading files in reverse in `bounded_tail`, this is the size of each
/// block read at a time.
const BLOCK_SIZE: u64 = 1 << 16;

fn follow<T: Read>(readers: &mut [BufReader<T>], filenames: &[String], settings: &Settings) {
    assert!(settings.follow);
    let mut last = readers.len() - 1;
    let mut read_some = false;
    let mut process = platform::ProcessChecker::new(settings.pid);

    loop {
        sleep(Duration::new(0, settings.sleep_msec*1000));

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
                    },
                    Err(err) => panic!(err)
                }
            }
        }

        if pid_is_dead {
            break;
        }
    }
}

/// Iterate over bytes in the file, in reverse, until `should_stop` returns
/// true. The `file` is left seek'd to the position just after the byte that
/// `should_stop` returned true for.
fn backwards_thru_file<F>(mut file: &File, size: u64, buf: &mut Vec<u8>, delimiter: u8, should_stop: &mut F)
    where F: FnMut(u8) -> bool
{
    assert!(buf.len() >= BLOCK_SIZE as usize);

    let max_blocks_to_read = (size as f64 / BLOCK_SIZE as f64).ceil() as usize;

    for block_idx in 0..max_blocks_to_read {
        let block_size = if block_idx == max_blocks_to_read - 1 {
            size % BLOCK_SIZE
        } else {
            BLOCK_SIZE
        };

        // Seek backwards by the next block, read the full block into
        // `buf`, and then seek back to the start of the block again.
        let pos = file.seek(SeekFrom::Current(-(block_size as i64))).unwrap();
        file.read_exact(&mut buf[0..(block_size as usize)]).unwrap();
        let pos2 = file.seek(SeekFrom::Current(-(block_size as i64))).unwrap();
        assert_eq!(pos, pos2);

        // Iterate backwards through the bytes, calling `should_stop` on each
        // one.
        let slice = &buf[0..(block_size as usize)];
        for (i, ch) in slice.iter().enumerate().rev() {
            // Ignore one trailing newline.
            if block_idx == 0 && i as u64 == block_size - 1 && *ch == delimiter {
                continue;
            }

            if should_stop(*ch) {
                file.seek(SeekFrom::Current((i + 1) as i64)).unwrap();
                return;
            }
        }
    }
}

/// When tail'ing a file, we do not need to read the whole file from start to
/// finish just to find the last n lines or bytes. Instead, we can seek to the
/// end of the file, and then read the file "backwards" in blocks of size
/// `BLOCK_SIZE` until we find the location of the first line/byte. This ends up
/// being a nice performance win for very large files.
fn bounded_tail(mut file: &File, settings: &Settings) {
    let size = file.seek(SeekFrom::End(0)).unwrap();
    let mut buf = vec![0; BLOCK_SIZE as usize];

    // Find the position in the file to start printing from.
    match settings.mode {
        FilterMode::Lines(mut count, delimiter) => {
            backwards_thru_file(&mut file, size, &mut buf, delimiter, &mut |byte| {
                if byte == delimiter {
                    count -= 1;
                    count == 0
                } else {
                    false
                }
            });
        },
        FilterMode::Bytes(count) => {
            file.seek(SeekFrom::End(-(count as i64))).unwrap();
        },
    }

    // Print the target section of the file.
    loop {
        let bytes_read = file.read(&mut buf).unwrap();

        let mut stdout = stdout();
        for b in &buf[0..bytes_read] {
            print_byte(&mut stdout, b);
        }

        if bytes_read == 0 {
            break;
        }
    }
}

fn unbounded_tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) {
    // Read through each line/char and store them in a ringbuffer that always
    // contains count lines/chars. When reaching the end of file, output the
    // data in the ringbuf.
    match settings.mode {
        FilterMode::Lines(mut count, _delimiter) => {
            let mut ringbuf: VecDeque<String> = VecDeque::new();
            let mut skip = if settings.beginning {
                let temp = count;
                count = ::std::u64::MAX;
                temp - 1
            } else {
                0
            };
            loop {
                let mut datum = String::new();
                match reader.read_line(&mut datum) {
                    Ok(0) => break,
                    Ok(_) => {
                        if skip > 0 {
                            skip -= 1;
                        } else {
                            if count <= ringbuf.len() as u64 {
                                ringbuf.pop_front();
                            }
                            ringbuf.push_back(datum);
                        }
                    },
                    Err(err) => panic!(err)
                }
            }
            let mut stdout = stdout();
            for datum in &ringbuf {
                print_string(&mut stdout, datum);
            }
        },
        FilterMode::Bytes(mut count) => {
            let mut ringbuf: VecDeque<u8> = VecDeque::new();
            let mut skip = if settings.beginning {
                let temp = count;
                count = ::std::u64::MAX;
                temp - 1
            } else {
                0
            };
            loop {
                let mut datum = [0; 1];
                match reader.read(&mut datum) {
                    Ok(0) => break,
                    Ok(_) => {
                        if skip > 0 {
                            skip -= 1;
                        } else {
                            if count <= ringbuf.len() as u64 {
                                ringbuf.pop_front();
                            }
                            ringbuf.push_back(datum[0]);
                        }
                    },
                    Err(err) => panic!(err)
                }
            }
            let mut stdout = stdout();
            for datum in &ringbuf {
                print_byte(&mut stdout, datum);
            }
        }
    }
}

fn is_seekable<T: Seek>(file: &mut T) -> bool {
    file.seek(SeekFrom::Current(0)).is_ok()
}

#[inline]
fn print_byte<T: Write>(stdout: &mut T, ch: &u8) {
    if let Err(err) = stdout.write(&[*ch]) {
        crash!(1, "{}", err);
    }
}

#[inline]
fn print_string<T: Write>(_: &mut T, s: &str) {
    print!("{}", s);
}

fn version() {
    println!("{} {}", NAME, VERSION);
}
