#![crate_name = "truncate"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::io::{File, Open, ReadWrite, fs};
use std::io::fs::PathExtensions;
use std::u64;

#[path = "../common/util.rs"]
mod util;

#[deriving(Eq, PartialEq)]
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

pub fn uumain(args: Vec<String>) -> int {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("c", "no-create", "do not create files that do not exist"),
        getopts::optflag("o", "io-blocks", "treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)"),
        getopts::optopt("r", "reference", "base the size of each file on the size of RFILE", "RFILE"),
        getopts::optopt("s", "size", "set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified", "SIZE"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "{}", f)
        }
    };

    if matches.opt_present("help") {
        println!("truncate 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... FILE...", program);
        println!("");
        print!("{}", getopts::usage("Shrink or extend the size of each file to the specified size.", opts));
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
        println!("truncate 1.0.0");
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
                Err(e) => return e
            }
        }
    }

    0
}

fn truncate(no_create: bool, _: bool, reference: Option<String>, size: Option<String>, filenames: Vec<String>) -> Result<(), int> {
    let (refsize, mode) = match reference {
        Some(rfilename) => {
            let rfile = match File::open(&Path::new(rfilename.clone())) {
                Ok(m) => m,
                Err(f) => {
                    crash!(1, "{}", f.to_string())
                }
            };
            match fs::stat(rfile.path()) {
                Ok(stat) => (stat.size, Reference),
                Err(f) => {
                    show_error!("{}", f.to_string());
                    return Err(1);
                }
            }
        }
        None => parse_size(size.unwrap().as_slice())
    };
    for filename in filenames.iter() {
        let filename = filename.as_slice();
        let path = Path::new(filename);
        if path.exists() || !no_create {
            match File::open_mode(&path, Open, ReadWrite) {
                Ok(mut file) => {
                    let fsize = match fs::stat(file.path()) {
                        Ok(stat) => stat.size,
                        Err(f) => {
                            show_warning!("{}", f.to_string());
                            continue;
                        }
                    };
                    let tsize = match mode {
                        Reference => refsize,
                        Extend => fsize + refsize,
                        Reduce => fsize - refsize,
                        AtMost => if fsize > refsize { refsize } else { fsize },
                        AtLeast => if fsize < refsize { refsize } else { fsize },
                        RoundDown => fsize - fsize % refsize,
                        RoundUp => fsize + fsize % refsize
                    };
                    match file.truncate(tsize as i64) {
                        Ok(_) => {}
                        Err(f) => {
                            show_error!("{}", f.to_string());
                            return Err(1);
                        }
                    }
                }
                Err(f) => {
                    show_error!("{}", f.to_string());
                    return Err(1);
                }
            }
        }
    }
    Ok(())
}

fn parse_size(size: &str) -> (u64, TruncateMode) {
    let mode = match size.char_at(0) {
        '+' => Extend,
        '-' => Reduce,
        '<' => AtMost,
        '>' => AtLeast,
        '/' => RoundDown,
        '*' => RoundUp,
        _ => Reference /* assume that the size is just a number */
    };
    let bytes = {
        let mut slice =
            if mode == Reference {
                let size: &str = size;
                size
            } else {
                size.slice_from(1)
            };
        if slice.char_at(slice.len() - 1).is_alphabetic() {
            slice = slice.slice_to(slice.len() - 1);
            if slice.len() > 0 && slice.char_at(slice.len() - 1).is_alphabetic() {
                slice = slice.slice_to(slice.len() - 1);
            }
        }
        slice
    }.to_string().into_bytes();
    let mut number = match u64::parse_bytes(bytes.as_slice(), 10) {
        Some(num) => num,
        None => {
            crash!(1, "'{}' is not a valid number.", size)
        }
    };
    if size.char_at(size.len() - 1).is_alphabetic() {
        number *= match size.char_at(size.len() - 1).to_ascii().to_uppercase().to_char() {
            'B' => match size.char_at(size.len() - 2).to_ascii().to_uppercase().to_char() {
                'K' => 1000,
                'M' => 1000 * 1000,
                'G' => 1000 * 1000 * 1000,
                'T' => 1000 * 1000 * 1000 * 1000,
                'P' => 1000 * 1000 * 1000 * 1000 * 1000,
                'E' => 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                'Z' => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                'Y' => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                letter => {
                    crash!(1, "'{}B' is not a valid suffix.", letter)
                }
            },
            'K' => 1024,
            'M' => 1024 * 1024,
            'G' => 1024 * 1024 * 1024,
            'T' => 1024 * 1024 * 1024 * 1024,
            'P' => 1024 * 1024 * 1024 * 1024 * 1024,
            'E' => 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            'Z' => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            'Y' => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            letter => {
                crash!(1, "'{}' is not a valid suffix.", letter)
            }
        };
    }
    (number, mode)
}
