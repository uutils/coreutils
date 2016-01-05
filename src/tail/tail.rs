#![crate_name = "uu_tail"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdin, stdout, Write};
use std::path::Path;
use std::str::from_utf8;
use std::thread::sleep;
use std::time::Duration;

static NAME: &'static str = "tail";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

enum FilterMode {
    Bytes(usize),
    Lines(usize),
}

struct Settings {
    mode: FilterMode,
    sleep_msec: u32,
    beginning: bool,
    follow: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: FilterMode::Lines(10),
            sleep_msec: 1000,
            beginning: false,
            follow: false,
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut settings: Settings = Default::default();

    // handle obsolete -number syntax
    let options = match obsolete(&args[1..]) {
        (args, Some(n)) => { settings.mode = FilterMode::Lines(n); args },
        (args, None) => args
    };

    let args = options;

    let mut opts = getopts::Options::new();

    opts.optopt("c", "bytes", "Number of bytes to print", "k");
    opts.optopt("n", "lines", "Number of lines to print", "k");
    opts.optflag("f", "follow", "Print the file as it grows");
    opts.optopt("s", "sleep-interval", "Number or seconds to sleep between polling the file when running with -f", "n");
    opts.optflag("h", "help", "help");
    opts.optflag("V", "version", "version");

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

    match given_options.opt_str("n") {
        Some(n) => {
            let mut slice: &str = n.as_ref();
            if slice.chars().next().unwrap_or('_') == '+' {
                settings.beginning = true;
                slice = &slice[1..];
            }
            match parse_size(slice) {
                Some(m) => settings.mode = FilterMode::Lines(m),
                None => {
                    show_error!("invalid number of lines ({})", slice);
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
                    Some(m) => settings.mode = FilterMode::Bytes(m),
                    None => {
                        show_error!("invalid number of bytes ({})", slice);
                        return 1;
                    }
                }
            }
            None => { }
        }
    };

    let files = given_options.free;

    if files.is_empty() {
        let mut buffer = BufReader::new(stdin());
        tail(&mut buffer, &settings);
    } else {
        let mut multiple = false;
        let mut firstime = true;

        if files.len() > 1 {
            multiple = true;
        }

        for file in &files {
            if multiple {
                if !firstime { println!(""); }
                println!("==> {} <==", file);
            }
            firstime = false;

            let path = Path::new(file);
            let reader = File::open(&path).unwrap();
            let mut buffer = BufReader::new(reader);
            tail(&mut buffer, &settings);
        }
    }

    0
}

fn parse_size(mut size_slice: &str) -> Option<usize> {
    let mut base =
        if size_slice.chars().last().unwrap_or('_') == 'B' {
            size_slice = &size_slice[..size_slice.len() - 1];
            1000usize
        } else {
            1024usize
        };
    let exponent = 
        if size_slice.len() > 0 {
            let mut has_suffix = true;
            let exp = match size_slice.chars().last().unwrap_or('_') {
                'K' => 1usize,
                'M' => 2usize,
                'G' => 3usize,
                'T' => 4usize,
                'P' => 5usize,
                'E' => 6usize,
                'Z' => 7usize,
                'Y' => 8usize,
                'b' => {
                    base = 512usize;
                    1usize
                }
                _ => {
                    has_suffix = false;
                    0usize
                }
            };
            if has_suffix {
                size_slice = &size_slice[..size_slice.len() - 1];
            }
            exp
        } else {
            0usize
        };

    let mut multiplier = 1usize;
    for _ in 0usize .. exponent {
        multiplier *= base;
    }
    if base == 1000usize && exponent == 0usize {
        // sole B is not a valid suffix
        None
    } else {
        let value: Option<usize> = size_slice.parse().ok();
        match value {
            Some(v) => Some(multiplier * v),
            _ => None
        }
    }
}

// It searches for an option in the form of -123123
//
// In case is found, the options vector will get rid of that object so that
// getopts works correctly.
fn obsolete(options: &[String]) -> (Vec<String>, Option<usize>) {
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
                    let number: Option<usize> = from_utf8(&current[1..len]).unwrap().parse().ok();
                    return (options, Some(number.unwrap()));
                }
            }
        }

        a += 1;
    };

    (options, None)
}

fn tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) {
    // Read through each line/char and store them in a ringbuffer that always
    // contains count lines/chars. When reaching the end of file, output the
    // data in the ringbuf.
    match settings.mode {
        FilterMode::Lines(mut count) => {
            let mut ringbuf: VecDeque<String> = VecDeque::new();
            let mut skip = if settings.beginning {
                let temp = count;
                count = ::std::usize::MAX;
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
                            if count <= ringbuf.len() {
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
                count = ::std::usize::MAX;
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
                            if count <= ringbuf.len() {
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

    // if we follow the file, sleep a bit and print the rest if the file has grown.
    while settings.follow {
        sleep(Duration::new(0, settings.sleep_msec*1000));
        loop {
            let mut datum = String::new();
            match reader.read_line(&mut datum) {
                Ok(0) => break,
                Ok(_) => print!("{}", datum),
                Err(err) => panic!(err)
            }
        }
    }
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
