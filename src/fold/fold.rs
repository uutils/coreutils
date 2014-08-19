#![crate_name = "fold"]

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

use std::io;
use std::io::fs::File;
use std::io::BufferedReader;
use std::uint;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "fold";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> int {
    let (args, obs_width) = handle_obsolete(args.as_slice());
    let program = args[0].clone();

    let opts = [
        getopts::optflag("b", "bytes", "count using bytes rather than columns (meaning control characters such as newline are not treated specially)"),
        getopts::optflag("s", "spaces", "break lines at word boundaries rather than a hard cut-off"),
        getopts::optopt("w", "width", "set WIDTH as the maximum line width rather than 80", "WIDTH"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("h") {
        println!("{} v{}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Writes each file (or standard input if no files are given) to standard output whilst breaking long lines", opts));
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
            Some(inp_width) => match uint::parse_bytes(inp_width.as_bytes(), 10) {
                Some(width) => width,
                None => crash!(1, "illegal width value (\"{}\")", inp_width)
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
    let mut args = Vec::<String>::from_slice(args);
    let mut i = 0;
    while i < args.len() {
        if args[i].as_slice().char_at(0) == '-' && args[i].len() > 1 && args[i].as_slice().char_at(1).is_digit() {
            return (args.clone(),
                    Some(args.remove(i).unwrap().as_slice().slice_from(1).to_string()));
        }
        i += 1;
    }
    (args, None)
}

fn fold(filenames: Vec<String>, bytes: bool, spaces: bool, width: uint) {
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

fn fold_file<T: io::Reader>(file: BufferedReader<T>, bytes: bool, spaces: bool, width: uint) {
    let mut file = file;
    for line in file.lines() {
        let line_string = safe_unwrap!(line);
        let mut line = line_string.as_slice();
        let len = line.len();
        if line.char_at(len - 1) == '\n' {
            if len == 1 {
                println!("");
                continue;
            } else {
                line = line.slice_to(len - 1);
            }
        }
        if bytes {
            let mut i = 0;
            while i < line.len() {
                let width = if line.len() - i >= width { width } else { line.len() - i };
                let slice = {
                    let slice = line.slice(i, i + width);
                    if spaces && i + width < line.len() {
                        match slice.rfind(|ch: char| ch.is_whitespace()) {
                            Some(m) => slice.slice_to(m + 1),
                            None => slice
                        }
                    } else {
                        slice
                    }
                };
                println!("{}", slice);
                i += slice.len();
            }
        } else {
            let mut output = String::new();
            let mut count = 0;
            for (i, ch) in line.chars().enumerate() {
                if count >= width {
                    let (val, ncount) = {
                        let slice = output.as_slice();
                        let (out, val, ncount) =
                            if spaces && i + 1 < line.len() {
                                match slice.rfind(|ch: char| ch.is_whitespace()) {
                                    Some(m) => {
                                        let routput = slice.slice_from(m + 1).to_string();
                                        let ncount = routput.as_slice().chars().fold(0u, |out, ch: char| {
                                            out + match ch {
                                                '\t' => 8,
                                                '\x08' => if out > 0 { -1 } else { 0 },
                                                '\r' => return 0,
                                                _ => 1
                                            }
                                        });
                                        (slice.slice_to(m + 1), routput, ncount)
                                    },
                                    None => (slice, "".to_string(), 0)
                                }
                            } else {
                                (slice, "".to_string(), 0)
                            };
                        println!("{}", out);
                        (val, ncount)
                    };
                    output = val.into_string();
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
                output.push_char(ch);
            }
            if count > 0 {
                print!("{}", output);
            }
        }
    }
}
