#![crate_name = "uu_cut"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Rolf Morel <rolfmorel@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

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

static NAME: &'static str = "cut";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

struct Options {
    out_delim: Option<String>,
}

struct FieldOptions {
    delimiter: String,  // one char long, String because of UTF8 representation
    out_delimeter: Option<String>,
    only_delimited: bool,
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

    let mut buf_read = buffer::ByteReader::new(reader);
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
                        pipe_crash_if_err!(1, out.write_all(&[b'\n']));
                        continue 'newline
                    }
                    Complete(len) => {
                        cur_pos += len;
                        break
                    }
                    Partial(len) => cur_pos += len,
                    EndOfFile => {
                        if orig_pos != cur_pos {
                            pipe_crash_if_err!(1, out.write_all(&[b'\n']));
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
                            pipe_crash_if_err!(1, out.write_all(&[b'\n']));
                        }

                        break 'newline
                    }
                }
            }
        }

        buf_read.consume_line();
        pipe_crash_if_err!(1, out.write_all(&[b'\n']));
    }

    0
}

fn cut_characters<R: Read>(reader: R, ranges: &[Range], opts: &Options) -> i32 {
    let mut buf_in = BufReader::new(reader);
    let mut out = stdout();
    let mut buffer = String::new();

    'newline: loop {
        buffer.clear();
        match buf_in.read_line(&mut buffer) {
            Ok(n) if n == 0 => break,
            Err(e) => {
                if buffer.is_empty() {
                    crash!(1, "read error: {}", e);
                }
            },
            _ => (),
        };

        let line = &buffer[..];
        let mut char_pos = 0;
        let mut char_indices = line.char_indices();
        let mut print_delim = false;
        let mut low_idx = 0;

        for &Range { low, high } in ranges.iter() {
            low_idx = if low - char_pos > 0 {
                match char_indices.nth(low - char_pos - 1) {
                    Some((low_idx, _)) => low_idx,
                    None => break,
                }
            } else {
                low_idx
            };

            match opts.out_delim {
                Some(ref delim) => {
                    if print_delim {
                        pipe_crash_if_err!(1, out.write_all(delim.as_bytes()));
                    }
                    print_delim = true;
                }
                None => ()
            }

            match char_indices.nth(high - low) {
                Some((high_idx, _)) => {
                    let segment = &line.as_bytes()[low_idx..high_idx];
                    low_idx = high_idx;

                    pipe_crash_if_err!(1, out.write_all(segment));
                }
                None => {
                    let bytes = line.as_bytes();
                    let segment = &bytes[low_idx..];

                    pipe_crash_if_err!(1, out.write_all(segment));

                    if line.as_bytes()[bytes.len() - 1] == b'\n' {
                        continue 'newline
                    }
                }
            }

            char_pos = high + 1;
        }
        pipe_crash_if_err!(1, out.write_all(&[b'\n']));
    }

    0
}

fn cut_fields_delimiter<R: Read>(reader: R, ranges: &[Range], delim: &str, only_delimited: bool, out_delim: &str) -> i32 {
    let mut buf_in = BufReader::new(reader);
    let mut out = stdout();
    let mut buffer = Vec::new();

    'newline: loop {
        buffer.clear();
        match buf_in.read_until(b'\n', &mut buffer) {
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
                if line[line.len() - 1] != b'\n' {
                    pipe_crash_if_err!(1, out.write_all(&[b'\n']));
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

                        if line[line.len() - 1] == b'\n' {
                            continue 'newline
                        }
                        break
                    }
                }
            }
        }

        pipe_crash_if_err!(1, out.write_all(&[b'\n']));
    }

    0
}

