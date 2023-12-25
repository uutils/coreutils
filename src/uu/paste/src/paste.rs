// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Read, Write};
use std::path::Path;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::line_ending::LineEnding;
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("paste.md");
const USAGE: &str = help_usage!("paste.md");

mod options {
    pub const DELIMITER: &str = "delimiters";
    pub const SERIAL: &str = "serial";
    pub const FILE: &str = "file";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
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
        .cloned()
        .collect();
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));

    paste(files, serial, delimiters, line_ending)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
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

#[allow(clippy::cognitive_complexity)]
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

    if delimiters.ends_with('\\') && !delimiters.ends_with("\\\\") {
        return Err(USimpleError::new(
            1,
            format!(
                "delimiter list ends with an unescaped backslash: {}",
                delimiters
            ),
        ));
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
fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}
