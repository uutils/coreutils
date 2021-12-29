//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

use clap::{crate_version, App, Arg};
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::iter::repeat;
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

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let serial = matches.is_present(options::SERIAL);
    let delimiters = matches.value_of(options::DELIMITER).unwrap().to_owned();
    let files = matches
        .values_of(options::FILE)
        .unwrap()
        .map(|s| s.to_owned())
        .collect();
    paste(files, serial, delimiters)
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
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
}

fn paste(filenames: Vec<String>, serial: bool, delimiters: String) -> UResult<()> {
    let mut files = vec![];
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
                match read_line(file.as_mut(), &mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        output.push_str(line.trim_end());
                        output.push_str(&delimiters[delim_count % delimiters.len()]);
                    }
                    Err(e) => return Err(e.map_err_context(String::new)),
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
                    match read_line(file.as_mut(), &mut line) {
                        Ok(0) => {
                            eof[i] = true;
                            eof_count += 1;
                        }
                        Ok(_) => output.push_str(line.trim_end()),
                        Err(e) => return Err(e.map_err_context(String::new)),
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
    Ok(())
}

// Unescape all special characters
// TODO: this will need work to conform to GNU implementation
fn unescape(s: String) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
        .replace('\\', "")
}
