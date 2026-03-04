// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::cell::{OnceCell, RefCell};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Stdin, Write, stdin, stdout};
use std::iter::Cycle;
use std::path::Path;
use std::rc::Rc;
use std::slice::Iter;
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;
use uucore::i18n::charmap::mb_char_len;
use uucore::line_ending::LineEnding;
use uucore::translate;

mod options {
    pub const DELIMITER: &str = "delimiters";
    pub const SERIAL: &str = "serial";
    pub const FILE: &str = "file";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let serial = matches.get_flag(options::SERIAL);
    let delimiters = matches.get_one::<OsString>(options::DELIMITER).unwrap();
    let files = matches
        .get_many::<OsString>(options::FILE)
        .unwrap()
        .cloned()
        .collect();
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));

    paste(files, serial, delimiters, line_ending)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("paste-about"))
        .override_usage(format_usage(&translate!("paste-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SERIAL)
                .long(options::SERIAL)
                .short('s')
                .help(translate!("paste-help-serial"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .long(options::DELIMITER)
                .short('d')
                .help(translate!("paste-help-delimiter"))
                .value_name("LIST")
                .default_value("\t")
                .hide_default_value(true)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::FILE)
                .value_name("FILE")
                .action(ArgAction::Append)
                .default_value("-")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .long(options::ZERO_TERMINATED)
                .short('z')
                .help(translate!("paste-help-zero-terminated"))
                .action(ArgAction::SetTrue),
        )
}

#[allow(clippy::cognitive_complexity)]
fn paste(
    filenames: Vec<OsString>,
    serial: bool,
    delimiters: &OsString,
    line_ending: LineEnding,
) -> UResult<()> {
    let unescaped_and_encoded_delimiters = parse_delimiters(delimiters)?;

    let stdin_once_cell = OnceCell::<Rc<RefCell<Stdin>>>::new();

    let mut input_source_vec = Vec::with_capacity(filenames.len());

    for filename in filenames {
        let input_source = if filename == "-" {
            InputSource::StandardInput(
                stdin_once_cell
                    .get_or_init(|| Rc::new(RefCell::new(stdin())))
                    .clone(),
            )
        } else {
            let path = Path::new(&filename);
            let file = File::open(path)?;
            InputSource::File(BufReader::new(file))
        };

        input_source_vec.push(input_source);
    }

    let line_ending_byte = u8::from(line_ending);
    let input_source_vec_len = input_source_vec.len();
    let mut stdout = stdout().lock();

    if !serial && input_source_vec_len == 1 {
        // With a single input source (no -s), `paste` output is identical to input,
        // except that a missing final line ending must be added.
        // Stream directly to avoid unbounded line buffering on inputs like /dev/zero.
        return write_single_input_source(
            &mut stdout,
            input_source_vec
                .pop()
                .expect("input_source_vec_len was checked to be exactly one"),
            line_ending_byte,
        );
    }

    let line_ending_byte_array_ref = &[line_ending_byte];

    let mut delimiter_state = DelimiterState::new(&unescaped_and_encoded_delimiters);

    let mut output = Vec::new();

    if serial {
        for input_source in &mut input_source_vec {
            output.clear();

            loop {
                if input_source.read_until(line_ending_byte, &mut output)? == 0 {
                    break;
                }
                remove_trailing_line_ending_byte(line_ending_byte, &mut output);

                delimiter_state.write_delimiter(&mut output);
            }

            delimiter_state.remove_trailing_delimiter(&mut output);

            stdout.write_all(&output)?;
            stdout.write_all(line_ending_byte_array_ref)?;
        }
    } else {
        let mut eof = vec![false; input_source_vec_len];

        loop {
            output.clear();

            let mut eof_count = 0;

            for (i, input_source) in input_source_vec.iter_mut().enumerate() {
                if eof[i] {
                    eof_count += 1;
                } else {
                    match input_source.read_until(line_ending_byte, &mut output)? {
                        0 => {
                            eof[i] = true;
                            eof_count += 1;
                        }
                        _ => {
                            remove_trailing_line_ending_byte(line_ending_byte, &mut output);
                        }
                    }
                }

                delimiter_state.write_delimiter(&mut output);
            }

            if eof_count == input_source_vec_len {
                break;
            }

            delimiter_state.remove_trailing_delimiter(&mut output);

            stdout.write_all(&output)?;
            stdout.write_all(line_ending_byte_array_ref)?;

            // Quote:
            //     When the -s option is not specified:
            //     [...]
            //     The delimiter shall be reset to the first element of list after each file operand is processed.
            // https://pubs.opengroup.org/onlinepubs/9799919799/utilities/paste.html
            delimiter_state.reset_to_first_delimiter();
        }
    }

    Ok(())
}

fn write_single_input_source(
    writer: &mut impl Write,
    mut input_source: InputSource,
    line_ending_byte: u8,
) -> UResult<()> {
    let mut buffer = [0_u8; 8 * 1024];
    let mut has_data = false;
    let mut last_byte = line_ending_byte;

    loop {
        let bytes_read = input_source.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        has_data = true;
        last_byte = buffer[bytes_read - 1];

        writer.write_all(&buffer[..bytes_read])?;
    }

    if has_data && last_byte != line_ending_byte {
        writer.write_all(&[line_ending_byte])?;
    }

    Ok(())
}

fn parse_delimiters(delimiters: &OsString) -> UResult<Box<[Box<[u8]>]>> {
    let bytes = uucore::os_str_as_bytes(delimiters)?;
    let mut vec = Vec::<Box<[u8]>>::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 1;
            if i >= bytes.len() {
                return Err(USimpleError::new(
                    1,
                    translate!("paste-error-delimiter-unescaped-backslash", "delimiters" => delimiters.to_string_lossy()),
                ));
            }
            match bytes[i] {
                b'0' => vec.push(Box::new([])),
                b'\\' => vec.push(Box::new([b'\\'])),
                b'n' => vec.push(Box::new([b'\n'])),
                b't' => vec.push(Box::new([b'\t'])),
                b'b' => vec.push(Box::new([b'\x08'])),
                b'f' => vec.push(Box::new([b'\x0C'])),
                b'r' => vec.push(Box::new([b'\r'])),
                b'v' => vec.push(Box::new([b'\x0B'])),
                _ => {
                    // Unknown escape: strip backslash, use the following character(s)
                    let remaining = &bytes[i..];
                    let len = mb_char_len(remaining).min(remaining.len());
                    vec.push(Box::from(&bytes[i..i + len]));
                    i += len;
                    continue;
                }
            }
            i += 1;
        } else {
            let remaining = &bytes[i..];
            let len = mb_char_len(remaining).min(remaining.len());
            vec.push(Box::from(&bytes[i..i + len]));
            i += len;
        }
    }

    Ok(vec.into_boxed_slice())
}

