#![crate_id(name = "fold", vers = "1.0.0", author = "Arcterus")]

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
use std::os;
use std::uint;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "fold";
static VERSION: &'static str = "1.0.0";

fn main() {

    let (args, obs_width) = handle_obsolete(os::args().as_slice().to_owned());
    let program = args.get(0).clone();
    let args: Vec<StrBuf> = os::args().iter().map(|x| x.to_strbuf()).collect();

    let opts = [
        getopts::optflag("b", "bytes", "count using bytes rather than columns (meaning control characters such as newline are not treated specially)"),
        getopts::optflag("s", "spaces", "break lines at word boundaries rather than a hard cut-off"),
        getopts::optopt("w", "width", "set WIDTH as the maximum line width rather than 80", "WIDTH"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f.to_err_msg())
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
                    Some(v) => Some(v.to_strbuf()),
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
            vec!("-".to_strbuf())
        } else {
            matches.free
        };
        fold(files, bytes, spaces, width);
    }
}

fn handle_obsolete(args: &[StrBuf]) -> (Vec<StrBuf>, Option<StrBuf>) {
    let mut args = Vec::<StrBuf>::from_slice(args);
    let mut i = 0;
    while i < args.len() {
        if args.get(i).as_slice().char_at(0) == '-' && args.get(i).len() > 1 && args.get(i).as_slice().char_at(1).is_digit() {
            return (args.clone(),
                    Some(args.remove(i).unwrap().as_slice().slice_from(1).to_owned()));
        }
        i += 1;
    }
    (args, None)
}

fn fold(filenames: Vec<StrBuf>, bytes: bool, spaces: bool, width: uint) {
    for filename in filenames.iter() {
        let filename: &str = filename.as_slice();
        let buffer = BufferedReader::new(
            if filename == "-" {
                box io::stdio::stdin_raw() as Box<Reader>
            } else {
                box safe_unwrap!(File::open(&Path::new(filename))) as Box<Reader>
            }
        );
        fold_file(buffer, bytes, spaces, width);
    }
}

fn fold_file<T: io::Reader>(file: BufferedReader<T>, bytes: bool, spaces: bool, width: uint) {
    let mut file = file;
    for line in file.lines() {
        let line = safe_unwrap!(line);
        if line.len() == 1 {
            println!("");
            continue;
        }
        let line = line.as_slice().slice_to(line.len() - 1);
        if bytes {
            let mut i = 0;
            while i < line.len() {
                let width = if line.len() - i >= width { width } else { line.len() - i };
                let slice = {
                    let slice = line.slice(i, i + width);
                    if spaces && i + width != line.len() {
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
            let mut output = StrBuf::new();
            let mut count = 0;
            for (i, ch) in line.chars().enumerate() {
                match ch {
                    '\t' => {
                        count += 8;
                        if count > width {
                            println!("{}", output.as_slice());
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
                if count == width {
                    let (val, ncount) = {
                        let slice = output.as_slice();
                        let (out, val, ncount) =
                            if spaces && i + 1 != line.len() {
                                match slice.rfind(|ch: char| ch.is_whitespace()) {
                                    Some(m) => {
                                        let routput = slice.slice_from(m + 1).to_owned();
                                        let ncount = routput.as_slice().chars().fold(0, |out, ch: char| out + if ch == '\t' { 8 } else { 1 });
                                        (slice.slice_to(m + 1), routput, ncount)
                                    },
                                    None => (slice, "".to_owned(), 0)
                                }
                            } else {
                                (slice, "".to_owned(), 0)
                            };
                        println!("{}", out);
                        (val, ncount)
                    };
                    output = val.into_strbuf();
                    count = ncount;
                }
            }
            if count > 0 {
                println!("{}", output);
            }
        }
    }
}
