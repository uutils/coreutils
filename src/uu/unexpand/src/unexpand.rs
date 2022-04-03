//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Virgile Andreani <virgile.andreani@anbuco.fr>
//  * (c) kwantam <kwantam@gmail.com>
//  *     * 2015-04-28 ~ updated to work with both UTF-8 and non-UTF-8 encodings
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) nums aflag uflag scol prevtab amode ctype cwidth nbytes lastcol pctype

#[macro_use]
extern crate uucore;
use clap::{crate_version, Arg, Command};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Stdout, Write};
use std::str::from_utf8;
use unicode_width::UnicodeWidthChar;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::{format_usage, InvalidEncodingHandling};

static NAME: &str = "unexpand";
static USAGE: &str = "{} [OPTION]... [FILE]...";
static SUMMARY: &str = "Convert blanks in each FILE to tabs, writing to standard output.\n\
                        With no FILE, or when FILE is -, read standard input.";

const DEFAULT_TABSTOP: usize = 8;

fn tabstops_parse(s: &str) -> Vec<usize> {
    let words = s.split(',');

    let nums = words
        .map(|sn| {
            sn.parse()
                .unwrap_or_else(|_| crash!(1, "{}\n", "tab size contains invalid character(s)"))
        })
        .collect::<Vec<usize>>();

    if nums.iter().any(|&n| n == 0) {
        crash!(1, "{}\n", "tab size cannot be 0");
    }

    if let (false, _) = nums
        .iter()
        .fold((true, 0), |(acc, last), &n| (acc && last <= n, n))
    {
        crash!(1, "{}\n", "tab sizes must be ascending");
    }

    nums
}

mod options {
    pub const FILE: &str = "file";
    pub const ALL: &str = "all";
    pub const FIRST_ONLY: &str = "first-only";
    pub const TABS: &str = "tabs";
    pub const NO_UTF8: &str = "no-utf8";
}

struct Options {
    files: Vec<String>,
    tabstops: Vec<usize>,
    aflag: bool,
    uflag: bool,
}

impl Options {
    fn new(matches: &clap::ArgMatches) -> Self {
        let tabstops = match matches.value_of(options::TABS) {
            None => vec![DEFAULT_TABSTOP],
            Some(s) => tabstops_parse(s),
        };

        let aflag = (matches.is_present(options::ALL) || matches.is_present(options::TABS))
            && !matches.is_present(options::FIRST_ONLY);
        let uflag = !matches.is_present(options::NO_UTF8);

        let files = match matches.value_of(options::FILE) {
            Some(v) => vec![v.to_string()],
            None => vec!["-".to_owned()],
        };

        Self {
            files,
            tabstops,
            aflag,
            uflag,
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    unexpand(&Options::new(&matches)).map_err_context(String::new)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(Arg::new(options::FILE).hide(true).multiple_occurrences(true))
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("convert all blanks, instead of just initial blanks")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::FIRST_ONLY)
                .long(options::FIRST_ONLY)
                .help("convert only leading sequences of blanks (overrides -a)")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::TABS)
                .short('t')
                .long(options::TABS)
                .long_help("use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)")
                .takes_value(true)
        )
        .arg(
            Arg::new(options::NO_UTF8)
                .short('U')
                .long(options::NO_UTF8)
                .takes_value(false)
                .help("interpret input file as 8-bit ASCII rather than UTF-8"))
}

fn open(path: &str) -> BufReader<Box<dyn Read + 'static>> {
    let file_buf;
    if path == "-" {
        BufReader::new(Box::new(stdin()) as Box<dyn Read>)
    } else {
        file_buf = match File::open(&path) {
            Ok(a) => a,
            Err(e) => crash!(1, "{}: {}", path.maybe_quote(), e),
        };
        BufReader::new(Box::new(file_buf) as Box<dyn Read>)
    }
}

fn next_tabstop(tabstops: &[usize], col: usize) -> Option<usize> {
    if tabstops.len() == 1 {
        Some(tabstops[0] - col % tabstops[0])
    } else {
        // find next larger tab
        // if there isn't one in the list, tab becomes a single space
        tabstops.iter().find(|&&t| t > col).map(|t| t - col)
    }
}

