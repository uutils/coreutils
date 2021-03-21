//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alan Andrade <alan.andradec@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//  *
//  * Synced with: https://raw.github.com/avsm/src/master/usr.bin/head/head.c

#[macro_use]
extern crate uucore;

use std::collections::VecDeque;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::Path;
use std::str::from_utf8;

static SYNTAX: &str = "";
static SUMMARY: &str = "";
static LONG_HELP: &str = "";

enum FilterMode {
    Bytes(usize),
    Lines(usize),
    NLines(usize),
}

struct Settings {
    mode: FilterMode,
    verbose: bool,
    zero_terminated: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: FilterMode::Lines(10),
            verbose: false,
            zero_terminated: false,
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let mut settings: Settings = Default::default();

    // handle obsolete -number syntax
    let new_args = match obsolete(&args[0..]) {
        (args, Some(n)) => {
            settings.mode = FilterMode::Lines(n);
            args
        }
        (args, None) => args,
    };

    let matches = app!(SYNTAX, SUMMARY, LONG_HELP)
        .optopt(
            "c",
            "bytes",
            "Print the first K bytes.  With the leading '-', print all but the last K bytes",
            "[-]K",
        )
        .optopt(
            "n",
            "lines",
            "Print the first K lines.  With the leading '-', print all but the last K lines",
            "[-]K",
        )
        .optflag("q", "quiet", "never print headers giving file names")
        .optflag("v", "verbose", "always print headers giving file names")
        .optflag("z", "zero-terminated", "line delimiter is NUL, not newline")
        .optflag("h", "help", "display this help and exit")
        .optflag("V", "version", "output version information and exit")
        .parse(new_args);

    let use_bytes = matches.opt_present("c");
    // TODO: suffixes (e.g. b, kB, etc.)
    match matches.opt_str("n") {
        Some(n) => {
            if use_bytes {
                show_error!("cannot specify both --bytes and --lines.");
                return 1;
            }

            match n.parse::<isize>() {
                Ok(m) => {
                    settings.mode = if m < 0 {
                        let m: usize = m.abs() as usize;
                        FilterMode::NLines(m)
                    } else {
                        let m: usize = m.abs() as usize;
                        FilterMode::Lines(m)
                    }
                }
                Err(e) => {
                    show_error!("invalid line count '{}': {}", n, e);
                    return 1;
                }
            }
        }
        None => {
            if let Some(count) = matches.opt_str("c") {
                match count.parse::<usize>() {
                    Ok(m) => settings.mode = FilterMode::Bytes(m),
                    Err(e) => {
                        show_error!("invalid byte count '{}': {}", count, e);
                        return 1;
                    }
                }
            }
        }
    };

    let quiet = matches.opt_present("q");
    let verbose = matches.opt_present("v");
    settings.zero_terminated = matches.opt_present("z");
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
        let mut first_time = true;

        for file in &files {
            if settings.verbose {
                if !first_time {
                    println!();
                }
                println!("==> {} <==", file);
            }
            first_time = false;

            let path = Path::new(file);
            if path.is_dir() || !path.metadata().is_ok() {
                eprintln!(
                    "cannot open '{}' for reading: No such file or directory",
                    &path.to_str().unwrap()
                );
                continue;
            }
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
    let mut a = 1;
    let b = options.len();

    while a < b {
        let previous = options[a - 1].clone();
        let current = options[a].clone();
        let current = current.as_bytes();

        if previous != "-n" && current.len() > 1 && current[0] == b'-' {
            let len = current.len();
            for pos in 1..len {
                // Ensure that the argument is only made out of digits
                if !(current[pos] as char).is_numeric() {
                    break;
                }

                // If this is the last number
                if pos == len - 1 {
                    options.remove(a);
                    let number: Option<usize> =
                        from_utf8(&current[1..len]).unwrap().parse::<usize>().ok();
                    return (options, Some(number.unwrap()));
                }
            }
        }

        a += 1;
    }

    (options, None)
}

// TODO: handle errors on read
fn head<T: Read>(reader: &mut BufReader<T>, settings: &Settings) -> bool {
    match settings.mode {
        FilterMode::Bytes(count) => {
            for byte in reader.bytes().take(count) {
                print!("{}", byte.unwrap() as char);
            }
        }
        FilterMode::Lines(count) => {
            if settings.zero_terminated {
                for line in reader.split(0).take(count) {
                    print!("{}\0", String::from_utf8(line.unwrap()).unwrap())
                }
            } else {
                for line in reader.lines().take(count) {
                    println!("{}", line.unwrap());
                }
            }
        }
        FilterMode::NLines(count) => {
            let mut vector: VecDeque<String> = VecDeque::new();

            for line in reader.lines() {
                vector.push_back(line.unwrap());
                if vector.len() <= count {
                    continue;
                }
                println!("{}", vector.pop_front().unwrap());
            }
        }
    }
    true
}
