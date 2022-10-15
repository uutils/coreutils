//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

use clap::{crate_version, Arg, ArgAction, Command};
use std::fmt::Display;
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
    pub const ZERO_TERMINATED: &str = "zero-terminated";
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum LineEnding {
    Newline = b'\n',
    Nul = 0,
}

impl Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Newline => writeln!(f),
            Self::Nul => write!(f, "\0"),
        }
    }
}

// Wraps BufReader and stdin
fn read_until<R: Read>(
    reader: Option<&mut BufReader<R>>,
    byte: u8,
    buf: &mut Vec<u8>,
) -> std::io::Result<usize> {
    match reader {
        Some(reader) => reader.read_until(byte, buf),
        None => stdin().lock().read_until(byte, buf),
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let serial = matches.get_flag(options::SERIAL);
    let delimiters = matches.get_one::<String>(options::DELIMITER).unwrap();
    let files = matches
        .get_many::<String>(options::FILE)
        .unwrap()
        .map(|s| s.to_owned())
        .collect();
    let line_ending = if matches.get_flag(options::ZERO_TERMINATED) {
        LineEnding::Nul
    } else {
        LineEnding::Newline
    };

    paste(files, serial, delimiters, line_ending)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::SERIAL)
                .long(options::SERIAL)
                .short('s')
                .help("paste one file at a time instead of in parallel")
                .action(ArgAction::SetTrue),
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
                .action(ArgAction::Append)
                .default_value("-")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .long(options::ZERO_TERMINATED)
                .short('z')
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
}

fn paste(
    filenames: Vec<String>,
    serial: bool,
    delimiters: &str,
    line_ending: LineEnding,
) -> UResult<()> {
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
    let mut delim_length = 1;
    let stdout = stdout();
    let mut stdout = stdout.lock();

    let mut output = Vec::new();
    if serial {
        for file in &mut files {
            output.clear();
            loop {
                match read_until(file.as_mut(), line_ending as u8, &mut output) {
                    Ok(0) => break,
                    Ok(_) => {
                        if output.ends_with(&[line_ending as u8]) {
                            output.pop();
                        }
                        // a buffer of length four is large enough to encode any char
                        let mut buffer = [0; 4];
                        let ch =
                            delimiters[delim_count % delimiters.len()].encode_utf8(&mut buffer);
                        delim_length = ch.len();

                        for byte in buffer.iter().take(delim_length) {
                            output.push(*byte);
                        }
                    }
                    Err(e) => return Err(e.map_err_context(String::new)),
                }
                delim_count += 1;
            }
            // remove final delimiter
            output.truncate(output.len() - delim_length);

            write!(
                stdout,
                "{}{}",
                String::from_utf8_lossy(&output),
                line_ending
            )?;
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
                    match read_until(file.as_mut(), line_ending as u8, &mut output) {
                        Ok(0) => {
                            eof[i] = true;
                            eof_count += 1;
                        }
                        Ok(_) => {
                            if output.ends_with(&[line_ending as u8]) {
                                output.pop();
                            }
                        }
                        Err(e) => return Err(e.map_err_context(String::new)),
                    }
                }
                // a buffer of length four is large enough to encode any char
                let mut buffer = [0; 4];
                let ch = delimiters[delim_count % delimiters.len()].encode_utf8(&mut buffer);
                delim_length = ch.len();

                for byte in buffer.iter().take(delim_length) {
                    output.push(*byte);
                }

                delim_count += 1;
            }
            if files.len() == eof_count {
                break;
            }
            // Remove final delimiter
            output.truncate(output.len() - delim_length);

            write!(
                stdout,
                "{}{}",
                String::from_utf8_lossy(&output),
                line_ending
            )?;
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
