//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) RFILE refsize rfilename fsize tsize

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs::{metadata, File, OpenOptions};
use std::path::Path;

#[derive(Eq, PartialEq)]
enum TruncateMode {
    Absolute,
    Reference,
    Extend,
    Reduce,
    AtMost,
    AtLeast,
    RoundDown,
    RoundUp,
}

static ABOUT: &str = "Shrink or extend the size of each file to the specified size.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod options {
    pub static IO_BLOCKS: &str = "io-blocks";
    pub static NO_CREATE: &str = "no-create";
    pub static REFERENCE: &str = "reference";
    pub static SIZE: &str = "size";
}

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

fn get_long_usage() -> String {
    String::from(
        "
    SIZE is an integer with an optional prefix and optional unit.
    The available units (K, M, G, T, P, E, Z, and Y) use the following format:
        'KB' =>           1000 (kilobytes)
        'K'  =>           1024 (kibibytes)
        'MB' =>      1000*1000 (megabytes)
        'M'  =>      1024*1024 (mebibytes)
        'GB' => 1000*1000*1000 (gigabytes)
        'G'  => 1024*1024*1024 (gibibytes)
    SIZE may also be prefixed by one of the following to adjust the size of each
    file based on its current size:
        '+'  => extend by
        '-'  => reduce by
        '<'  => at most
        '>'  => at least
        '/'  => round down to multiple of
        '%'  => round up to multiple of",
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let long_usage = get_long_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&long_usage[..])
        .arg(
            Arg::with_name(options::IO_BLOCKS)
            .short("o")
            .long(options::IO_BLOCKS)
            .help("treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)")
        )
        .arg(
            Arg::with_name(options::NO_CREATE)
            .short("c")
            .long(options::NO_CREATE)
            .help("do not create files that do not exist")
        )
        .arg(
            Arg::with_name(options::REFERENCE)
            .short("r")
            .long(options::REFERENCE)
            .help("base the size of each file on the size of RFILE")
            .value_name("RFILE")
        )
        .arg(
            Arg::with_name(options::SIZE)
            .short("s")
            .long("size")
            .help("set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified")
            .value_name("SIZE")
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true).min_values(1))
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        show_error!("Missing an argument");
        return 1;
    } else {
        let io_blocks = matches.is_present(options::IO_BLOCKS);
        let no_create = matches.is_present(options::NO_CREATE);
        let reference = matches.value_of(options::REFERENCE).map(String::from);
        let size = matches.value_of(options::SIZE).map(String::from);
        if reference.is_none() && size.is_none() {
            crash!(1, "you must specify either --reference or --size");
        } else {
            truncate(no_create, io_blocks, reference, size, files);
        }
    }

    0
}

fn truncate(
    no_create: bool,
    _: bool,
    reference: Option<String>,
    size: Option<String>,
    filenames: Vec<String>,
) {
    let (modsize, mode) = match size {
        Some(size_string) => parse_size(&size_string),
        None => (0, TruncateMode::Reference),
    };

    let refsize = match reference {
        Some(ref rfilename) => {
            match mode {
                // Only Some modes work with a reference
                TruncateMode::Reference => (), //No --size was given
                TruncateMode::Extend => (),
                TruncateMode::Reduce => (),
                _ => crash!(1, "you must specify a relative ‘--size’ with ‘--reference’"),
            };
            let _ = match File::open(Path::new(rfilename)) {
                Ok(m) => m,
                Err(f) => crash!(1, "{}", f.to_string()),
            };
            match metadata(rfilename) {
                Ok(meta) => meta.len(),
                Err(f) => crash!(1, "{}", f.to_string()),
            }
        }
        None => 0,
    };
    for filename in &filenames {
        let path = Path::new(filename);
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(!no_create)
            .open(path)
        {
            Ok(file) => {
                let fsize = match reference {
                    Some(_) => refsize,
                    None => match metadata(filename) {
                        Ok(meta) => meta.len(),
                        Err(f) => {
                            show_warning!("{}", f.to_string());
                            continue;
                        }
                    },
                };
                let tsize: u64 = match mode {
                    TruncateMode::Absolute => modsize,
                    TruncateMode::Reference => fsize,
                    TruncateMode::Extend => fsize + modsize,
                    TruncateMode::Reduce => fsize - modsize,
                    TruncateMode::AtMost => {
                        if fsize > modsize {
                            modsize
                        } else {
                            fsize
                        }
                    }
                    TruncateMode::AtLeast => {
                        if fsize < modsize {
                            modsize
                        } else {
                            fsize
                        }
                    }
                    TruncateMode::RoundDown => fsize - fsize % modsize,
                    TruncateMode::RoundUp => fsize + fsize % modsize,
                };
                match file.set_len(tsize) {
                    Ok(_) => {}
                    Err(f) => crash!(1, "{}", f.to_string()),
                };
            }
            Err(f) => crash!(1, "{}", f.to_string()),
        }
    }
}

fn parse_size(size: &str) -> (u64, TruncateMode) {
    let clean_size = size.replace(" ", "");
    let mode = match clean_size.chars().next().unwrap() {
        '+' => TruncateMode::Extend,
        '-' => TruncateMode::Reduce,
        '<' => TruncateMode::AtMost,
        '>' => TruncateMode::AtLeast,
        '/' => TruncateMode::RoundDown,
        '*' => TruncateMode::RoundUp,
        _ => TruncateMode::Absolute, /* assume that the size is just a number */
    };
    let bytes = {
        let mut slice = if mode == TruncateMode::Absolute {
            &clean_size
        } else {
            &clean_size[1..]
        };
        if slice.chars().last().unwrap().is_alphabetic() {
            slice = &slice[..slice.len() - 1];
            if !slice.is_empty() && slice.chars().last().unwrap().is_alphabetic() {
                slice = &slice[..slice.len() - 1];
            }
        }
        slice
    }
    .to_owned();
    let mut number: u64 = match bytes.parse() {
        Ok(num) => num,
        Err(e) => crash!(1, "'{}' is not a valid number: {}", size, e),
    };
    if clean_size.chars().last().unwrap().is_alphabetic() {
        number *= match clean_size.chars().last().unwrap().to_ascii_uppercase() {
            'B' => match clean_size
                .chars()
                .nth(clean_size.len() - 2)
                .unwrap()
                .to_ascii_uppercase()
            {
                'K' => 1000u64,
                'M' => 1000u64.pow(2),
                'G' => 1000u64.pow(3),
                'T' => 1000u64.pow(4),
                'P' => 1000u64.pow(5),
                'E' => 1000u64.pow(6),
                'Z' => 1000u64.pow(7),
                'Y' => 1000u64.pow(8),
                letter => crash!(1, "'{}B' is not a valid suffix.", letter),
            },
            'K' => 1024u64,
            'M' => 1024u64.pow(2),
            'G' => 1024u64.pow(3),
            'T' => 1024u64.pow(4),
            'P' => 1024u64.pow(5),
            'E' => 1024u64.pow(6),
            'Z' => 1024u64.pow(7),
            'Y' => 1024u64.pow(8),
            letter => crash!(1, "'{}' is not a valid suffix.", letter),
        };
    }
    (number, mode)
}
