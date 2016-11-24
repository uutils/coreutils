#![crate_name = "uu_paste"]

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

use std::io::{BufRead, BufReader, Read, stdin, Write};
use std::iter::repeat;
use std::fs::File;
use std::path::Path;

static NAME: &'static str = "paste";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("s", "serial", "paste one file at a time instead of in parallel");
    opts.optopt("d", "delimiters", "reuse characters from LIST instead of TABs", "LIST");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => crash!(1, "{}", e)
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
  {0} [OPTION]... [FILE]...

Write lines consisting of the sequentially corresponding lines from each
FILE, separated by TABs, to standard output.", NAME, VERSION);
        print!("{}", opts.usage(&msg));
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let serial = matches.opt_present("serial");
        let delimiters = matches.opt_str("delimiters").unwrap_or("\t".to_owned());
        paste(matches.free, serial, delimiters);
    }

    0
}

fn paste(filenames: Vec<String>, serial: bool, delimiters: String) {
    let mut files: Vec<BufReader<Box<Read>>> = filenames.into_iter().map(|name|
        BufReader::new(
            if name == "-" {
                Box::new(stdin()) as Box<Read>
            } else {
                let r = crash_if_err!(1, File::open(Path::new(&name)));
                Box::new(r) as Box<Read>
            }
        )
    ).collect();

    let delimiters: Vec<String> = unescape(delimiters).chars().map(|x| x.to_string()).collect();
    let mut delim_count = 0;

    if serial {
        for file in &mut files {
            let mut output = String::new();
            loop {
                let mut line = String::new();
                match file.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        output.push_str(line.trim_right());
                        output.push_str(&delimiters[delim_count % delimiters.len()]);
                    }
                    Err(e) => crash!(1, "{}", e.to_string())
                }
                delim_count += 1;
            }
            println!("{}", &output[..output.len()-1]);
        }
    } else {
        let mut eof: Vec<bool> = repeat(false).take(files.len()).collect();
        loop {
            let mut output = String::new();
            let mut eof_count = 0;
            for (i, file) in files.iter_mut().enumerate() {
                if eof[i] {
                    eof_count += 1;
                } else {
                    let mut line = String::new();
                    match file.read_line(&mut line) {
                        Ok(0) => {
                            eof[i] = true;
                            eof_count += 1;
                        }
                        Ok(_) => output.push_str(line.trim_right()),
                        Err(e) => crash!(1, "{}", e.to_string())
                    }
                }
                output.push_str(&delimiters[delim_count % delimiters.len()]);
                delim_count += 1;
            }
            if files.len() == eof_count {
                break;
            }
            println!("{}", &output[..output.len()-1]);
            delim_count = 0;
        }
    }
}

// Unescape all special characters
// TODO: this will need work to conform to GNU implementation
fn unescape(s: String) -> String {
    s.replace("\\n", "\n")
     .replace("\\t", "\t")
     .replace("\\\\", "\\")
     .replace("\\", "")
}
