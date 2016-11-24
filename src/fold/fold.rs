#![crate_name = "uu_fold"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdin, Write};
use std::path::Path;

static SYNTAX: &'static str = "[OPTION]... [FILE]..."; 
static SUMMARY: &'static str = "Writes each file (or standard input if no files are given) 
 to standard output whilst breaking long lines"; 
static LONG_HELP: &'static str = ""; 

pub fn uumain(args: Vec<String>) -> i32 {
    let (args, obs_width) = handle_obsolete(&args[..]);
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("b", "bytes", "count using bytes rather than columns (meaning control characters such as newline are not treated specially)")
        .optflag("s", "spaces", "break lines at word boundaries rather than a hard cut-off")
        .optopt("w", "width", "set WIDTH as the maximum line width rather than 80", "WIDTH")
        .parse(args);

    let bytes = matches.opt_present("b");
    let spaces = matches.opt_present("s");
    let poss_width =
        if matches.opt_present("w") {
            matches.opt_str("w")
        } else {
            obs_width
        };
    let width = match poss_width {
        Some(inp_width) => match inp_width.parse::<usize>() {
            Ok(width) => width,
            Err(e) => crash!(1, "illegal width value (\"{}\"): {}", inp_width, e)
        },
        None => 80
    };
    let files = if matches.free.is_empty() {
        vec!("-".to_owned())
    } else {
        matches.free
    };
    fold(files, bytes, spaces, width);

    0
}

fn handle_obsolete(args: &[String]) -> (Vec<String>, Option<String>) {
    for (i, arg) in args.iter().enumerate() {
        let slice = &arg;
        if slice.chars().next().unwrap() == '-' && slice.len() > 1 && slice.chars().nth(1).unwrap().is_digit(10) {
            let mut v = args.to_vec();
            v.remove(i);
            return (v, Some(slice[1..].to_owned()));
        }
    }
    (args.to_vec(), None)
}

#[inline]
fn fold(filenames: Vec<String>, bytes: bool, spaces: bool, width: usize) {
    for filename in &filenames {
        let filename: &str = &filename;
        let mut stdin_buf;
        let mut file_buf;
        let buffer = BufReader::new(
            if filename == "-" {
                stdin_buf = stdin();
                &mut stdin_buf as &mut Read
            } else {
                file_buf = safe_unwrap!(File::open(Path::new(filename)));
                &mut file_buf as &mut Read
            }
        );
        fold_file(buffer, bytes, spaces, width);
    }
}

#[inline]
fn fold_file<T: Read>(mut file: BufReader<T>, bytes: bool, spaces: bool, width: usize) {
    let mut line = String::new();
    while safe_unwrap!(file.read_line(&mut line)) > 0 {
        if bytes {
            let len = line.len();
            let mut i = 0;
            while i < len {
                let width = if len - i >= width { width } else { len - i };
                let slice = {
                    let slice = &line[i..i + width];
                    if spaces && i + width < len {
                        match slice.rfind(|ch: char| ch.is_whitespace()) {
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
                len -= 1;
                line.truncate(len);
            }
            let mut output = String::new();
            let mut count = 0;
            for (i, ch) in line.chars().enumerate() {
                if count >= width {
                    let (val, ncount) = {
                        let slice = &output[..];
                        let (out, val, ncount) =
                            if spaces && i + 1 < len {
                                match rfind_whitespace(slice) {
                                    Some(m) => {
                                        let routput = &slice[m + 1 .. slice.chars().count()];
                                        let ncount = routput.chars().fold(0, |out, ch: char| {
                                            out + match ch {
                                                '\t' => 8,
                                                '\x08' => if out > 0 { !0 } else { 0 },
                                                '\r' => return 0,
                                                _ => 1
                                            }
                                        });
                                        (&slice[0 .. m + 1], routput, ncount)
                                    },
                                    None => (slice, "", 0)
                                }
                            } else {
                                (slice, "", 0)
                            };
                        println!("{}", out);
                        (val.to_owned(), ncount)
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
