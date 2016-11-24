#![crate_name = "uu_cut"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Rolf Morel <rolfmorel@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{stdout, stdin, BufRead, BufReader, Read, Stdout, Write};
use std::path::Path;

use ranges::Range;
use searcher::Searcher;

mod buffer;
mod ranges;
mod searcher;

static SYNTAX: &'static str = "[-d] [-s] [-z] [--output-delimiter] ((-f|-b|-c) {{sequence}}) {{sourcefile}}+";
static SUMMARY: &'static str = "Prints specified byte or field columns from each line of stdin or the input files";
static LONG_HELP: &'static str = "
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
    delimiter: String,  // one char long, String because of UTF8 representation
    out_delimeter: Option<String>,
    only_delimited: bool,
    zero_terminated: bool,
}

enum Mode {
    Bytes(Vec<Range>, Options),
    Characters(Vec<Range>, Options),
    Fields(Vec<Range>, FieldOptions),
}

fn list_to_ranges(list: &str, complement: bool) -> Result<Vec<Range>, String> {
    if complement {
        Range::from_list(list).map(|r| ranges::complement(&r))
    } else {
        Range::from_list(list)
    }
}

fn cut_bytes<R: Read>(reader: R, ranges: &[Range], opts: &Options) -> i32 {
    use buffer::Bytes::Select;
    use buffer::Bytes::Selected::*;

    let newline_char =
        if opts.zero_terminated { b'\0' } else { b'\n' };
    let mut buf_read = buffer::ByteReader::new(reader, newline_char);
    let mut out = stdout();

    'newline: loop {
        let mut cur_pos = 1;
        let mut print_delim = false;

        for &Range { low, high } in ranges.iter() {
            // skip upto low
            let orig_pos = cur_pos;
            loop {
                match buf_read.select(low - cur_pos, None::<&mut Stdout>) {
                    NewlineFound => {
                        pipe_crash_if_err!(1, out.write_all(&[newline_char]));
                        continue 'newline
                    }
                    Complete(len) => {
                        cur_pos += len;
                        break
                    }
                    Partial(len) => cur_pos += len,
                    EndOfFile => {
                        if orig_pos != cur_pos {
                            pipe_crash_if_err!(1, out.write_all(&[newline_char]));
                        }

                        break 'newline
                    }
                }
            }

            match opts.out_delim {
                Some(ref delim) => {
                    if print_delim {
                        pipe_crash_if_err!(1, out.write_all(delim.as_bytes()));
                    }
                    print_delim = true;
                }
                None => ()
            }

            // write out from low to high
            loop {
                match buf_read.select(high - cur_pos + 1, Some(&mut out)) {
                    NewlineFound => continue 'newline,
                    Partial(len) => cur_pos += len,
                    Complete(_) => {
                        cur_pos = high + 1;
                        break
                    }
                    EndOfFile => {
                        if cur_pos != low || low == high {
                            pipe_crash_if_err!(1, out.write_all(&[newline_char]));
                        }

                        break 'newline
                    }
                }
            }
        }

        buf_read.consume_line();
        pipe_crash_if_err!(1, out.write_all(&[newline_char]));
    }

    0
}

