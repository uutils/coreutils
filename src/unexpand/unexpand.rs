#![crate_name = "uu_unexpand"]

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

extern crate getopts;
extern crate unicode_width;

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Stdout, Write};
use std::str::from_utf8;
use unicode_width::UnicodeWidthChar;

static NAME: &'static str = "unexpand";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

static DEFAULT_TABSTOP: usize = 8;

fn tabstops_parse(s: String) -> Vec<usize> {
    let words = s.split(',').collect::<Vec<&str>>();

    let nums = words.into_iter()
        .map(|sn| sn.parse()
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
    aflag: bool,
    uflag: bool,
}

impl Options {
    fn new(matches: getopts::Matches) -> Options {
        let tabstops = match matches.opt_str("t") {
            None => vec!(DEFAULT_TABSTOP),
            Some(s) => tabstops_parse(s)
        };

        let aflag = (matches.opt_present("all") || matches.opt_present("tabs"))
                    && !matches.opt_present("first-only");
        let uflag = !matches.opt_present("U");

        let files =
            if matches.free.is_empty() {
                vec!("-".to_owned())
            } else {
                matches.free
            };

        Options { files: files, tabstops: tabstops, aflag: aflag, uflag: uflag }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("a", "all", "convert all blanks, instead of just initial blanks");
    opts.optflag("", "first-only", "convert only leading sequences of blanks (overrides -a)");
    opts.optopt("t", "tabs", "have tabs N characters apart instead of 8 (enables -a)", "N");
    opts.optopt("t", "tabs", "use comma separated LIST of tab positions (enables -a)", "LIST");
    opts.optflag("U", "no-utf8", "interpret input file as 8-bit ASCII rather than UTF-8");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        println!("{} {}\n", NAME, VERSION);
        println!("Usage: {} [OPTION]... [FILE]...\n", NAME);
        println!("{}", opts.usage(
            "Convert blanks in each FILE to tabs, writing to standard output.\n\
            With no FILE, or when FILE is -, read standard input."));
        return 0;
    }

    if matches.opt_present("V") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    unexpand(Options::new(matches));

    0
}

fn open(path: String) -> BufReader<Box<Read+'static>> {
    let file_buf;
    if path == "-" {
        BufReader::new(Box::new(stdin()) as Box<Read>)
    } else {
        file_buf = match File::open(&path[..]) {
            Ok(a) => a,
            Err(e) => crash!(1, "{}: {}", &path[..], e),
        };
        BufReader::new(Box::new(file_buf) as Box<Read>)
    }
}

fn next_tabstop(tabstops: &[usize], col: usize) -> Option<usize> {
    if tabstops.len() == 1 {
        Some(tabstops[0] - col % tabstops[0])
    } else {
        // find next larger tab
        match tabstops.iter().skip_while(|&&t| t <= col).next() {
            Some(t) => Some(t - col),
            None => None,   // if there isn't one in the list, tab becomes a single space
        }
    }
}

fn write_tabs(output: &mut BufWriter<Stdout>, tabstops: &[usize],
              mut scol: usize, col: usize, prevtab: bool, init: bool, amode: bool) {
    // This conditional establishes the following:
    // We never turn a single space before a non-blank into
    // a tab, unless it's at the start of the line.
    let ai = init || amode;
    if (ai && !prevtab && col > scol + 1) ||
       (col > scol && (init || ai && prevtab)) {
        while let Some(nts) = next_tabstop(tabstops, scol) {
            if col < scol + nts {
                break;
            }

            safe_unwrap!(output.write_all("\t".as_bytes()));
            scol += nts;
        }
    }

    while col > scol {
        safe_unwrap!(output.write_all(" ".as_bytes()));
        scol += 1;
    }
}

#[derive(PartialEq, Eq, Debug)]
enum CharType {
    Backspace,
    Space,
    Tab,
    Other,
}

fn unexpand(options: Options) {
    use self::CharType::*;

    let mut output = BufWriter::new(stdout());
    let ts = &options.tabstops[..];
    let mut buf = Vec::new();
    let lastcol = if ts.len() > 1 {
        *ts.last().unwrap()
    } else {
        0
    };

    for file in options.files.into_iter() {
        let mut fh = open(file);

        while match fh.read_until('\n' as u8, &mut buf) {
            Ok(s) => s > 0,
            Err(_) => !buf.is_empty(),
        } {
            let mut byte = 0;       // offset into the buffer
            let mut col = 0;        // the current column
            let mut scol = 0;       // the start col for the current span, i.e., the already-printed width
            let mut init = true;    // are we at the start of the line?
            let mut pctype = Other;

            while byte < buf.len() {
                // when we have a finite number of columns, never convert past the last column
                if lastcol > 0 && col >= lastcol {
                    write_tabs(&mut output, ts, scol, col, pctype == Tab, init, true);
                    safe_unwrap!(output.write_all(&buf[byte..]));
                    scol = col;
                    break;
                }

                let (ctype, cwidth, nbytes) = if options.uflag {
                    let nbytes = uucore::utf8::utf8_char_width(buf[byte]);

                    // figure out how big the next char is, if it's UTF-8
                    if byte + nbytes > buf.len() {
                        // make sure we don't overrun the buffer because of invalid UTF-8
                        (Other, 1, 1)
                    } else if let Ok(t) = from_utf8(&buf[byte..byte+nbytes]) {
                        // Now that we think it's UTF-8, figure out what kind of char it is
                        match t.chars().next() {
                            Some(' ') => (Space, 0, 1),
                            Some('\t') => (Tab, 0, 1),
                            Some('\x08') => (Backspace, 0, 1),
                            Some(c) => (Other, UnicodeWidthChar::width(c).unwrap_or(0), nbytes),
                            None => {   // invalid char snuck past the utf8_validation_iterator somehow???
                                (Other, 1, 1)
                            },
                        }
                    } else {
                        // otherwise, it's not valid
                        (Other, 1, 1)       // implicit assumption: non-UTF8 char has display width 1
                    }
                } else {
                    (match buf[byte] {      // always take exactly 1 byte in strict ASCII mode
                        0x20 => Space,
                        0x09 => Tab,
                        0x08 => Backspace,
                        _ => Other,
                    }, 1, 1)
                };

                // now figure out how many columns this char takes up, and maybe print it
                let tabs_buffered = init || options.aflag;
                match ctype {
                    Space | Tab => {    // compute next col, but only write space or tab chars if not buffering
                        col += if ctype == Space {
                            1
                        } else {
                            next_tabstop(ts, col).unwrap_or(1)
                        };

                        if !tabs_buffered {
                            safe_unwrap!(output.write_all(&buf[byte..byte+nbytes]));
                            scol = col;             // now printed up to this column
                        }
                    },
                    Other | Backspace => {  // always 
                        write_tabs(&mut output, ts, scol, col, pctype == Tab, init, options.aflag);
                        init = false;               // no longer at the start of a line
                        col = if ctype == Other {   // use computed width
                            col + cwidth
                        } else if col > 0 {         // Backspace case, but only if col > 0
                            col - 1
                        } else {
                            0
                        };
                        safe_unwrap!(output.write_all(&buf[byte..byte+nbytes]));
                        scol = col;                 // we've now printed up to this column
                    },
                }

                byte += nbytes; // move on to next char
                pctype = ctype; // save the previous type
            }

            // write out anything remaining
            write_tabs(&mut output, ts, scol, col, pctype == Tab, init, true);
            buf.truncate(0);    // clear out the buffer
        }
    }
    pipe_flush!(output);
}
