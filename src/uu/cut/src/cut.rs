// This file is part of the uutils coreutils package.
//
// (c) Rolf Morel <rolfmorel@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim sourcefiles

#[macro_use]
extern crate uucore;

use bstr::io::BufReadExt;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::app::{get_app, options};

use self::searcher::Searcher;
use uucore::ranges::Range;
use uucore::InvalidEncodingHandling;

mod app;
mod searcher;

struct Options {
    out_delim: Option<String>,
    zero_terminated: bool,
}

struct FieldOptions {
    delimiter: String, // one char long, String because of UTF8 representation
    out_delimiter: Option<String>,
    only_delimited: bool,
    zero_terminated: bool,
}

enum Mode {
    Bytes(Vec<Range>, Options),
    Characters(Vec<Range>, Options),
    Fields(Vec<Range>, FieldOptions),
}

fn stdout_writer() -> Box<dyn Write> {
    if atty::is(atty::Stream::Stdout) {
        Box::new(stdout())
    } else {
        Box::new(BufWriter::new(stdout())) as Box<dyn Write>
    }
}

fn list_to_ranges(list: &str, complement: bool) -> Result<Vec<Range>, String> {
    if complement {
        Range::from_list(list).map(|r| uucore::ranges::complement(&r))
    } else {
        Range::from_list(list)
    }
}

fn cut_bytes<R: Read>(reader: R, ranges: &[Range], opts: &Options) -> i32 {
    let newline_char = if opts.zero_terminated { b'\0' } else { b'\n' };
    let buf_in = BufReader::new(reader);
    let mut out = stdout_writer();
    let delim = opts
        .out_delim
        .as_ref()
        .map_or("", String::as_str)
        .as_bytes();

    let res = buf_in.for_byte_record(newline_char, |line| {
        let mut print_delim = false;
        for &Range { low, high } in ranges {
            if low > line.len() {
                break;
            }
            if print_delim {
                out.write_all(delim)?;
            } else if opts.out_delim.is_some() {
                print_delim = true;
            }
            // change `low` from 1-indexed value to 0-index value
            let low = low - 1;
            let high = high.min(line.len());
            out.write_all(&line[low..high])?;
        }
        out.write_all(&[newline_char])?;
        Ok(true)
    });
    crash_if_err!(1, res);
    0
}

#[allow(clippy::cognitive_complexity)]
fn cut_fields_delimiter<R: Read>(
    reader: R,
    ranges: &[Range],
    delim: &str,
    only_delimited: bool,
    newline_char: u8,
    out_delim: &str,
) -> i32 {
    let buf_in = BufReader::new(reader);
    let mut out = stdout_writer();
    let input_delim_len = delim.len();

    let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(line, delim.as_bytes()).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if !only_delimited {
                out.write_all(line)?;
                if line[line.len() - 1] != newline_char {
                    out.write_all(&[newline_char])?;
                }
            }

            return Ok(true);
        }

        for &Range { low, high } in ranges {
            if low - fields_pos > 0 {
                low_idx = match delim_search.nth(low - fields_pos - 1) {
                    Some(index) => index + input_delim_len,
                    None => break,
                };
            }

            for _ in 0..=high - low {
                if print_delim {
                    out.write_all(out_delim.as_bytes())?;
                } else {
                    print_delim = true;
                }

                match delim_search.next() {
                    Some(high_idx) => {
                        let segment = &line[low_idx..high_idx];

                        out.write_all(segment)?;

                        low_idx = high_idx + input_delim_len;
                        fields_pos = high + 1;
                    }
                    None => {
                        let segment = &line[low_idx..];

                        out.write_all(segment)?;

                        if line[line.len() - 1] == newline_char {
                            return Ok(true);
                        }
                        break;
                    }
                }
            }
        }

        out.write_all(&[newline_char])?;
        Ok(true)
    });
    crash_if_err!(1, result);
    0
}

#[allow(clippy::cognitive_complexity)]
fn cut_fields<R: Read>(reader: R, ranges: &[Range], opts: &FieldOptions) -> i32 {
    let newline_char = if opts.zero_terminated { b'\0' } else { b'\n' };
    if let Some(ref o_delim) = opts.out_delimiter {
        return cut_fields_delimiter(
            reader,
            ranges,
            &opts.delimiter,
            opts.only_delimited,
            newline_char,
            o_delim,
        );
    }

    let buf_in = BufReader::new(reader);
    let mut out = stdout_writer();
    let delim_len = opts.delimiter.len();

    let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(line, opts.delimiter.as_bytes()).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if !opts.only_delimited {
                out.write_all(line)?;
                if line[line.len() - 1] != newline_char {
                    out.write_all(&[newline_char])?;
                }
            }

            return Ok(true);
        }

        for &Range { low, high } in ranges {
            if low - fields_pos > 0 {
                if let Some(delim_pos) = delim_search.nth(low - fields_pos - 1) {
                    low_idx = if print_delim {
                        delim_pos
                    } else {
                        delim_pos + delim_len
                    }
                } else {
                    break;
                }
            }

            match delim_search.nth(high - low) {
                Some(high_idx) => {
                    let segment = &line[low_idx..high_idx];

                    out.write_all(segment)?;

                    print_delim = true;
                    low_idx = high_idx;
                    fields_pos = high + 1;
                }
                None => {
                    let segment = &line[low_idx..line.len()];

                    out.write_all(segment)?;

                    if line[line.len() - 1] == newline_char {
                        return Ok(true);
                    }
                    break;
                }
            }
        }
        out.write_all(&[newline_char])?;
        Ok(true)
    });
    crash_if_err!(1, result);
    0
}

