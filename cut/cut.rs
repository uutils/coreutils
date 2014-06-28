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

use std::io::{File, BufferedWriter, BufferedReader, stdin, print};
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
            Err(std::io::IoError { kind: std::io::EndOfFile, .. }) => break,
            _ => fail!(),
        };

        let line_len = line.len();
        let mut print_delim = false;

        for &Range { low: low, high: high } in ranges.iter() {
            if low > line_len { break; }

            if use_delim {
                if print_delim {
                    out.write_str(out_delim.as_slice()).unwrap();
                }
                print_delim = true;
            }

            if high >= line_len {
                let segment = line.slice(low - 1, line_len);

                out.write(segment).unwrap();

                if *line.get(line_len - 1) == b'\n' {
                    continue 'newline
                }
            } else {
                let segment = line.slice(low - 1, high);

                out.write(segment).unwrap();
            }
        }

        out.write(&[b'\n']).unwrap();
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
            Err(std::io::IoError { kind: std::io::EndOfFile, .. }) => break,
            _ => fail!(),
        };

        let mut char_pos = 0;
        let mut char_indices = line.as_slice().char_indices();
        let mut print_delim = false;

        for &Range { low: low, high: high } in ranges.iter() {
            let low_idx = match char_indices.nth(low - char_pos - 1) {
                Some((low_idx, _)) => low_idx,
                None => break
            };

            if use_delim {
                if print_delim {
                    out.write_str(out_delim.as_slice()).unwrap();
                }
                print_delim = true;
            }

            match char_indices.nth(high - low) {
                Some((high_idx, _)) => {
                    let segment = line.as_bytes().slice(low_idx, high_idx);

                    out.write(segment).unwrap();
                }
                None => {
                    let bytes = line.as_bytes();
                    let segment = bytes.slice(low_idx, bytes.len());

                    out.write(segment).unwrap();

                    if line.as_bytes()[bytes.len() - 1] == b'\n' {
                        continue 'newline
                    }
                }
            }

            char_pos = high + 1;
        }
        out.write(&[b'\n']).unwrap();
    }

    0
}

#[deriving(Clone)]
struct Searcher<'a> {
    haystack: &'a [u8],
    needle: &'a [u8],
    position: uint
}

impl<'a> Searcher<'a> {
    fn new(haystack: &'a [u8], needle: &'a [u8]) -> Searcher<'a> {
        Searcher {
            haystack: haystack,
            needle: needle,
            position: 0
        }
    }
}

impl<'a> Iterator<(uint, uint)> for Searcher<'a> {
    fn next(&mut self) -> Option<(uint, uint)> {
        if self.needle.len() == 1 {
            for offset in range(self.position, self.haystack.len()) {
                if self.haystack[offset] == self.needle[0] {
                    self.position = offset + 1;
                    return Some((offset, offset + 1));
                }
            }

            self.position = self.haystack.len();
            return None;
        }

        while self.position + self.needle.len() <= self.haystack.len() {
            if self.haystack.slice(self.position,
                                   self.position + self.needle.len()) == self.needle {
                let match_pos = self.position;
                self.position += self.needle.len();
                return Some((match_pos, match_pos + self.needle.len()));
            } else {
                self.position += 1;
            }
        }
        None
    }
}

