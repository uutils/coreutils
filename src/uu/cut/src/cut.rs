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
use clap::{crate_version, Arg, Command};
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};

use self::searcher::Searcher;
use uucore::ranges::Range;
use uucore::{format_usage, InvalidEncodingHandling};

mod searcher;

static NAME: &str = "cut";
static USAGE: &str =
    "{} [-d] [-s] [-z] [--output-delimiter] ((-f|-b|-c) {{sequence}}) {{sourcefile}}+";
static SUMMARY: &str =
    "Prints specified byte or field columns from each line of stdin or the input files";
static LONG_HELP: &str = "
 Each call must specify a mode (what to use for columns),
 a sequence (which columns to print), and provide a data source

 Specifying a mode

    Use --bytes (-b) or --characters (-c) to specify byte mode

    Use --fields (-f) to specify field mode, where each line is broken into
    fields identified by a delimiter character. For example for a typical CSV
    you could use this in combination with setting comma as the delimiter

 Specifying a sequence

    A sequence is a group of 1 or more numbers or inclusive ranges separated
    by a commas.

    cut -f 2,5-7 some_file.txt
    will display the 2nd, 5th, 6th, and 7th field for each source line

    Ranges can extend to the end of the row by excluding the the second number

    cut -f 3- some_file.txt
    will display the 3rd field and all fields after for each source line

    The first number of a range can be excluded, and this is effectively the
    same as using 1 as the first number: it causes the range to begin at the
    first column. Ranges can also display a single column

    cut -f 1,3-5 some_file.txt
    will display the 1st, 3rd, 4th, and 5th field for each source line

    The --complement option, when used, inverts the effect of the sequence

    cut --complement -f 4-6 some_file.txt
    will display the every field but the 4th, 5th, and 6th

 Specifying a data source

    If no sourcefile arguments are specified, stdin is used as the source of
    lines to print

    If sourcefile arguments are specified, stdin is ignored and all files are
    read in consecutively if a sourcefile is not successfully read, a warning
    will print to stderr, and the eventual status code will be 1, but cut
    will continue to read through proceeding sourcefiles

    To print columns from both STDIN and a file argument, use - (dash) as a
    sourcefile argument to represent stdin.

 Field Mode options

    The fields in each line are identified by a delimiter (separator)

    Set the delimiter
        Set the delimiter which separates fields in the file using the
        --delimiter (-d) option. Setting the delimiter is optional.
        If not set, a default delimiter of Tab will be used.

    Optionally Filter based on delimiter
        If the --only-delimited (-s) flag is provided, only lines which
        contain the delimiter will be printed

    Replace the delimiter
        If the --output-delimiter option is provided, the argument used for
        it will replace the delimiter character in each line printed. This is
        useful for transforming tabular data - e.g. to convert a CSV to a
        TSV (tab-separated file)

 Line endings

    When the --zero-terminated (-z) option is used, cut sees \\0 (null) as the
    'line ending' character (both for the purposes of reading lines and
    separating printed lines) instead of \\n (newline). This is useful for
    tabular data where some of the cells may contain newlines

    echo 'ab\\0cd' | cut -z -c 1
    will result in 'a\\0c\\0'
";

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