fn cut_files(mut filenames: Vec<String>, mode: Mode) -> i32 {
    let mut stdin_read = false;
    let mut exit_code = 0;

    if filenames.is_empty() {
        filenames.push("-".to_owned());
    }

    for filename in &filenames {
        if filename == "-" {
            if stdin_read {
                continue;
            }

            exit_code |= match mode {
                Mode::Bytes(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Characters(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Fields(ref ranges, ref opts) => cut_fields(stdin(), ranges, opts),
            };

            stdin_read = true;
        } else {
            let path = Path::new(&filename[..]);

            if path.is_dir() {
                show_error!("{}: Is a directory", filename);
                continue;
            }

            if path.metadata().is_err() {
                show_error!("{}: No such file or directory", filename);
                continue;
            }

            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    show_error!("opening '{}': {}", &filename[..], e);
                    continue;
                }
            };

            exit_code |= match mode {
                Mode::Bytes(ref ranges, ref opts) => cut_bytes(file, ranges, opts),
                Mode::Characters(ref ranges, ref opts) => cut_bytes(file, ranges, opts),
                Mode::Fields(ref ranges, ref opts) => cut_fields(file, ranges, opts),
            };
        }
    }

    exit_code
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = get_app(executable!()).get_matches_from(args);

    let complement = matches.is_present(options::COMPLEMENT);

    let mode_parse = match (
        matches.value_of(options::BYTES),
        matches.value_of(options::CHARACTERS),
        matches.value_of(options::FIELDS),
    ) {
        (Some(byte_ranges), None, None) => list_to_ranges(byte_ranges, complement).map(|ranges| {
            Mode::Bytes(
                ranges,
                Options {
                    out_delim: Some(
                        matches
                            .value_of(options::OUTPUT_DELIMITER)
                            .unwrap_or_default()
                            .to_owned(),
                    ),
                    zero_terminated: matches.is_present(options::ZERO_TERMINATED),
                },
            )
        }),
        (None, Some(char_ranges), None) => list_to_ranges(char_ranges, complement).map(|ranges| {
            Mode::Characters(
                ranges,
                Options {
                    out_delim: Some(
                        matches
                            .value_of(options::OUTPUT_DELIMITER)
                            .unwrap_or_default()
                            .to_owned(),
                    ),
                    zero_terminated: matches.is_present(options::ZERO_TERMINATED),
                },
            )
        }),
        (None, None, Some(field_ranges)) => {
            list_to_ranges(field_ranges, complement).and_then(|ranges| {
                let out_delim = match matches.value_of(options::OUTPUT_DELIMITER) {
                    Some(s) => {
                        if s.is_empty() {
                            Some("\0".to_owned())
                        } else {
                            Some(s.to_owned())
                        }
                    }
                    None => None,
                };

                let only_delimited = matches.is_present(options::ONLY_DELIMITED);
                let zero_terminated = matches.is_present(options::ZERO_TERMINATED);

                match matches.value_of(options::DELIMITER) {
                    Some(delim) => {
                        if delim.chars().count() > 1 {
                            Err(msg_opt_invalid_should_be!(
                                "empty or 1 character long",
                                "a value 2 characters or longer",
                                "--delimiter",
                                "-d"
                            ))
                        } else {
                            let delim = if delim.is_empty() {
                                "\0".to_owned()
                            } else {
                                delim.to_owned()
                            };

                            Ok(Mode::Fields(
                                ranges,
                                FieldOptions {
                                    delimiter: delim,
                                    out_delimiter: out_delim,
                                    only_delimited,
                                    zero_terminated,
                                },
                            ))
                        }
                    }
                    None => Ok(Mode::Fields(
                        ranges,
                        FieldOptions {
                            delimiter: "\t".to_owned(),
                            out_delimiter: out_delim,
                            only_delimited,
                            zero_terminated,
                        },
                    )),
                }
            })
        }
        (ref b, ref c, ref f) if b.is_some() || c.is_some() || f.is_some() => Err(
            msg_expects_no_more_than_one_of!("--fields (-f)", "--chars (-c)", "--bytes (-b)"),
        ),
        _ => Err(msg_expects_one_of!(
            "--fields (-f)",
            "--chars (-c)",
            "--bytes (-b)"
        )),
    };

    let mode_parse = match mode_parse {
        Err(_) => mode_parse,
        Ok(mode) => match mode {
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.is_present(options::DELIMITER) =>
            {
                Err(msg_opt_only_usable_if!(
                    "printing a sequence of fields",
                    "--delimiter",
                    "-d"
                ))
            }
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.is_present(options::ONLY_DELIMITED) =>
            {
                Err(msg_opt_only_usable_if!(
                    "printing a sequence of fields",
                    "--only-delimited",
                    "-s"
                ))
            }
            _ => Ok(mode),
        },
    };

    let files: Vec<String> = matches
        .values_of(options::FILE)
        .unwrap_or_default()
        .map(str::to_owned)
        .collect();

    match mode_parse {
        Ok(mode) => cut_files(files, mode),
        Err(err_msg) => {
            show_error!("{}", err_msg);
            1
        }
    }
}
