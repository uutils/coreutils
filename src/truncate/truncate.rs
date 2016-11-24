#![crate_name = "uu_truncate"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::ascii::AsciiExt;
use std::fs::{File, metadata, OpenOptions};
use std::io::{Result, Write};
use std::path::Path;

#[derive(Eq, PartialEq)]
enum TruncateMode {
    Reference,
    Extend,
    Reduce,
    AtMost,
    AtLeast,
    RoundDown,
    RoundUp
}

static NAME: &'static str = "truncate";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("c", "no-create", "do not create files that do not exist");
    opts.optflag("o", "io-blocks", "treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)");
    opts.optopt("r", "reference", "base the size of each file on the size of RFILE", "RFILE");
    opts.optopt("s", "size", "set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified", "SIZE");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => { crash!(1, "{}", f) }
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]... FILE...", NAME);
        println!("");
        print!("{}", opts.usage("Shrink or extend the size of each file to the specified size."));
        print!("
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
    '%'  => round up to multiple of
");
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else if matches.free.is_empty() {
        show_error!("missing an argument");
        return 1;
    } else {
        let no_create = matches.opt_present("no-create");
        let io_blocks = matches.opt_present("io-blocks");
        let reference = matches.opt_str("reference");
        let size = matches.opt_str("size");
        if reference.is_none() && size.is_none() {
            crash!(1, "you must specify either --reference or --size");
        } else {
            match truncate(no_create, io_blocks, reference, size, matches.free) {
                Ok(()) => ( /* pass */ ),
                Err(_) => return 1
            }
        }
    }

    0
}

fn truncate(no_create: bool, _: bool, reference: Option<String>, size: Option<String>, filenames: Vec<String>) -> Result<()> {
    let (refsize, mode) = match reference {
        Some(rfilename) => {
            let _ = match File::open(Path::new(&rfilename)) {
                Ok(m) => m,
                Err(f) => {
                    crash!(1, "{}", f.to_string())
                }
            };
            match metadata(rfilename) {
                Ok(meta) => (meta.len(), TruncateMode::Reference),
                Err(f) => {
                    crash!(1, "{}", f.to_string())
                }
            }
        }
        None => parse_size(size.unwrap().as_ref())
    };
    for filename in &filenames {
        let path = Path::new(filename);
        match OpenOptions::new().read(true).write(true).create(!no_create).open(path) {
            Ok(file) => {
                let fsize = match metadata(filename) {
                    Ok(meta) => meta.len(),
                    Err(f) => {
                        show_warning!("{}", f.to_string());
                        continue;
                    }
                };
                let tsize: u64 = match mode {
                    TruncateMode::Reference => refsize,
                    TruncateMode::Extend => fsize + refsize,
                    TruncateMode::Reduce => fsize - refsize,
                    TruncateMode::AtMost => if fsize > refsize { refsize } else { fsize },
                    TruncateMode::AtLeast => if fsize < refsize { refsize } else { fsize },
                    TruncateMode::RoundDown => fsize - fsize % refsize,
                    TruncateMode::RoundUp => fsize + fsize % refsize
                };
                match file.set_len(tsize) {
                    Ok(_) => {},
                    Err(f) => {
                        crash!(1, "{}", f.to_string())
                    }
                };
            }
            Err(f) => {
                crash!(1, "{}", f.to_string())
            }
        }
    }
    Ok(())
}

fn parse_size(size: &str) -> (u64, TruncateMode) {
    let mode = match size.chars().next().unwrap() {
        '+' => TruncateMode::Extend,
        '-' => TruncateMode::Reduce,
        '<' => TruncateMode::AtMost,
        '>' => TruncateMode::AtLeast,
        '/' => TruncateMode::RoundDown,
        '*' => TruncateMode::RoundUp,
        _ => TruncateMode::Reference /* assume that the size is just a number */
    };
    let bytes = {
        let mut slice =
            if mode == TruncateMode::Reference {
                size
            } else {
                &size[1..]
            };
        if slice.chars().last().unwrap().is_alphabetic() {
            slice = &slice[..slice.len() - 1];
            if slice.len() > 0 && slice.chars().last().unwrap().is_alphabetic() {
                slice = &slice[..slice.len() - 1];
            }
        }
        slice
    }.to_owned();
    let mut number: u64 = match bytes.parse() {
        Ok(num) => num,
        Err(e) => {
            crash!(1, "'{}' is not a valid number: {}", size, e)
        }
    };
    if size.chars().last().unwrap().is_alphabetic() {
        number *= match size.chars().last().unwrap().to_ascii_uppercase() {
            'B' => match size.chars().nth(size.len() - 2).unwrap().to_ascii_uppercase() {
                'K' => 1000u64,
                'M' => 1000u64.pow(2),
                'G' => 1000u64.pow(3),
                'T' => 1000u64.pow(4),
                'P' => 1000u64.pow(5),
                'E' => 1000u64.pow(6),
                'Z' => 1000u64.pow(7),
                'Y' => 1000u64.pow(8),
                letter => {
                    crash!(1, "'{}B' is not a valid suffix.", letter)
                }
            },
            'K' => 1024u64,
            'M' => 1024u64.pow(2),
            'G' => 1024u64.pow(3),
            'T' => 1024u64.pow(4),
            'P' => 1024u64.pow(5),
            'E' => 1024u64.pow(6),
            'Z' => 1024u64.pow(7),
            'Y' => 1024u64.pow(8),
            letter => {
                crash!(1, "'{}' is not a valid suffix.", letter)
            }
        };
    }
    (number, mode)
}
