#![crate_id(name="cut", vers="1.0.0", author="Rolf Morel")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Rolf Morel <rolfmorel@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::os;
use std::io::{print,File,BufferedWriter,BufferedReader,stdin};
use getopts::{optopt, optflag, getopts, usage};

use ranges::Range;

#[path = "../common/util.rs"]
mod util;
mod ranges;

static NAME: &'static str = "cut";
static VERSION: &'static str = "1.0.0";

struct Options {
    out_delim: Option<String>,
}

struct FieldOptions {
    delimiter: char,
    out_delimeter: String,
    only_delimited: bool,
}

enum Mode {
    Bytes(Vec<Range>, Options),
    Characters(Vec<Range>, Options),
    Fields(Vec<Range>, FieldOptions),
}

fn list_to_ranges(list: &str, complement: bool) -> Result<Vec<Range>, String> {
    use std::uint;

    let mut range_vec = {
        try!(
            if complement {
                Range::from_list(list).map(|r| ranges::complement(&r))
            } else {
                Range::from_list(list)
            }
        )
    };

    // add sentinel value for increased performance during cutting
    range_vec.push(Range{ low: uint::MAX, high: uint::MAX });

    Ok(range_vec)
}

fn cut_bytes<T: Reader>(mut reader: BufferedReader<T>,
                        ranges: &Vec<Range>,
                        opts: &Options) -> int {
    let mut out = BufferedWriter::new(std::io::stdio::stdout_raw());
    let (use_delim, out_delim) = match opts.out_delim.clone() {
        Some(delim) => (true, delim),
        None => (false, "".to_str())
    };

    'newline: loop {
        let line = match reader.read_until(b'\n') {
            Ok(line) => line,
            Err(std::io::IoError{ kind: std::io::EndOfFile, ..}) => break,
            _ => fail!(),
        };

        let line_len = line.len();
        let mut print_delim = false;

        for &Range{ low: low, high: high } in ranges.iter() {
            if low > line_len { break; }

            if use_delim {
                if print_delim {
                    out.write_str(out_delim.as_slice());
                }
                print_delim = true;
            }

            if high >= line_len {
                let segment = line.slice(low - 1, line_len);

                out.write(segment);

                if *line.get(line_len - 1) == b'\n' {
                    continue 'newline
                }
            } else {
                let segment = line.slice(low - 1, high);

                out.write(segment);
            }
        }

        out.write(&[b'\n']);
    }

    0
}

fn cut_characters<T: Reader>(mut reader: BufferedReader<T>,
                             ranges: &Vec<Range>,
                             opts: &Options) -> int {
    let mut out = BufferedWriter::new(std::io::stdio::stdout_raw());
    let (use_delim, out_delim) = match opts.out_delim.clone() {
        Some(delim) => (true, delim),
        None => (false, "".to_str())
    };

    'newline: loop {
        let line = match reader.read_line() {
            Ok(line) => line,
            Err(std::io::IoError{ kind: std::io::EndOfFile, ..}) => break,
            _ => fail!(),
        };

        let mut char_pos = 0;
        let mut char_indices = line.as_slice().char_indices();
        let mut print_delim = false;

        for &Range{ low: low, high: high } in ranges.iter() {
            let low_idx = match char_indices.nth(low - char_pos - 1) {
                Some((low_idx, _)) => low_idx,
                None => break
            };

            if use_delim {
                if print_delim {
                    out.write_str(out_delim.as_slice());
                }
                print_delim = true;
            }

            match char_indices.nth(high - low) {
                Some((high_idx, _)) => {
                    let segment = line.as_bytes().slice(low_idx, high_idx);

                    out.write(segment);
                }
                None => {
                    let bytes = line.as_bytes();
                    let segment = bytes.slice(low_idx, bytes.len());

                    out.write(segment);

                    if line.as_bytes()[bytes.len() - 1] == b'\n' {
                        continue 'newline
                    }
                }
            }

            char_pos = high + 1;
        }
        out.write(&[b'\n']);
    }

    0
}