fn cut_fields_delimiter<R: Read>(reader: R, ranges: &[Range], delim: &str, only_delimited: bool, newline_char: u8, out_delim: &str) -> i32 {
    let mut buf_in = BufReader::new(reader);
    let mut out = stdout();
    let mut buffer = Vec::new();

    'newline: loop {
        buffer.clear();
        match buf_in.read_until(newline_char, &mut buffer) {
            Ok(n) if n == 0 => break,
            Err(e) => {
                if buffer.is_empty() {
                    crash!(1, "read error: {}", e);
                }
            },
            _ => (),
        }

        let line = &buffer[..];
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(line, delim.as_bytes()).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if ! only_delimited {
                pipe_crash_if_err!(1, out.write_all(line));
                if line[line.len() - 1] != newline_char {
                    pipe_crash_if_err!(1, out.write_all(&[newline_char]));
                }
            }

            continue
        }

        for &Range { low, high } in ranges.iter() {
            if low - fields_pos > 0 {
                low_idx = match delim_search.nth(low - fields_pos - 1) {
                    Some((_, beyond_delim)) => beyond_delim,
                    None => break
                };
            }

            for _ in 0..high - low + 1 {
                if print_delim {
                    pipe_crash_if_err!(1, out.write_all(out_delim.as_bytes()));
                }

                match delim_search.next() {
                    Some((high_idx, next_low_idx)) => {
                        let segment = &line[low_idx..high_idx];

                        pipe_crash_if_err!(1, out.write_all(segment));

                        print_delim = true;

                        low_idx = next_low_idx;
                        fields_pos = high + 1;
                    }
                    None => {
                        let segment = &line[low_idx..];

                        pipe_crash_if_err!(1, out.write_all(segment));

                        if line[line.len() - 1] == newline_char {
                            continue 'newline
                        }
                        break
                    }
                }
            }
        }

        pipe_crash_if_err!(1, out.write_all(&[newline_char]));
    }

    0
}

fn cut_fields<R: Read>(reader: R, ranges: &[Range], opts: &FieldOptions) -> i32 {
    let newline_char =
        if opts.zero_terminated { b'\0' } else { b'\n' };
    match opts.out_delimeter {
        Some(ref o_delim) => {
            return cut_fields_delimiter(reader, ranges, &opts.delimiter,
                                            opts.only_delimited, newline_char, o_delim)
        }
        None => ()
    }

    let mut buf_in = BufReader::new(reader);
    let mut out = stdout();
    let mut buffer = Vec::new();

    'newline: loop {
        buffer.clear();
        match buf_in.read_until(newline_char, &mut buffer) {
            Ok(n) if n == 0 => break,
            Err(e) => {
                if buffer.is_empty() {
                    crash!(1, "read error: {}", e);
                }
            },
            _ => (),
        }

        let line = &buffer[..];
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(line, opts.delimiter.as_bytes()).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if ! opts.only_delimited {
                pipe_crash_if_err!(1, out.write_all(line));
                if line[line.len() - 1] != newline_char {
                    pipe_crash_if_err!(1, out.write_all(&[newline_char]));
                }
            }

            continue
        }

        for &Range { low, high } in ranges.iter() {
            if low - fields_pos > 0 {
                low_idx = match delim_search.nth(low - fields_pos - 1) {
                    Some((_, beyond_delim)) => beyond_delim,
                    None => break
                };
            }

            if print_delim {
                if low_idx >= opts.delimiter.as_bytes().len() {
                    low_idx -= opts.delimiter.as_bytes().len();
                }
            }

            match delim_search.nth(high - low) {
                Some((high_idx, next_low_idx)) => {
                    let segment = &line[low_idx..high_idx];

                    pipe_crash_if_err!(1, out.write_all(segment));

                    print_delim = true;
                    low_idx = next_low_idx;
                    fields_pos = high + 1;
                }
                None => {
                    let segment = &line[low_idx..line.len()];

                    pipe_crash_if_err!(1, out.write_all(segment));

                    if line[line.len() - 1] == newline_char {
                        continue 'newline
                    }
                    break
                }
            }
        }

        pipe_crash_if_err!(1, out.write_all(&[newline_char]));
    }

    0
}