fn cut_fields_delimiter<T: Reader>(mut reader: BufferedReader<T>,
                                   ranges: &Vec<Range>,
                                   delim: &String,
                                   only_delimited: bool,
                                   out_delim: &String) -> int {
    let mut out = BufferedWriter::new(std::io::stdio::stdout_raw());

    'newline: loop {
        let line = match reader.read_until(b'\n') {
            Ok(line) => line,
            Err(std::io::IoError { kind: std::io::EndOfFile, .. }) => break,
            _ => fail!(),
        };

        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(line.as_slice(),
                                             delim.as_bytes()).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if ! only_delimited {
                out.write(line.as_slice()).unwrap();
                if *line.get(line.len() - 1) != b'\n' {
                    out.write([b'\n']).unwrap();
                }
            }

            continue
        }

        for &Range { low: low, high: high } in ranges.iter() {
            if low - fields_pos > 0 {
                low_idx = match delim_search.nth(low - fields_pos - 1) {
                    Some((_, beyond_delim)) => beyond_delim,
                    None => break
                };
            }

            for _ in range(0, high - low + 1) {
                if print_delim {
                    out.write_str(out_delim.as_slice()).unwrap();
                }

                match delim_search.next() {
                    Some((high_idx, next_low_idx)) => {
                        let segment = line.slice(low_idx, high_idx);

                        out.write(segment).unwrap();

                        print_delim = true;

                        low_idx = next_low_idx;
                        fields_pos = high + 1;
                    }
                    None => {
                        let segment = line.slice(low_idx, line.len());

                        out.write(segment).unwrap();

                        if *line.get(line.len() - 1) == b'\n' {
                            continue 'newline
                        }
                        break
                    }
                }
            }
        }

        out.write(&[b'\n']).unwrap();
    }

    0
}

fn cut_fields<T: Reader>(mut reader: BufferedReader<T>,
                         ranges: &Vec<Range>,
                         opts: &FieldOptions) -> int {
    match opts.out_delimeter {
        Some(ref delim) => {
            return cut_fields_delimiter(reader, ranges, &opts.delimiter,
                                        opts.only_delimited, delim);
        }
        None => ()
    }

    let mut out = BufferedWriter::new(std::io::stdio::stdout_raw());

    'newline: loop {
        let line = match reader.read_until(b'\n') {
            Ok(line) => line,
            Err(std::io::IoError { kind: std::io::EndOfFile, .. }) => break,
            _ => fail!(),
        };

        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(line.as_slice(),
                                             opts.delimiter.as_bytes()).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if ! opts.only_delimited {
                out.write(line.as_slice()).unwrap();
                if *line.get(line.len() - 1) != b'\n' {
                    out.write([b'\n']).unwrap();
                }
            }

            continue
        }

        for &Range { low: low, high: high } in ranges.iter() {
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
                    let segment = line.slice(low_idx, high_idx);

                    out.write(segment).unwrap();

                    print_delim = true;
                    low_idx = next_low_idx;
                    fields_pos = high + 1;
                }
                None => {
                    let segment = line.slice(low_idx, line.len());

                    out.write(segment).unwrap();

                    if *line.get(line.len() - 1) == b'\n' {
                        continue 'newline
                    }
                    break
                }
            }
        }

        out.write(&[b'\n']).unwrap();
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
            show_error!("Invalid options\n{}", f)
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
                      Options { out_delim: matches.opt_str("output-delimiter") })
            )
        }
        (None, Some(char_ranges), None) => {
            list_to_ranges(char_ranges.as_slice(), complement).map(|ranges|
                Characters(ranges,
                           Options { out_delim: matches.opt_str("output-delimiter") })
            )
        }
        (None, None, Some(field_ranges)) => {
            list_to_ranges(field_ranges.as_slice(), complement).and_then(|ranges|
                {
                    let out_delim = matches.opt_str("output-delimiter");
                    let only_delimited = matches.opt_present("only-delimited");

                    match matches.opt_str("delimiter") {
                        Some(delim) => {
                            if delim.as_slice().char_len() != 1 {
                                Err("the delimiter must be a single character".to_str())
                            } else {
                                Ok(Fields(ranges,
                                          FieldOptions {
                                              delimiter: delim,
                                              out_delimeter: out_delim,
                                              only_delimited: only_delimited
                                          }))
                            }
                        }
                        None => Ok(Fields(ranges,
                                          FieldOptions {
                                              delimiter: "\t".to_str(),
                                              out_delimeter: out_delim,
                                              only_delimited: only_delimited
                                          }))
                    }
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
            show_error!("{}\n\
                         Try '{} --help' for more information",
                        err_msg, args.get(0));
            1
        }
    }
}
