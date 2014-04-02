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
    let args = os::args();

    let program = args[0].clone();

    let (args, obs_width) = handle_obsolete(args);

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
                obs_width
            };
        let width = match poss_width {
            Some(inp_width) => match uint::parse_bytes(inp_width.as_bytes(), 10) {
                Some(width) => width,
                None => crash!(1, "illegal width value (\"{}\")", inp_width)
            },
            None => 80
        };
        fold(matches.free, bytes, spaces, width);
    }
}

fn handle_obsolete(args: ~[~str]) -> (~[~str], Option<~str>) {
    let mut args = args;
    let mut i = 0;
    while i < args.len() {
        if args[i].char_at(0) == '-' && args[i].len() > 1 && args[i].char_at(1).is_digit() {
            let mut removed = args.remove(i).unwrap();
            removed.shift_char();
            return (args, Some(removed));
        }
        i += 1;
    }
    (args, None)
}

fn fold(filenames: Vec<~str>, bytes: bool, spaces: bool, width: uint) {
    if filenames.len() == 0 {
        fold_file(io::stdin(), bytes, spaces, width);
    } else {
        for filename in filenames.iter() {
            let filename: &str = *filename;
            fold_file(BufferedReader::new(safe_unwrap!(File::open(&Path::new(filename)))), bytes, spaces, width);
        }
    }
}

fn fold_file<T: io::Reader>(file: BufferedReader<T>, bytes: bool, spaces: bool, width: uint) {
    let mut file = file;
    for line in file.lines() {
        let mut line = safe_unwrap!(line);
        if line.len() == 1 {
            println!("");
            continue;
        }
        line.pop_char();
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
            let mut output = ~"";
            let mut count = 0;
            for (i, ch) in line.chars().enumerate() {
                match ch {
                    '\t' => {
                        count += 8;
                        if count > width {
                            println!("{}", output);
                            output = ~"";
                            count = 8;
                        }
                    }
                    '\x08' => {
                        if count > 0 {
                            count -= 1;
                            output.pop_char();
                        }
                        continue;
                    }
                    '\r' => {
                        output = ~"";
                        count = 0;
                        continue;
                    }
                    _ => count += 1
                };
                output.push_char(ch);
                if count == width {
                    let (val, ncount) = {
                        let (out, val, ncount) =
                            if spaces && i + 1 != line.len() {
                                match output.rfind(|ch: char| ch.is_whitespace()) {
                                    Some(m) => {
                                        let routput = output.slice_from(m + 1).to_owned();
                                        let ncount = routput.chars().fold(0, |out, ch: char| out + if ch == '\t' { 8 } else { 1 });
                                        (output.slice_to(m + 1), routput, ncount)
                                    },
                                    None => (output.as_slice(), ~"", 0)
                                }
                            } else {
                                (output.as_slice(), ~"", 0)
                            };
                        println!("{}", out);
                        (val, ncount)
                    };
                    output = val;
                    count = ncount;
                }
            }
            if count > 0 {
                println!("{}", output);
            }
        }
    }
}