fn cut_fields<R: Read>(reader: R, ranges: &[Range], opts: &FieldOptions) -> i32 {
    match opts.out_delimeter {
        Some(ref o_delim) => {
            return cut_fields_delimiter(reader, ranges, &opts.delimiter,
                                        opts.only_delimited, o_delim);
        }
        None => ()
    }

    let mut buf_in = BufReader::new(reader);
    let mut out = stdout();
    let mut buffer = Vec::new();

    'newline: loop {
        buffer.clear();
        match buf_in.read_until(b'\n', &mut buffer) {
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
                if line[line.len() - 1] != b'\n' {
                    pipe_crash_if_err!(1, out.write_all(&[b'\n']));
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

                    if line[line.len() - 1] == b'\n' {
                        continue 'newline
                    }
                    break
                }
            }
        }

        pipe_crash_if_err!(1, out.write_all(&[b'\n']));
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
                Mode::Characters(ref ranges, ref opts) => cut_characters(stdin(), ranges, opts),
                Mode::Fields(ref ranges, ref opts) => cut_fields(stdin(), ranges, opts),
            };

            stdin_read = true;
        } else {
            let path = Path::new(&filename[..]);

            if !path.exists() {
                show_error!("{}: No such file or directory", filename);
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
                Mode::Characters(ref ranges, ref opts) => cut_characters(file, ranges, opts),
                Mode::Fields(ref ranges, ref opts) => cut_fields(file, ranges, opts),
            };
        }
    }

    exit_code
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("b", "bytes", "select only these bytes", "LIST");
    opts.optopt("c", "characters", "select only these characters", "LIST");
    opts.optopt("d", "delimiter", "use DELIM instead of TAB for field delimiter", "DELIM");
    opts.optopt("f", "fields", "select only these fields;  also print any line that contains no delimiter character, unless the -s option is specified", "LIST");
    opts.optflag("n", "", "(ignored)");
    opts.optflag("", "complement", "complement the set of selected bytes, characters or fields");
    opts.optflag("s", "only-delimited", "do not print lines not containing delimiters");
    opts.optopt("", "output-delimiter", "use STRING as the output delimiter the default is to use the input delimiter", "STRING");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("Invalid options\n{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} OPTION... [FILE]...", NAME);
        println!("");
        println!("{}", opts.usage("Print selected parts of lines from each FILE to standard output."));
        println!("");
        println!("Use one, and only one of -b, -c or -f.  Each LIST is made up of one");
        println!("range, or many ranges separated by commas.  Selected input is written");
        println!("in the same order that it is read, and is written exactly once.");
        println!("Each range is one of:");
        println!("");
        println!("  N     N'th byte, character or field, counted from 1");
        println!("  N-    from N'th byte, character or field, to end of line");
        println!("  N-M   from N'th to M'th (included) byte, character or field");
        println!("  -M    from first to M'th (included) byte, character or field");
        println!("");
        println!("With no FILE, or when FILE is -, read standard input.");
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let complement = matches.opt_present("complement");

    let mode_parse = match (matches.opt_str("bytes"),
                            matches.opt_str("characters"),
                            matches.opt_str("fields")) {
        (Some(byte_ranges), None, None) => {
            list_to_ranges(&byte_ranges[..], complement)
                .map(|ranges| Mode::Bytes(ranges, Options { out_delim: matches.opt_str("output-delimiter") }))
        }
        (None, Some(char_ranges), None) => {
            list_to_ranges(&char_ranges[..], complement)
                .map(|ranges| Mode::Characters(ranges, Options { out_delim: matches.opt_str("output-delimiter") }))
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

                    match matches.opt_str("delimiter") {
                        Some(delim) => {
                            if delim.chars().count() > 1 {
                                Err("the delimiter must be a single character, or the empty string for null".to_owned())
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
                                              only_delimited: only_delimited
                                          }))
                            }
                        }
                        None => Ok(Mode::Fields(ranges,
                                          FieldOptions {
                                              delimiter: "\t".to_owned(),
                                              out_delimeter: out_delim,
                                              only_delimited: only_delimited
                                          }))
                    }
                }
            )
        }
        (ref b, ref c, ref f) if b.is_some() || c.is_some() || f.is_some() => {
            Err("only one type of list may be specified".to_owned())
        }
        _ => Err("you must specify a list of bytes, characters, or fields".to_owned())
    };

    let mode_parse = match mode_parse {
        Err(_) => mode_parse,
        Ok(mode) => {
            match mode {
                Mode::Bytes(_, _) | Mode::Characters(_, _) if matches.opt_present("delimiter") =>
                    Err("an input delimiter may be specified only when operating on fields".to_owned()),
                Mode::Bytes(_, _) | Mode::Characters(_, _) if matches.opt_present("only-delimited") =>
                    Err("suppressing non-delimited lines makes sense only when operating on fields".to_owned()),
                _ => Ok(mode),
            }
        }
    };

    match mode_parse {
        Ok(mode) => cut_files(matches.free, mode),
        Err(err_msg) => {
            show_error!("{}\n\
                         Try '{} --help' for more information",
                        err_msg, args[0]);
            1
        }
    }
}