fn cut_bytes<R: Read>(reader: R, ranges: &[Range], opts: &Options) -> UResult<()> {
    let newline_char = if opts.zero_terminated { b'\0' } else { b'\n' };
    let buf_in = BufReader::new(reader);
    let mut out = stdout_writer();
    let delim = opts
        .out_delim
        .as_ref()
        .map_or("", String::as_str)
        .as_bytes();

    let result = buf_in.for_byte_record(newline_char, |line| {
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

    if let Err(e) = result {
        return Err(USimpleError::new(1, e.to_string()));
    }

    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn cut_fields_delimiter<R: Read>(
    reader: R,
    ranges: &[Range],
    delim: &str,
    only_delimited: bool,
    newline_char: u8,
    out_delim: &str,
) -> UResult<()> {
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

    if let Err(e) = result {
        return Err(USimpleError::new(1, e.to_string()));
    }

    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn cut_fields<R: Read>(reader: R, ranges: &[Range], opts: &FieldOptions) -> UResult<()> {
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

    if let Err(e) = result {
        return Err(USimpleError::new(1, e.to_string()));
    }

    Ok(())
}

fn cut_files(mut filenames: Vec<String>, mode: &Mode) -> UResult<()> {
    let mut stdin_read = false;

    if filenames.is_empty() {
        filenames.push("-".to_owned());
    }

    for filename in &filenames {
        if filename == "-" {
            if stdin_read {
                continue;
            }

            show_if_err!(match mode {
                Mode::Bytes(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Characters(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Fields(ref ranges, ref opts) => cut_fields(stdin(), ranges, opts),
            });

            stdin_read = true;
        } else {
            let path = Path::new(&filename[..]);

            if path.is_dir() {
                show_error!("{}: Is a directory", filename.maybe_quote());
                continue;
            }

            show_if_err!(File::open(&path)
                .map_err_context(|| filename.maybe_quote().to_string())
                .and_then(|file| {
                    match &mode {
                        Mode::Bytes(ranges, opts) | Mode::Characters(ranges, opts) => {
                            cut_bytes(file, ranges, opts)
                        }
                        Mode::Fields(ranges, opts) => cut_fields(file, ranges, opts),
                    }
                }));
        }
    }

    Ok(())
}

mod options {
    pub const BYTES: &str = "bytes";
    pub const CHARACTERS: &str = "characters";
    pub const DELIMITER: &str = "delimiter";
    pub const FIELDS: &str = "fields";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const ONLY_DELIMITED: &str = "only-delimited";
    pub const OUTPUT_DELIMITER: &str = "output-delimiter";
    pub const COMPLEMENT: &str = "complement";
    pub const FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

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
                    Some(mut delim) => {
                        // GNU's `cut` supports `-d=` to set the delimiter to `=`.
                        // Clap parsing is limited in this situation, see:
                        // https://github.com/uutils/coreutils/issues/2424#issuecomment-863825242
                        // Since clap parsing handles `-d=` as delimiter explicitly set to "" and
                        // an empty delimiter is not accepted by GNU's `cut` (and makes no sense),
                        // we can use this as basis for a simple workaround:
                        if delim.is_empty() {
                            delim = "=";
                        }
                        if delim.chars().count() > 1 {
                            Err("invalid input: The '--delimiter' ('-d') option expects empty or 1 character long, but was provided a value 2 characters or longer".into())
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
            "invalid usage: expects no more than one of --fields (-f), --chars (-c) or --bytes (-b)".into()
        ),
        _ => Err("invalid usage: expects one of --fields (-f), --chars (-c) or --bytes (-b)".into()),
    };

    let mode_parse = match mode_parse {
        Err(_) => mode_parse,
        Ok(mode) => match mode {
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.is_present(options::DELIMITER) =>
            {
                Err("invalid input: The '--delimiter' ('-d') option only usable if printing a sequence of fields".into())
            }
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.is_present(options::ONLY_DELIMITED) =>
            {
                Err("invalid input: The '--only-delimited' ('-s') option only usable if printing a sequence of fields".into())
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
        Ok(mode) => cut_files(files, &mode),
        Err(e) => Err(USimpleError::new(1, e)),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .after_help(LONG_HELP)
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long(options::BYTES)
                .takes_value(true)
                .help("filter byte columns from the input source")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .display_order(1),
        )
        .arg(
            Arg::new(options::CHARACTERS)
                .short('c')
                .long(options::CHARACTERS)
                .help("alias for character mode")
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("LIST")
                .display_order(2),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .short('d')
                .long(options::DELIMITER)
                .help("specify the delimiter character that separates fields in the input source. Defaults to Tab.")
                .takes_value(true)
                .value_name("DELIM")
                .display_order(3),
        )
        .arg(
            Arg::new(options::FIELDS)
                .short('f')
                .long(options::FIELDS)
                .help("filter field columns from the input source")
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("LIST")
                .display_order(4),
        )
        .arg(
            Arg::new(options::COMPLEMENT)
                .long(options::COMPLEMENT)
                .help("invert the filter - instead of displaying only the filtered columns, display all but those columns")
                .takes_value(false)
                .display_order(5),
        )
        .arg(
            Arg::new(options::ONLY_DELIMITED)
            .short('s')
                .long(options::ONLY_DELIMITED)
                .help("in field mode, only print lines which contain the delimiter")
                .takes_value(false)
                .display_order(6),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
            .short('z')
                .long(options::ZERO_TERMINATED)
                .help("instead of filtering columns based on line, filter columns based on \\0 (NULL character)")
                .takes_value(false)
                .display_order(8),
        )
        .arg(
            Arg::new(options::OUTPUT_DELIMITER)
            .long(options::OUTPUT_DELIMITER)
                .help("in field mode, replace the delimiter in output lines with this option's argument")
                .takes_value(true)
                .value_name("NEW_DELIM")
                .display_order(7),
        )
        .arg(
            Arg::new(options::FILE)
            .hide(true)
                .multiple_occurrences(true)
        )
}