fn write_tabs(
    output: &mut BufWriter<Stdout>,
    tabstops: &[usize],
    mut scol: usize,
    col: usize,
    prevtab: bool,
    init: bool,
    amode: bool,
) {
    // This conditional establishes the following:
    // We never turn a single space before a non-blank into
    // a tab, unless it's at the start of the line.
    let ai = init || amode;
    if (ai && !prevtab && col > scol + 1) || (col > scol && (init || ai && prevtab)) {
        while let Some(nts) = next_tabstop(tabstops, scol) {
            if col < scol + nts {
                break;
            }

            crash_if_err!(1, output.write_all(b"\t"));
            scol += nts;
        }
    }

    while col > scol {
        crash_if_err!(1, output.write_all(b" "));
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

fn next_char_info(uflag: bool, buf: &[u8], byte: usize) -> (CharType, usize, usize) {
    let (ctype, cwidth, nbytes) = if uflag {
        let nbytes = char::from(buf[byte]).len_utf8();

        if byte + nbytes > buf.len() {
            // make sure we don't overrun the buffer because of invalid UTF-8
            (CharType::Other, 1, 1)
        } else if let Ok(t) = from_utf8(&buf[byte..byte + nbytes]) {
            // Now that we think it's UTF-8, figure out what kind of char it is
            match t.chars().next() {
                Some(' ') => (CharType::Space, 0, 1),
                Some('\t') => (CharType::Tab, 0, 1),
                Some('\x08') => (CharType::Backspace, 0, 1),
                Some(c) => (
                    CharType::Other,
                    UnicodeWidthChar::width(c).unwrap_or(0),
                    nbytes,
                ),
                None => {
                    // invalid char snuck past the utf8_validation_iterator somehow???
                    (CharType::Other, 1, 1)
                }
            }
        } else {
            // otherwise, it's not valid
            (CharType::Other, 1, 1) // implicit assumption: non-UTF8 char has display width 1
        }
    } else {
        (
            match buf[byte] {
                // always take exactly 1 byte in strict ASCII mode
                0x20 => CharType::Space,
                0x09 => CharType::Tab,
                0x08 => CharType::Backspace,
                _ => CharType::Other,
            },
            1,
            1,
        )
    };

    (ctype, cwidth, nbytes)
}

fn unexpand(options: &Options) -> std::io::Result<()> {
    let mut output = BufWriter::new(stdout());
    let ts = &options.tabstops[..];
    let mut buf = Vec::new();
    let lastcol = if ts.len() > 1 { *ts.last().unwrap() } else { 0 };

    for file in &options.files {
        let mut fh = open(file);

        while match fh.read_until(b'\n', &mut buf) {
            Ok(s) => s > 0,
            Err(_) => !buf.is_empty(),
        } {
            let mut byte = 0; // offset into the buffer
            let mut col = 0; // the current column
            let mut scol = 0; // the start col for the current span, i.e., the already-printed width
            let mut init = true; // are we at the start of the line?
            let mut pctype = CharType::Other;

            while byte < buf.len() {
                // when we have a finite number of columns, never convert past the last column
                if lastcol > 0 && col >= lastcol {
                    write_tabs(
                        &mut output,
                        ts,
                        scol,
                        col,
                        pctype == CharType::Tab,
                        init,
                        true,
                    );
                    output.write_all(&buf[byte..])?;
                    scol = col;
                    break;
                }

                // figure out how big the next char is, if it's UTF-8
                let (ctype, cwidth, nbytes) = next_char_info(options.uflag, &buf, byte);

                // now figure out how many columns this char takes up, and maybe print it
                let tabs_buffered = init || options.aflag;
                match ctype {
                    CharType::Space | CharType::Tab => {
                        // compute next col, but only write space or tab chars if not buffering
                        col += if ctype == CharType::Space {
                            1
                        } else {
                            next_tabstop(ts, col).unwrap_or(1)
                        };

                        if !tabs_buffered {
                            output.write_all(&buf[byte..byte + nbytes])?;
                            scol = col; // now printed up to this column
                        }
                    }
                    CharType::Other | CharType::Backspace => {
                        // always
                        write_tabs(
                            &mut output,
                            ts,
                            scol,
                            col,
                            pctype == CharType::Tab,
                            init,
                            options.aflag,
                        );
                        init = false; // no longer at the start of a line
                        col = if ctype == CharType::Other {
                            // use computed width
                            col + cwidth
                        } else if col > 0 {
                            // Backspace case, but only if col > 0
                            col - 1
                        } else {
                            0
                        };
                        output.write_all(&buf[byte..byte + nbytes])?;
                        scol = col; // we've now printed up to this column
                    }
                }

                byte += nbytes; // move on to next char
                pctype = ctype; // save the previous type
            }

            // write out anything remaining
            write_tabs(
                &mut output,
                ts,
                scol,
                col,
                pctype == CharType::Tab,
                init,
                true,
            );
            output.flush()?;
            buf.truncate(0); // clear out the buffer
        }
    }
    output.flush()
}