fn cut_files(mut filenames: Vec<String>, mode: Mode) -> i32 {
    let mut stdin_read = false;
    let mut exit_code = 0;

    if filenames.is_empty() { filenames.push("-".to_owned()); }

    for filename in &filenames {
        if filename == "-" {
            if stdin_read { continue }

            exit_code |= match mode {
                Mode::Bytes(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Characters(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Fields(ref ranges, ref opts) => cut_fields(stdin(), ranges, opts),
            };

            stdin_read = true;
        } else {
            let path = Path::new(&filename[..]);

            if !path.exists() {
                show_error!("{}", msg_args_nonexistent_file!(filename));
                continue
            }

            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    show_error!("opening '{}': {}", &filename[..], e);
                    continue
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

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optopt("b", "bytes", "filter byte columns from the input source", "sequence")
        .optopt("c", "characters", "alias for character mode", "sequence")
        .optopt("d", "delimiter", "specify the delimiter character that separates fields in the input source. Defaults to Tab.", "delimiter")
        .optopt("f", "fields", "filter field columns from the input source", "sequence")
        .optflag("n", "", "legacy option - has no effect.")
        .optflag("", "complement", "invert the filter - instead of displaying only the filtered columns, display all but those columns")
        .optflag("s", "only-delimited", "in field mode, only print lines which contain the delimiter")
        .optflag("z", "zero-terminated", "instead of filtering columns based on line, filter columns based on \\0 (NULL character)")
        .optopt("", "output-delimiter", "in field mode, replace the delimiter in output lines with this option's argument", "new delimiter")
        .parse(args);
    let complement = matches.opt_present("complement");

    let mode_parse = match (matches.opt_str("bytes"),
                            matches.opt_str("characters"),
                            matches.opt_str("fields")) {
        (Some(byte_ranges), None, None) => {
            list_to_ranges(&byte_ranges[..], complement)
                .map(|ranges| Mode::Bytes(ranges, Options { out_delim: matches.opt_str("output-delimiter"), zero_terminated : matches.opt_present("zero-terminated") }))
        }
        (None, Some(char_ranges), None) => {
            list_to_ranges(&char_ranges[..], complement)
                .map(|ranges| Mode::Characters(ranges, Options { out_delim: matches.opt_str("output-delimiter"), zero_terminated : matches.opt_present("zero-terminated") }))
        }
        (None, None, Some(field_ranges)) => {
            list_to_ranges(&field_ranges[..], complement).and_then(|ranges|
                {
                    let out_delim = match matches.opt_str("output-delimiter") {
                        Some(s) => {
                            if s.is_empty() {
                                Some("\0".to_owned())
                            } else {
                                Some(s)
                            }
                        },
                        None => None,
                    };

                    let only_delimited = matches.opt_present("only-delimited");
                    let zero_terminated = matches.opt_present("zero-terminated");

                    match matches.opt_str("delimiter") {
                        Some(delim) => {
                            if delim.chars().count() > 1 {
                                Err(msg_opt_invalid_should_be!("empty or 1 character long", "a value 2 characters or longer", "--delimiter", "-d").to_owned())
                            } else {
                                let delim = if delim.is_empty() {
                                    "\0".to_owned()
                                } else {
                                    delim
                                };

                                Ok(Mode::Fields(ranges,
                                          FieldOptions {
                                              delimiter: delim,
                                              out_delimeter: out_delim,
                                              only_delimited: only_delimited,
                                              zero_terminated: zero_terminated
                                          }))
                            }
                        }
                        None => Ok(Mode::Fields(ranges,
                                          FieldOptions {
                                              delimiter: "\t".to_owned(),
                                              out_delimeter: out_delim,
                                              only_delimited: only_delimited,
                                              zero_terminated: zero_terminated
                                          }))
                    }
                }
            )
        }
        (ref b, ref c, ref f) if b.is_some() || c.is_some() || f.is_some() => {
            Err(msg_expects_no_more_than_one_of!("--fields (-f)", "--chars (-c)", "--bytes (-b)").to_owned())
        }
        _ => Err(msg_expects_one_of!("--fields (-f)", "--chars (-c)", "--bytes (-b)").to_owned())
    };

    let mode_parse = match mode_parse {
        Err(_) => mode_parse,
        Ok(mode) => {
            match mode {
                Mode::Bytes(_, _) | Mode::Characters(_, _) if matches.opt_present("delimiter") =>
                    Err(msg_opt_only_usable_if!("printing a sequence of fields", "--delimiter", "-d").to_owned()),
                Mode::Bytes(_, _) | Mode::Characters(_, _) if matches.opt_present("only-delimited") =>
                    Err(msg_opt_only_usable_if!("printing a sequence of fields", "--only-delimited", "-s").to_owned()),
                _ => Ok(mode),
            }
        }
    };

    match mode_parse {
        Ok(mode) => cut_files(matches.free, mode),
        Err(err_msg) => {
            show_error!("{}", err_msg);
            1
        }
    }
}
