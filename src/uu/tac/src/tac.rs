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
use std::io::{self, stdin, stdout, Cursor, ErrorKind, Read, Seek, Write};
use std::{fs::File, path::Path};
use uucore::InvalidEncodingHandling;

static NAME: &str = "tac";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static USAGE: &str = "[OPTION]... [FILE]...";
static SUMMARY: &str = "Write each file to standard output, last line first.";

mod chunks;
mod lines;
use lines::rlines_leading_separator;
use lines::rlines_trailing_separator;

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

fn generic_tac<T>(f: &mut T, separator: &str, before: bool) -> io::Result<()>
where
    T: Read + Seek,
{
    let mut out = stdout();
    let sep = separator.as_bytes().first().unwrap();
    if before {
        for line in rlines_leading_separator(f, *sep) {
            out.write_all(&line)?;
        }
    } else {
        for line in rlines_trailing_separator(f, *sep) {
            out.write_all(&line)?;
        }
    }
    Ok(())
}

/// Print lines of `stdin` in reverse.
fn stdin_tac(separator: &str, before: bool) -> io::Result<()> {
    let mut data = Vec::new();
    stdin().read_to_end(&mut data).unwrap();
    let mut file = Cursor::new(&data);
    generic_tac(&mut file, separator, before)
}

/// Print lines of the given file in reverse.
fn file_tac(filename: &str, separator: &str, before: bool) -> io::Result<()> {
    let mut file = File::open(Path::new(filename))?;
    generic_tac(&mut file, separator, before)
}

fn tac(filenames: Vec<String>, before: bool, _: bool, separator: &str) -> i32 {
    let mut exit_code = 0;
    for filename in &filenames {
        if filename == "-" {
            if let Err(e) = stdin_tac(separator, before) {
                show_error!("failed to read '{}': {}", filename, e);
                exit_code = 1;
            }
        } else {
            if Path::new(filename).is_dir() {
                show_error!("{}: read error: Invalid argument", filename);
                exit_code = 1;
                continue;
            }
            if let Err(e) = file_tac(filename, separator, before) {
                match e.kind() {
                    ErrorKind::NotFound => {
                        show_error!("failed to open '{}' for reading: {}", filename, e)
                    }
                    _ => show_error!("failed to read '{}': {}", filename, e),
                }
                exit_code = 1;
            }
        }
    }
    exit_code
}
