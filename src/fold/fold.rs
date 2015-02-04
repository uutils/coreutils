#![crate_name = "fold"]
#![feature(collections, core, io, libc, path, rustc_private, unicode)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::old_io as io;
use std::old_io::fs::File;
use std::old_io::BufferedReader;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "fold";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let (args, obs_width) = handle_obsolete(args.as_slice());
    let program = args[0].clone();

    let opts = [
        getopts::optflag("b", "bytes", "count using bytes rather than columns (meaning control characters such as newline are not treated specially)"),
        getopts::optflag("s", "spaces", "break lines at word boundaries rather than a hard cut-off"),
        getopts::optopt("w", "width", "set WIDTH as the maximum line width rather than 80", "WIDTH"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("h") {
        println!("{} v{}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Writes each file (or standard input if no files are given) to standard output whilst breaking long lines", &opts));
    } else if matches.opt_present("V") {
        println!("{} v{}", NAME, VERSION);
    } else {
        let bytes = matches.opt_present("b");
        let spaces = matches.opt_present("s");
        let poss_width =
            if matches.opt_present("w") {
                matches.opt_str("w")
            } else {
                match obs_width {
                    Some(v) => Some(v.to_string()),
                    None => None,
                }
            };
        let width = match poss_width {
            Some(inp_width) => match inp_width.parse::<usize>() {
                Ok(width) => width,
                Err(e) => crash!(1, "illegal width value (\"{}\"): {}", inp_width, e)
            },
            None => 80
        };
        let files = if matches.free.is_empty() {
            vec!("-".to_string())
        } else {
            matches.free
        };
        fold(files, bytes, spaces, width);
    }

    0
}

fn handle_obsolete(args: &[String]) -> (Vec<String>, Option<String>) {
    for (i, arg) in args.iter().enumerate() {
        let slice = arg.as_slice();
        if slice.char_at(0) == '-' && slice.len() > 1 && slice.char_at(1).is_digit(10) {
            return (args[..i].to_vec() + &args[i + 1..],
                    Some(slice[1..].to_string()));
        }
    }
    (args.to_vec(), None)
}

#[inline]
fn fold(filenames: Vec<String>, bytes: bool, spaces: bool, width: usize) {
    for filename in filenames.iter() {
        let filename: &str = filename.as_slice();
        let mut stdin_buf;
        let mut file_buf;
        let buffer = BufferedReader::new(
            if filename == "-" {
                stdin_buf = io::stdio::stdin_raw();
                &mut stdin_buf as &mut Reader
            } else {
                file_buf = safe_unwrap!(File::open(&Path::new(filename)));
                &mut file_buf as &mut Reader
            }
        );
        fold_file(buffer, bytes, spaces, width);
    }
}

#[inline]
fn fold_file<T: io::Reader>(file: BufferedReader<T>, bytes: bool, spaces: bool, width: usize) {
    let mut file = file;
    for line in file.lines() {
        let line_string = safe_unwrap!(line);
        let mut line = line_string.as_slice();
        if bytes {
            let len = line.len();
            let mut i = 0;
            while i < len {
                let width = if len - i >= width { width } else { len - i };
                let slice = {
                    let slice = &line[i..i + width];
                    if spaces && i + width < len {
                        match slice.rfind(|&: ch: char| ch.is_whitespace()) {
                            Some(m) => &slice[..m + 1],
                            None => slice
                        }
                    } else {
                        slice
                    }
                };
                print!("{}", slice);
                i += slice.len();
            }
        } else {
            let mut len = line.chars().count();
            let newline = line.ends_with("\n");
            if newline {
                if len == 1 {
                    println!("");
                    continue;
                }
                line = &line[..line.len() - 1];
                len -= 1;
            }
            let mut output = String::new();
            let mut count = 0;
            for (i, ch) in line.chars().enumerate() {
                if count >= width {
                    let (val, ncount) = {
                        let slice = output.as_slice();
                        let (out, val, ncount) =
                            if spaces && i + 1 < len {
                                match rfind_whitespace(slice) {
                                    Some(m) => {
                                        let routput = slice.slice_chars(m + 1, slice.chars().count());
                                        let ncount = routput.chars().fold(0us, |out, ch: char| {
                                            out + match ch {
                                                '\t' => 8,
                                                '\x08' => if out > 0 { -1 } else { 0 },
                                                '\r' => return 0,
                                                _ => 1
                                            }
                                        });
                                        (slice.slice_chars(0, m + 1), routput, ncount)
                                    },
                                    None => (slice, "", 0)
                                }
                            } else {
                                (slice, "", 0)
                            };
                        println!("{}", out);
                        (val.to_string(), ncount)
                    };
                    output = val;
                    count = ncount;
                }
                match ch {
                    '\t' => {
                        count += 8;
                        if count > width {
                            println!("{}", output);
                            output.truncate(0);
                            count = 8;
                        }
                    }
                    '\x08' => {
                        if count > 0 {
                            count -= 1;
                            let len = output.len() - 1;
                            output.truncate(len);
                        }
                        continue;
                    }
                    '\r' => {
                        output.truncate(0);
                        count = 0;
                        continue;
                    }
                    _ => count += 1
                };
                output.push(ch);
            }
            if count > 0 {
                if newline {
                    println!("{}", output);
                } else {
                    print!("{}", output);
                }
            }
        }
    }
}

#[inline]
fn rfind_whitespace(slice: &str) -> Option<usize> {
    for (i, ch) in slice.chars().rev().enumerate() {
        if ch.is_whitespace() {
            return Some(slice.chars().count() - (i + 1));
        }
    }
    None
}
