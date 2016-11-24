#![crate_name = "uu_expand"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Virgile Andreani <virgile.andreani@anbuco.fr>
 * (c) kwantam <kwantam@gmail.com>
 *     20150428 updated to work with both UTF-8 and non-UTF-8 encodings
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate unicode_width;
extern crate getopts;

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::iter::repeat;
use std::str::from_utf8;
use unicode_width::UnicodeWidthChar;

static SYNTAX: &'static str = "[OPTION]... [FILE]..."; 
static SUMMARY: &'static str = "Convert tabs in each FILE to spaces, writing to standard output.
 With no FILE, or when FILE is -, read standard input."; 
static LONG_HELP: &'static str = "";

static DEFAULT_TABSTOP: usize = 8;

fn tabstops_parse(s: String) -> Vec<usize> {
    let words = s.split(',').collect::<Vec<&str>>();

    let nums = words.into_iter()
        .map(|sn| sn.parse::<usize>()
            .unwrap_or_else(
                |_| crash!(1, "{}\n", "tab size contains invalid character(s)"))
            )
        .collect::<Vec<usize>>();

    if nums.iter().any(|&n| n == 0) {
        crash!(1, "{}\n", "tab size cannot be 0");
    }

    if let (false, _) = nums.iter().fold((true, 0), |(acc, last), &n| (acc && last <= n, n)) {
        crash!(1, "{}\n", "tab sizes must be ascending");
    }

    nums
}

struct Options {
    files: Vec<String>,
    tabstops: Vec<usize>,
    tspaces: String,
    iflag: bool,
    uflag: bool,
}

impl Options {
    fn new(matches: getopts::Matches) -> Options {
        let tabstops = match matches.opt_str("t") {
            None => vec!(DEFAULT_TABSTOP),
            Some(s) => tabstops_parse(s)
        };

        let iflag = matches.opt_present("i");
        let uflag = !matches.opt_present("U");

        // avoid allocations when dumping out long sequences of spaces
        // by precomputing the longest string of spaces we will ever need
        let nspaces = tabstops.iter().scan(0, |pr,&it| {
            let ret = Some(it - *pr);
            *pr = it;
            ret
        }).max().unwrap();  // length of tabstops is guaranteed >= 1
        let tspaces = repeat(' ').take(nspaces).collect();

        let files =
            if matches.free.is_empty() {
                vec!("-".to_owned())
            } else {
                matches.free
            };

        Options { files: files, tabstops: tabstops, tspaces: tspaces, iflag: iflag, uflag: uflag }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("i", "initial", "do not convert tabs after non blanks")
        .optopt("t", "tabs", "have tabs NUMBER characters apart, not 8", "NUMBER")
        .optopt("t", "tabs", "use comma separated list of explicit tab positions", "LIST")
        .optflag("U", "no-utf8", "interpret input file as 8-bit ASCII rather than UTF-8")
        .parse(args);

    expand(Options::new(matches));

    0
}

fn open(path: String) -> BufReader<Box<Read+'static>> {
    let file_buf;
    if path == "-" {
        BufReader::new(Box::new(stdin()) as Box<Read>)
    } else {
        file_buf = match File::open(&path[..]) {
            Ok(a) => a,
            Err(e) => crash!(1, "{}: {}\n", &path[..], e),
        };
        BufReader::new(Box::new(file_buf) as Box<Read>)
    }
}

fn next_tabstop(tabstops: &[usize], col: usize) -> usize {
    if tabstops.len() == 1 {
        tabstops[0] - col % tabstops[0]
    } else {
        match tabstops.iter().skip_while(|&&t| t <= col).next() {
            Some(t) => t - col,
            None => 1,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum CharType {
    Backspace,
    Tab,
    Other,
}

fn expand(options: Options) {
    use self::CharType::*;

    let mut output = BufWriter::new(stdout());
    let ts = options.tabstops.as_ref();
    let mut buf = Vec::new();

    for file in options.files.into_iter() {
        let mut fh = open(file);

        while match fh.read_until('\n' as u8, &mut buf) {
            Ok(s) => s > 0,
            Err(_) => buf.is_empty(),
        } {
            let mut col = 0;
            let mut byte = 0;
            let mut init = true;

            while byte < buf.len() {
                let (ctype, cwidth, nbytes) = if options.uflag {
                    let nbytes = uucore::utf8::utf8_char_width(buf[byte]);

                    if byte + nbytes > buf.len() {
                        // don't overrun buffer because of invalid UTF-8
                        (Other, 1, 1)
                    } else if let Ok(t) = from_utf8(&buf[byte..byte+nbytes]) {
                        match t.chars().next() {
                            Some('\t') => (Tab, 0, nbytes),
                            Some('\x08') => (Backspace, 0, nbytes),
                            Some(c) => (Other, UnicodeWidthChar::width(c).unwrap_or(0), nbytes),
                            None => {   // no valid char at start of t, so take 1 byte
                                (Other, 1, 1)
                            },
                        }
                    } else {
                        (Other, 1, 1)   // implicit assumption: non-UTF-8 char is 1 col wide
                    }
                } else {
                    (match buf[byte] {   // always take exactly 1 byte in strict ASCII mode
                        0x09 => Tab,
                        0x08 => Backspace,
                        _ => Other,
                    }, 1, 1)
                };

                // figure out how many columns this char takes up
                match ctype {
                    Tab => {
                        // figure out how many spaces to the next tabstop
                        let nts = next_tabstop(ts, col);
                        col += nts;

                        // now dump out either spaces if we're expanding, or a literal tab if we're not
                        if init || !options.iflag {
                            safe_unwrap!(output.write_all(&options.tspaces[..nts].as_bytes()));
                        } else {
                            safe_unwrap!(output.write_all(&buf[byte..byte+nbytes]));
                        }
                    },
                    _ => {
                        col = if ctype == Other {
                            col + cwidth
                        } else if col > 0 {
                            col - 1
                        } else {
                            0
                        };

                        // if we're writing anything other than a space, then we're
                        // done with the line's leading spaces
                        if buf[byte] != 0x20 {
                            init = false;
                        }

                        safe_unwrap!(output.write_all(&buf[byte..byte+nbytes]));
                    },
                }

                byte += nbytes; // advance the pointer
            }

            buf.truncate(0);    // clear the buffer
        }
    }
}
