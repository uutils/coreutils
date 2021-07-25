//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) sbytes slen

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::io::{stdin, stdout, BufReader, Read, Stdout, Write};
use std::{fs::File, path::Path};
use uucore::InvalidEncodingHandling;

static NAME: &str = "tac";
static USAGE: &str = "[OPTION]... [FILE]...";
static SUMMARY: &str = "Write each file to standard output, last line first.";

mod options {
    pub static BEFORE: &str = "before";
    pub static REGEX: &str = "regex";
    pub static SEPARATOR: &str = "separator";
    pub static FILE: &str = "file";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

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

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .name(NAME)
        .version(crate_version!())
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
            let path = Path::new(filename);
            if path.is_dir() || path.metadata().is_err() {
                if path.is_dir() {
                    show_error!("dir: read error: Invalid argument");
                } else {
                    show_error!(
                        "failed to open '{}' for reading: No such file or directory",
                        filename
                    );
                }
                exit_code = 1;
                continue;
            }
            match File::open(path) {
                Ok(f) => Box::new(f) as Box<dyn Read>,
                Err(e) => {
                    show_error!("failed to open '{}' for reading: {}", filename, e);
                    exit_code = 1;
                    continue;
                }
            }
        });

        let mut data = Vec::new();
        if let Err(e) = file.read_to_end(&mut data) {
            show_error!("failed to read '{}': {}", filename, e);
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

        // If the file contains no line separators, then simply write
        // the contents of the file directly to stdout.
        if offsets.is_empty() {
            out.write_all(&data)
                .unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
            return exit_code;
        }

        // If the `-b` option was given, assume the line separators are
        // at the *beginning* of the line. Otherwise, assume the line
        // separators are at the *end* of the line.
        if before {
            if *offsets.first().unwrap() > 0 {
                offsets.insert(0, 0);
            }
            offsets.push(data.len());
            let n = offsets.len();
            for i in (0..n - 1).rev() {
                let start = offsets[i as usize];
                let end = offsets[i as usize + 1];
                show_line(&mut out, &[], &data[start..end]);
            }
        } else {
            // if there isn't a separator at the end of the file, fake it
            if *offsets.last().unwrap() < data.len() - slen {
                offsets.push(data.len());
            }

            let mut prev = *offsets.last().unwrap();
            let mut start = true;
            for off in offsets.iter().rev().skip(1) {
                // correctly handle case of no final separator in file
                if start && prev == data.len() {
                    show_line(&mut out, &[], &data[*off + slen..prev]);
                    start = false;
                } else {
                    show_line(&mut out, sbytes, &data[*off + slen..prev]);
                }
                prev = *off;
            }
            show_line(&mut out, sbytes, &data[0..prev]);
        }
    }

    exit_code
}

fn show_line(out: &mut Stdout, sep: &[u8], dat: &[u8]) {
    out.write_all(dat)
        .unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
    out.write_all(sep)
        .unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
}
