#![crate_name = "uu_head"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 * Synced with: https://raw.github.com/avsm/src/master/usr.bin/head/head.c
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::io::{BufRead, BufReader, Read, stdin, Write};
use std::fs::File;
use std::path::Path;
use std::str::from_utf8;

static NAME: &'static str = "head";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

enum FilterMode {
    Bytes(usize),
    Lines(usize),
}

struct Settings {
    mode: FilterMode,
    verbose: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: FilterMode::Lines(10),
            verbose: false,
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

    opts.optopt("c", "bytes", "Print the first K bytes.  With the leading '-', print all but the last K bytes", "[-]K");
    opts.optopt("n", "lines", "Print the first K lines.  With the leading '-', print all but the last K lines", "[-]K");
    opts.optflag("q", "quiet", "never print headers giving file names");
    opts.optflag("v", "verbose", "always print headers giving file names");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args) {
        Ok (m) => { m }
        Err(_) => {
            println!("{}", opts.usage(""));
            return 1;
        }
    };

    if matches.opt_present("h") {
        println!("{}", opts.usage(""));
        return 0;
    }
    if matches.opt_present("V") { version(); return 0 }

    let use_bytes = matches.opt_present("c");

    // TODO: suffixes (e.g. b, kB, etc.)
    match matches.opt_str("n") {
        Some(n) => {
            if use_bytes {
                show_error!("cannot specify both --bytes and --lines.");
                return 1;
            }
            match n.parse::<usize>() {
                Ok(m) => { settings.mode = FilterMode::Lines(m) }
                Err(e) => {
                    show_error!("invalid line count '{}': {}", n, e);
                    return 1;
                }
            }
        }
        None => match matches.opt_str("c") {
            Some(count) => match count.parse::<usize>() {
                Ok(m) => settings.mode = FilterMode::Bytes(m),
                Err(e)=> {
                    show_error!("invalid byte count '{}': {}", count, e);
                    return 1;
                }
            },
            None => {}
        }
    };

    let quiet = matches.opt_present("q");
    let verbose = matches.opt_present("v");
    let files = matches.free;

    // GNU implementation allows multiple declarations of "-q" and "-v" with the
    // last flag winning. This can't be simulated with the getopts cargo unless
    // we manually parse the arguments. Given the declaration of both flags,
    // verbose mode always wins. This is a potential future improvement.
    if files.len() > 1 && !quiet && !verbose {
        settings.verbose = true;
    }
    if quiet {
        settings.verbose = false;
    }
    if verbose {
        settings.verbose = true;
    }

    if files.is_empty() {
        let mut buffer = BufReader::new(stdin());
        head(&mut buffer, &settings);
    } else {
        let mut firstime = true;

        for file in &files {
            if settings.verbose {
                if !firstime { pipe_println!(""); }
                pipe_println!("==> {} <==", file);
            }
            firstime = false;

            let path = Path::new(file);
            let reader = File::open(&path).unwrap();
            let mut buffer = BufReader::new(reader);
            if !head(&mut buffer, &settings) {
                break;
            }
        }
    }

    0
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
                    let number: Option<usize> = from_utf8(&current[1..len]).unwrap().parse::<usize>().ok();
                    return (options, Some(number.unwrap()));
                }
            }
        }

        a += 1;
    };

    (options, None)
}

// TODO: handle errors on read
fn head<T: Read>(reader: &mut BufReader<T>, settings: &Settings) -> bool {
    match settings.mode {
        FilterMode::Bytes(count) => {
            for byte in reader.bytes().take(count) {
                if !pipe_print!("{}", byte.unwrap() as char) {
                    return false;
                }
            }
        },
        FilterMode::Lines(count) => {
            for line in reader.lines().take(count) {
                if !pipe_println!("{}", line.unwrap()) {
                    return false;
                }
            }
        }
    }
    true
}

fn version() {
    println!("{} {}", NAME, VERSION);
}
