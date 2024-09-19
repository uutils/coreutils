// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::slice::Iter;
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
            format!("delimiter list ends with an unescaped backslash: {delimiters}"),
        ));
    }

    struct DelimiterData<'a> {
        current_delimiter_length: usize,
        delimiters_encoded: &'a [Box<[u8]>],
        delimiters_encoded_iter: Iter<'a, Box<[u8]>>,
    }

    // Precompute instead of doing this inside the loops
    let mut delimiters_encoded_option = {
        let delimiters_unescaped = unescape(delimiters).chars().collect::<Vec<_>>();

        let number_of_delimiters = delimiters_unescaped.len();

        if number_of_delimiters > 0 {
            let mut vec = Vec::<Box<[u8]>>::with_capacity(number_of_delimiters);

            {
                // a buffer of length four is large enough to encode any char
                let mut buffer = [0_u8; 4_usize];

                for ch in delimiters_unescaped {
                    let delimiter_encoded = ch.encode_utf8(&mut buffer);

                    vec.push(Box::from(delimiter_encoded.as_bytes()));
                }
            }

            Some(vec.into_boxed_slice())
        } else {
            None
        }
    };

    let mut delimiter_data_option = match &mut delimiters_encoded_option {
        &mut Some(ref mut delimiters_encoded) => {
            // TODO
            // Is this initial value correct?
            let current_delimiter_length = delimiters_encoded.first().unwrap().len();

            Some(DelimiterData {
                delimiters_encoded,
                delimiters_encoded_iter: delimiters_encoded.iter(),
                current_delimiter_length,
            })
        }
        None => None,
    };

    let stdout = stdout();
    let mut stdout = stdout.lock();

    let mut output = Vec::new();

    if serial {
        for file in &mut files {
            output.clear();

            loop {
                let current_delimiter_option = match &mut delimiter_data_option {
                    Some(DelimiterData {
                        current_delimiter_length,
                        delimiters_encoded,
                        delimiters_encoded_iter,
                    }) => {
                        let current_delimiter = if let Some(delimiter_from_current_iter) =
                            delimiters_encoded_iter.next()
                        {
                            delimiter_from_current_iter
                        } else {
                            // Reset iter after hitting the end
                            *delimiters_encoded_iter = delimiters_encoded.iter();

                            // Unwrapping because:
                            // 1) `delimiters_encoded` is non-empty
                            // 2) `delimiters_encoded_iter` is a newly constructed Iter
                            // So `next` should always return an element
                            delimiters_encoded_iter.next().unwrap()
                        };

                        *current_delimiter_length = current_delimiter.len();

                        Some(current_delimiter)
                    }
                    None => None,
                };

                match read_until(file.as_mut(), line_ending as u8, &mut output) {
                    Ok(0) => break,
                    Ok(_) => {
                        if output.ends_with(&[line_ending as u8]) {
                            output.pop();
                        }

                        // Write delimiter, if one exists, to output
                        if let Some(current_delimiter) = current_delimiter_option {
                            output.extend_from_slice(current_delimiter);
                        }
                    }
                    Err(e) => return Err(e.map_err_context(String::new)),
                }
            }

            if let Some(DelimiterData {
                current_delimiter_length,
                ..
            }) = &mut delimiter_data_option
            {
                // Remove trailing delimiter, if there is a delimiter
                // It's safe to truncate to zero (or to a length greater than the length of the Vec),so as long as
                // the subtraction didn't panic, this should be fine
                if let Some(us) = output.len().checked_sub(*current_delimiter_length) {
                    output.truncate(us);
                }
            }

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
                let current_delimiter_option = if let Some(DelimiterData {
                    current_delimiter_length,
                    delimiters_encoded,
                    delimiters_encoded_iter,
                }) = &mut delimiter_data_option
                {
                    let current_delimiter = if let Some(bo) = delimiters_encoded_iter.next() {
                        bo
                    } else {
                        // Reset iter after hitting the end
                        *delimiters_encoded_iter = delimiters_encoded.iter();

                        // Unwrapping because:
                        // 1) `delimiters_encoded` is non-empty
                        // 2) `delimiters_encoded_iter` is a newly constructed Iter
                        // So `next` should always return an element
                        delimiters_encoded_iter.next().unwrap()
                    };

                    *current_delimiter_length = current_delimiter.len();

                    Some(current_delimiter)
                } else {
                    None
                };

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

                // Write delimiter, if one exists, to output
                if let Some(current_delimiter) = current_delimiter_option {
                    output.extend_from_slice(current_delimiter);
                }
            }

            if files.len() == eof_count {
                break;
            }

            if let Some(DelimiterData {
                current_delimiter_length,
                delimiters_encoded,
                delimiters_encoded_iter,
            }) = &mut delimiter_data_option
            {
                // Reset iter after file is processed
                *delimiters_encoded_iter = delimiters_encoded.iter();

                // Remove trailing delimiter, if there is a delimiter
                // It's safe to truncate to zero (or to a length greater than the length of the Vec),so as long as
                // the subtraction didn't panic, this should be fine
                if let Some(us) = output.len().checked_sub(*current_delimiter_length) {
                    output.truncate(us);
                }
            }

            write!(
                stdout,
                "{}{}",
                String::from_utf8_lossy(&output),
                line_ending
            )?;
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
