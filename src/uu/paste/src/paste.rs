//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Stdin};
use std::iter::repeat;
use std::path::Path;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Write lines consisting of the sequentially corresponding lines from each
FILE, separated by TABs, to standard output.";

mod options {
    pub const DELIMITER: &str = "delimiters";
    pub const SERIAL: &str = "serial";
    pub const FILE: &str = "file";
}

// We need this trait to wrap both BufReader and Stdin. We need
// `read_line` function only, but Stdin does not provide BufRead
// unless lock function is called, which prevents us from using stdin
// multiple times
trait ReadLine {
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize>;
}

struct StdinReadLine(Stdin);
struct BufReadReadLine<R: BufRead>(R);

impl ReadLine for StdinReadLine {
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        return self.0.read_line(buf);
    }
}

impl<R: BufRead> ReadLine for BufReadReadLine<R> {
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        return self.0.read_line(buf);
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .arg(
            Arg::with_name(options::SERIAL)
                .long(options::SERIAL)
                .short("s")
                .help("paste one file at a time instead of in parallel"),
        )
        .arg(
            Arg::with_name(options::DELIMITER)
                .long(options::DELIMITER)
                .short("d")
                .help("reuse characters from LIST instead of TABs")
                .value_name("LIST")
                .default_value("\t")
                .hide_default_value(true),
        )
        .arg(
            Arg::with_name(options::FILE)
                .value_name("FILE")
                .multiple(true)
                .default_value("-"),
        )
        .get_matches_from(args);

    let serial = matches.is_present(options::SERIAL);
    let delimiters = matches.value_of(options::DELIMITER).unwrap().to_owned();
    let files = matches
        .values_of(options::FILE)
        .unwrap()
        .map(|s| s.to_owned())
        .collect();
    paste(files, serial, delimiters);

    0
}

fn paste(filenames: Vec<String>, serial: bool, delimiters: String) {
    let mut files: Vec<Box<dyn ReadLine>> = filenames
        .into_iter()
        .map(|name| {
            if name == "-" {
                Box::new(StdinReadLine(stdin())) as Box<dyn ReadLine>
            } else {
                let r = crash_if_err!(1, File::open(Path::new(&name)));
                Box::new(BufReadReadLine(BufReader::new(r))) as Box<dyn ReadLine>
            }
        })
        .collect();

    let delimiters: Vec<String> = unescape(delimiters)
        .chars()
        .map(|x| x.to_string())
        .collect();
    let mut delim_count = 0;

    if serial {
        for file in &mut files {
            let mut output = String::new();
            loop {
                let mut line = String::new();
                match file.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        output.push_str(line.trim_end());
                        output.push_str(&delimiters[delim_count % delimiters.len()]);
                    }
                    Err(e) => crash!(1, "{}", e.to_string()),
                }
                delim_count += 1;
            }
            println!("{}", &output[..output.len() - 1]);
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
                        Ok(_) => output.push_str(line.trim_end()),
                        Err(e) => crash!(1, "{}", e.to_string()),
                    }
                }
                output.push_str(&delimiters[delim_count % delimiters.len()]);
                delim_count += 1;
            }
            if files.len() == eof_count {
                break;
            }
            println!("{}", &output[..output.len() - 1]);
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
