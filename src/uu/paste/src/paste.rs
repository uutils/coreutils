//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::iter::repeat;
use std::path::Path;

use crate::app::{get_app, options};

pub mod app;

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

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = get_app(executable!()).get_matches_from(args);

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
    let mut files: Vec<_> = filenames
        .into_iter()
        .map(|name| {
            if name == "-" {
                None
            } else {
                let r = crash_if_err!(1, File::open(Path::new(&name)));
                Some(BufReader::new(r))
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
                match read_line(file.as_mut(), &mut line) {
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
                    match read_line(file.as_mut(), &mut line) {
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
