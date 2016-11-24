#![crate_name = "uu_tac"]

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

use std::fs::File;
use std::io::{BufReader, Read, stdin, stdout, Stdout, Write};

static NAME: &'static str = "tac";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("b", "before", "attach the separator before instead of after");
    opts.optflag("r", "regex", "interpret the sequence as a regular expression (NOT IMPLEMENTED)");
    opts.optopt("s", "separator", "use STRING as the separator instead of newline", "STRING");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };
    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTION]... [FILE]...

Write each file to standard output, last line first.", NAME, VERSION);

        print!("{}", opts.usage(&msg));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let before = matches.opt_present("b");
        let regex = matches.opt_present("r");
        let separator = match matches.opt_str("s") {
            Some(m) => {
                if m.is_empty() {
                    crash!(1, "separator cannot be empty")
                } else {
                    m
                }
            }
            None => "\n".to_owned()
        };
        let files = if matches.free.is_empty() {
            vec!("-".to_owned())
        } else {
            matches.free
        };
        tac(files, before, regex, &separator[..]);
    }

    0
}

fn tac(filenames: Vec<String>, before: bool, _: bool, separator: &str) {
    let mut out = stdout();
    let sbytes = separator.as_bytes();
    let slen = sbytes.len();

    for filename in &filenames {
        let mut file = BufReader::new(
            if filename == "-" {
                Box::new(stdin()) as Box<Read>
            } else {
                match File::open(filename) {
                    Ok(f) => Box::new(f) as Box<Read>,
                    Err(e) => {
                        show_warning!("failed to open '{}' for reading: {}", filename, e);
                        continue;
                    },
                }
            });

        let mut data = Vec::new();
        match file.read_to_end(&mut data) {
            Err(e) => {
                show_warning!("failed to read '{}': {}", filename, e);
                continue;
            },
            Ok(_) => (),
        };

        // find offsets in string of all separators
        let mut offsets = Vec::new();
        let mut i = 0;
        loop {
            if i + slen > data.len() {
                break;
            }

            if &data[i..i+slen] == sbytes {
                offsets.push(i);
                i += slen;
            } else {
                i += 1;
            }
        }
        drop(i);

        // if there isn't a separator at the end of the file, fake it
        if offsets.is_empty() || *offsets.last().unwrap() < data.len() - slen {
            offsets.push(data.len());
        }

        let mut prev = *offsets.last().unwrap();
        let mut start = true;
        for off in offsets.iter().rev().skip(1) {
            // correctly handle case of no final separator in file
            if start && prev == data.len() {
                show_line(&mut out, &[], &data[*off+slen..prev], before);
                start = false;
            } else {
                show_line(&mut out, sbytes, &data[*off+slen..prev], before);
            }
            prev = *off;
        }
        show_line(&mut out, sbytes, &data[0..prev], before);
    }
}

fn show_line(out: &mut Stdout, sep: &[u8], dat: &[u8], before: bool) {
    if before {
        out.write_all(sep).unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
    }

    out.write_all(dat).unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));

    if !before {
        out.write_all(sep).unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
    }
}
