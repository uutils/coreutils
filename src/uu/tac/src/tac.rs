//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) sbytes slen

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs::File;
use std::io::{stdin, stdout, BufReader, Read, Stdout, Write};

static NAME: &str = "tac";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static USAGE: &str = "[OPTION]... [FILE]...";
static SUMMARY: &str = "Write each file to standard output, last line first.";

mod options {
    pub static BEFORE: &str = "before";
    pub static REGEX: &str = "regex";
    pub static SEPARATOR: &str = "separator";
    pub static FILE: &str = "file";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let matches = App::new(executable!())
        .name(NAME)
        .version(VERSION)
        .usage(USAGE)
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::BEFORE)
                .short("b")
                .long(options::BEFORE)
                .help("attach the separator before instead of after")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::REGEX)
                .short("r")
                .long(options::REGEX)
                .help("interpret the sequence as a regular expression (NOT IMPLEMENTED)")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::SEPARATOR)
                .short("s")
                .long(options::SEPARATOR)
                .help("use STRING as the separator instead of newline")
                .takes_value(true),
        )
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
        .get_matches_from(args);

    let before = matches.is_present(options::BEFORE);
    let regex = matches.is_present(options::REGEX);
    let separator = match matches.value_of(options::SEPARATOR) {
        Some(m) => {
            if m.is_empty() {
                crash!(1, "separator cannot be empty")
            } else {
                m.to_owned()
            }
        }
        None => "\n".to_owned(),
    };

    let files: Vec<String> = match matches.values_of(options::FILE) {
        Some(v) => v.map(|v| v.to_owned()).collect(),
        None => vec!["-".to_owned()],
    };

    tac(files, before, regex, &separator[..])
}

fn tac(filenames: Vec<String>, before: bool, _: bool, separator: &str) -> i32 {
    let mut exit_code = 0;
    let mut out = stdout();
    let sbytes = separator.as_bytes();
    let slen = sbytes.len();

    for filename in &filenames {
        let mut file = BufReader::new(if filename == "-" {
            Box::new(stdin()) as Box<dyn Read>
        } else {
            match File::open(filename) {
                Ok(f) => Box::new(f) as Box<dyn Read>,
                Err(e) => {
                    show_warning!("failed to open '{}' for reading: {}", filename, e);
                    exit_code = 1;
                    continue;
                }
            }
        });

        let mut data = Vec::new();
        if let Err(e) = file.read_to_end(&mut data) {
            show_warning!("failed to read '{}': {}", filename, e);
            exit_code = 1;
            continue;
        };

        // find offsets in string of all separators
        let mut offsets = Vec::new();
        let mut i = 0;
        loop {
            if i + slen > data.len() {
                break;
            }

            if &data[i..i + slen] == sbytes {
                offsets.push(i);
                i += slen;
            } else {
                i += 1;
            }
        }

        // if there isn't a separator at the end of the file, fake it
        if offsets.is_empty() || *offsets.last().unwrap() < data.len() - slen {
            offsets.push(data.len());
        }

        let mut prev = *offsets.last().unwrap();
        let mut start = true;
        for off in offsets.iter().rev().skip(1) {
            // correctly handle case of no final separator in file
            if start && prev == data.len() {
                show_line(&mut out, &[], &data[*off + slen..prev], before);
                start = false;
            } else {
                show_line(&mut out, sbytes, &data[*off + slen..prev], before);
            }
            prev = *off;
        }
        show_line(&mut out, sbytes, &data[0..prev], before);
    }

    exit_code
}

fn show_line(out: &mut Stdout, sep: &[u8], dat: &[u8], before: bool) {
    if before {
        out.write_all(sep)
            .unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
    }

    out.write_all(dat)
        .unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));

    if !before {
        out.write_all(sep)
            .unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
    }
}
