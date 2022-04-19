//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

use clap::{crate_version, Arg, Command};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Read, Write};
use std::path::Path;
use uucore::error::{FromIo, UResult};

static ABOUT: &str = "Write lines consisting of the sequentially corresponding lines from each
FILE, separated by TABs, to standard output.";

mod options {
    pub const DELIMITER: &str = "delimiters";
    pub const SERIAL: &str = "serial";
    pub const FILE: &str = "file";
}

// Wraps BufReader and stdin
fn read_line<R: Read>(
    reader: Option<&mut BufReader<R>>,
    buf: &mut String,
) -> std::io::Result<usize> {
    match reader {
        Some(reader) => reader.read_line(buf),
        None => stdin().read_line(buf),
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let serial = matches.is_present(options::SERIAL);
    let delimiters = matches.value_of(options::DELIMITER).unwrap();
    let files = matches
        .values_of(options::FILE)
        .unwrap()
        .map(|s| s.to_owned())
        .collect();
    paste(files, serial, delimiters)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::SERIAL)
                .long(options::SERIAL)
                .short('s')
                .help("paste one file at a time instead of in parallel"),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .long(options::DELIMITER)
                .short('d')
                .help("reuse characters from LIST instead of TABs")
                .value_name("LIST")
                .default_value("\t")
                .hide_default_value(true),
        )
        .arg(
            Arg::new(options::FILE)
                .value_name("FILE")
                .multiple_occurrences(true)
                .default_value("-"),
        )
}

fn paste(filenames: Vec<String>, serial: bool, delimiters: &str) -> UResult<()> {
    let mut files = Vec::with_capacity(filenames.len());
    for name in filenames {
        let file = if name == "-" {
            None
        } else {
            let path = Path::new(&name);
            let r = File::open(path).map_err_context(String::new)?;
            Some(BufReader::new(r))
        };
        files.push(file);
    }

    let delimiters: Vec<char> = unescape(delimiters).chars().collect();
    let mut delim_count = 0;
    let stdout = stdout();
    let mut stdout = stdout.lock();

    let mut output = String::new();
    if serial {
        for file in &mut files {
            output.clear();
            loop {
                match read_line(file.as_mut(), &mut output) {
                    Ok(0) => break,
                    Ok(_) => {
                        if output.ends_with('\n') {
                            output.pop();
                        }
                        output.push(delimiters[delim_count % delimiters.len()]);
                    }
                    Err(e) => return Err(e.map_err_context(String::new)),
                }
                delim_count += 1;
            }
            output.pop();
            writeln!(stdout, "{}", output)?;
        }
    } else {
        let mut eof = vec![false; files.len()];
        loop {
            output.clear();
            let mut eof_count = 0;
            for (i, file) in files.iter_mut().enumerate() {
                if eof[i] {
                    eof_count += 1;
                } else {
                    match read_line(file.as_mut(), &mut output) {
                        Ok(0) => {
                            eof[i] = true;
                            eof_count += 1;
                        }
                        Ok(_) => {
                            if output.ends_with('\n') {
                                output.pop();
                            }
                        }
                        Err(e) => return Err(e.map_err_context(String::new)),
                    }
                }
                output.push(delimiters[delim_count % delimiters.len()]);
                delim_count += 1;
            }
            if files.len() == eof_count {
                break;
            }
            // Remove final delimiter
            output.pop();
            writeln!(stdout, "{}", output)?;
            delim_count = 0;
        }
    }
    Ok(())
}

// Unescape all special characters
// TODO: this will need work to conform to GNU implementation
fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
        .replace('\\', "")
}