fn cut_fields<T: Reader>(reader: BufferedReader<T>,
                         ranges: &Vec<Range>,
                         opts: &FieldOptions) -> int {
    for range in ranges.iter() {
        println!("{}-{}", range.low, range.high);
    }

    0
}

fn cut_files(mut filenames: Vec<String>, mode: Mode) -> int {
    let mut stdin_read = false;
    let mut exit_code = 0;

    if filenames.len() == 0 { filenames.push("-".to_str()); }

    for filename in filenames.iter() {
        if filename.as_slice() == "-" {
            if stdin_read { continue; }

            exit_code |= match mode {
                Bytes(ref ranges, ref opts) => {
                    cut_bytes(stdin(), ranges, opts)
                }
                Characters(ref ranges, ref opts) => {
                    cut_characters(stdin(), ranges, opts)
                }
                Fields(ref ranges, ref opts) => {
                    cut_fields(stdin(), ranges, opts)
                }
            };

            stdin_read = true;
        } else {
            let path = Path::new(filename.as_slice());

            if ! path.exists() {
                show_error!("{}: No such file or directory", filename);
                continue;
            }

            let buf_file = match File::open(&path) {
                Ok(file) => BufferedReader::new(file),
                Err(e) => {
                    show_error!("{}: {}", filename, e.desc);
                    continue
                }
            };

            exit_code |= match mode {
                Bytes(ref ranges, ref opts) => cut_bytes(buf_file, ranges, opts),
                Characters(ref ranges, ref opts) => {
                    cut_characters(buf_file, ranges, opts)
                }
                Fields(ref ranges, ref opts) => {
                    cut_fields(buf_file, ranges, opts)
                }
            };
        }
    }

    exit_code
}

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        optopt("b", "bytes", "select only these bytes", "LIST"),
        optopt("c", "characters", "select only these characters", "LIST"),
        optopt("d", "delimiter", "use DELIM instead of TAB for field delimiter", "DELIM"),
        optopt("f", "fields", "select only these fields;  also print any line that contains no delimiter character, unless the -s option is specified", "LIST"),
        optflag("n", "", "(ignored)"),
        optflag("", "complement", "complement the set of selected bytes, characters or fields"),
        optflag("s", "only-delimited", "do not print lines not containing delimiters"),
        optopt("", "output-delimiter", "use STRING as the output delimiter the default is to use the input delimiter", "STRING"),
        optflag("", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit"),
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            show_error!("Invalid options\n{}", f.to_err_msg())
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("Usage:");
        println!("  {0} OPTION... [FILE]...", args.get(0));
        println!("");
        print(usage("Print selected parts of lines from each FILE to standard output.", opts).as_slice());
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
            list_to_ranges(byte_ranges.as_slice(), complement).map(|ranges|
                Bytes(ranges,
                      Options{ out_delim: matches.opt_str("output-delimiter") })
            )
        }
        (None ,Some(char_ranges), None) => {
            list_to_ranges(char_ranges.as_slice(), complement).map(|ranges|
                Characters(ranges,
                           Options{ out_delim: matches.opt_str("output-delimiter") })
            )
        }
        (None, None ,Some(field_ranges)) => {
            list_to_ranges(field_ranges.as_slice(), complement).map(|ranges|
                {
                    use std::str::from_char;

                    let delim = matches.opt_str("delimiter")
                                       .filtered(|s| s.len() == 1)
                                       .map(|s| s.as_slice().char_at(0))
                                       .unwrap_or('\t');
                    let out_delim = matches.opt_str("output-delimiter")
                                           .unwrap_or(from_char(delim));
                    let only_delimited = matches.opt_present("only-delimited");

                    Fields(ranges,
                           FieldOptions{ delimiter: delim,
                                         out_delimeter: out_delim,
                                         only_delimited: only_delimited })
                }
            )
        }
        (ref b, ref c, ref f) if b.is_some() || c.is_some() || f.is_some() => {
            Err("only one type of list may be specified".to_str())
        }
        _ => Err("you must specify a list of bytes, characters, or fields".to_str())
    };

    match mode_parse {
        Ok(mode) => cut_files(matches.free, mode),
        Err(err_msg) => {
            show_error!("{}", err_msg);
            1
        }
    }
}