fn remove_trailing_line_ending_byte(line_ending_byte: u8, output: &mut Vec<u8>) {
    if let Some(&byte) = output.last() {
        if byte == line_ending_byte {
            assert_eq!(output.pop(), Some(line_ending_byte));
        }
    }
}

enum DelimiterState<'a> {
    NoDelimiters,
    OneDelimiter(&'a [u8]),
    MultipleDelimiters {
        current_delimiter: &'a [u8],
        delimiters: &'a [Box<[u8]>],
        delimiters_iterator: Cycle<Iter<'a, Box<[u8]>>>,
    },
}

impl<'a> DelimiterState<'a> {
    fn new(unescaped_and_encoded_delimiters: &'a [Box<[u8]>]) -> Self {
        match unescaped_and_encoded_delimiters {
            [] => DelimiterState::NoDelimiters,
            [only_delimiter] => {
                // -d '\0' is equivalent to -d ''
                if only_delimiter.is_empty() {
                    DelimiterState::NoDelimiters
                } else {
                    DelimiterState::OneDelimiter(only_delimiter)
                }
            }
            [first_delimiter, ..] => DelimiterState::MultipleDelimiters {
                current_delimiter: first_delimiter,
                delimiters: unescaped_and_encoded_delimiters,
                delimiters_iterator: unescaped_and_encoded_delimiters.iter().cycle(),
            },
        }
    }

    /// This should only be used to return to the start of the delimiter list after a file has been processed.
    /// This should only be used when the "serial" option is disabled.
    /// This is a no-op unless there are multiple delimiters.
    fn reset_to_first_delimiter(&mut self) {
        if let DelimiterState::MultipleDelimiters {
            delimiters_iterator,
            delimiters,
            ..
        } = self
        {
            *delimiters_iterator = delimiters.iter().cycle();
        }
    }

    /// Remove the trailing delimiter.
    /// If there are no delimiters, this is a no-op.
    fn remove_trailing_delimiter(&mut self, output: &mut Vec<u8>) {
        let delimiter_length = match self {
            DelimiterState::OneDelimiter(only_delimiter) => only_delimiter.len(),
            DelimiterState::MultipleDelimiters {
                current_delimiter, ..
            } => current_delimiter.len(),
            DelimiterState::NoDelimiters => {
                return;
            }
        };

        // `delimiter_length` will be zero if the current delimiter is a "\0" delimiter
        if delimiter_length > 0 {
            let output_len = output.len();

            if let Some(output_without_delimiter_length) = output_len.checked_sub(delimiter_length)
            {
                output.truncate(output_without_delimiter_length);
            } else {
                // This branch is NOT unreachable, must be skipped
                // `output` should be empty in this case
                assert_eq!(output_len, 0);
            }
        }
    }

    /// Append the current delimiter to `output`.
    /// If there are no delimiters, this is a no-op.
    fn write_delimiter(&mut self, output: &mut Vec<u8>) {
        match self {
            DelimiterState::OneDelimiter(only_delimiter) => {
                output.extend_from_slice(only_delimiter);
            }
            DelimiterState::MultipleDelimiters {
                current_delimiter,
                delimiters_iterator,
                ..
            } => {
                // Unwrap because `delimiters_iterator` is a cycle iter and was created from a non-empty slice
                let bo = delimiters_iterator.next().unwrap();

                output.extend_from_slice(bo);

                *current_delimiter = bo;
            }
            DelimiterState::NoDelimiters => {}
        }
    }
}

enum InputSource {
    File(BufReader<File>),
    StandardInput(Rc<RefCell<Stdin>>),
}

impl InputSource {
    fn read(&mut self, buf: &mut [u8]) -> UResult<usize> {
        let us = match self {
            Self::File(bu) => bu.read(buf)?,
            Self::StandardInput(rc) => rc
                .try_borrow()
                .map_err(|bo| {
                    USimpleError::new(1, translate!("paste-error-stdin-borrow", "error" => bo))
                })?
                .lock()
                .read(buf)?,
        };

        Ok(us)
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> UResult<usize> {
        let us = match self {
            Self::File(bu) => bu.read_until(byte, buf)?,
            Self::StandardInput(rc) => rc
                .try_borrow()
                .map_err(|bo| {
                    USimpleError::new(1, translate!("paste-error-stdin-borrow", "error" => bo))
                })?
                .lock()
                .read_until(byte, buf)?,
        };

        Ok(us)
    }
}
