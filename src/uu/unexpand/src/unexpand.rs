// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) nums aflag uflag scol prevtab amode ctype cwidth nbytes lastcol pctype Preprocess

use clap::{crate_version, Arg, ArgAction, Command};
use quick_error::quick_error;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Stdout, Write};
use std::num::IntErrorKind;
use std::path::Path;
use std::str::from_utf8;
use unicode_width::UnicodeWidthChar;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::{crash_if_err, format_usage, help_about, help_usage, show};

const USAGE: &str = help_usage!("unexpand.md");
const ABOUT: &str = help_about!("unexpand.md");

const DEFAULT_TABSTOP: usize = 8;

quick_error! {
    #[derive(Debug)]
    pub enum ParseError {
        InvalidCharacter(s: String) {
            display("tab size contains invalid character(s): {}", s.quote())
        }
        TabSizeCannotBeZero {
            display("tab size cannot be 0")
        }
        TabSizeTooLarge {
            display("tab stop value is too large")
        }
        TabSizesMustBeAscending {
            display("tab sizes must be ascending")
        }
    }
}

impl UError for ParseError {}

fn tabstops_parse(s: &str) -> Result<Vec<usize>, ParseError> {
    let words = s.split(',');

    let mut nums = Vec::new();

    for word in words {
        match word.parse::<usize>() {
            Ok(num) => nums.push(num),
            Err(e) => match e.kind() {
                IntErrorKind::PosOverflow => return Err(ParseError::TabSizeTooLarge),
                _ => {
                    return Err(ParseError::InvalidCharacter(
                        word.trim_start_matches(char::is_numeric).to_string(),
                    ))
                }
            },
        }
    }

    if nums.iter().any(|&n| n == 0) {
        return Err(ParseError::TabSizeCannotBeZero);
    }

    if let (false, _) = nums
        .iter()
        .fold((true, 0), |(acc, last), &n| (acc && last < n, n))
    {
        return Err(ParseError::TabSizesMustBeAscending);
    }

    Ok(nums)
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
    fn new(matches: &clap::ArgMatches) -> Result<Self, ParseError> {
        let tabstops = match matches.get_many::<String>(options::TABS) {
            None => vec![DEFAULT_TABSTOP],
            Some(s) => tabstops_parse(&s.map(|s| s.as_str()).collect::<Vec<_>>().join(","))?,
        };

        let aflag = (matches.get_flag(options::ALL) || matches.contains_id(options::TABS))
            && !matches.get_flag(options::FIRST_ONLY);
        let uflag = !matches.get_flag(options::NO_UTF8);

        let files = match matches.get_many::<String>(options::FILE) {
            Some(v) => v.cloned().collect(),
            None => vec!["-".to_owned()],
        };

        Ok(Self {
            files,
            tabstops,
            aflag,
            uflag,
        })
    }
}

/// Decide whether the character is either a digit or a comma.
fn is_digit_or_comma(c: char) -> bool {
    c.is_ascii_digit() || c == ','
}

/// Preprocess command line arguments and expand shortcuts. For example, "-7" is expanded to
/// "--tabs=7 --first-only" and "-1,3" to "--tabs=1 --tabs=3 --first-only". However, if "-a" or
/// "--all" is provided, "--first-only" is omitted.
fn expand_shortcuts(args: &[String]) -> Vec<String> {
    let mut processed_args = Vec::with_capacity(args.len());
    let mut is_all_arg_provided = false;
    let mut has_shortcuts = false;

    for arg in args {
        if arg.starts_with('-') && arg[1..].chars().all(is_digit_or_comma) {
            arg[1..]
                .split(',')
                .filter(|s| !s.is_empty())
                .for_each(|s| processed_args.push(format!("--tabs={s}")));
            has_shortcuts = true;
        } else {
            processed_args.push(arg.to_string());

            if arg == "--all" || arg == "-a" {
                is_all_arg_provided = true;
            }
        }
    }

    if has_shortcuts && !is_all_arg_provided {
        processed_args.push("--first-only".into());
    }

    processed_args
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let matches = uu_app().try_get_matches_from(expand_shortcuts(&args))?;

    unexpand(&Options::new(&matches)?)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("convert all blanks, instead of just initial blanks")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FIRST_ONLY)
                .long(options::FIRST_ONLY)
                .help("convert only leading sequences of blanks (overrides -a)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TABS)
                .short('t')
                .long(options::TABS)
                .help(
                    "use comma separated LIST of tab positions or have tabs N characters \
                apart instead of 8 (enables -a)",
                )
                .action(ArgAction::Append)
                .value_name("N, LIST"),
        )
        .arg(
            Arg::new(options::NO_UTF8)
                .short('U')
                .long(options::NO_UTF8)
                .help("interpret input file as 8-bit ASCII rather than UTF-8")
                .action(ArgAction::SetTrue),
        )
}

fn open(path: &str) -> UResult<BufReader<Box<dyn Read + 'static>>> {
    let file_buf;
    let filename = Path::new(path);
    if filename.is_dir() {
        Err(Box::new(USimpleError {
            code: 1,
            message: format!("{}: Is a directory", filename.display()),
        }))
    } else if path == "-" {
        Ok(BufReader::new(Box::new(stdin()) as Box<dyn Read>))
    } else {
        file_buf = File::open(path).map_err_context(|| path.to_string())?;
        Ok(BufReader::new(Box::new(file_buf) as Box<dyn Read>))
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

#[allow(clippy::cognitive_complexity)]
fn unexpand_line(
    buf: &mut Vec<u8>,
    output: &mut BufWriter<std::io::Stdout>,
    options: &Options,
    lastcol: usize,
    ts: &[usize],
) -> std::io::Result<()> {
    let mut byte = 0; // offset into the buffer
    let mut col = 0; // the current column
    let mut scol = 0; // the start col for the current span, i.e., the already-printed width
    let mut init = true; // are we at the start of the line?
    let mut pctype = CharType::Other;

    while byte < buf.len() {
        // when we have a finite number of columns, never convert past the last column
        if lastcol > 0 && col >= lastcol {
            write_tabs(output, ts, scol, col, pctype == CharType::Tab, init, true);
            output.write_all(&buf[byte..])?;
            scol = col;
            break;
        }

        // figure out how big the next char is, if it's UTF-8
        let (ctype, cwidth, nbytes) = next_char_info(options.uflag, buf, byte);

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
                    output,
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
    write_tabs(output, ts, scol, col, pctype == CharType::Tab, init, true);
    output.flush()?;
    buf.truncate(0); // clear out the buffer

    Ok(())
}

fn unexpand(options: &Options) -> UResult<()> {
    let mut output = BufWriter::new(stdout());
    let ts = &options.tabstops[..];
    let mut buf = Vec::new();
    let lastcol = if ts.len() > 1 { *ts.last().unwrap() } else { 0 };

    for file in &options.files {
        let mut fh = match open(file) {
            Ok(reader) => reader,
            Err(err) => {
                show!(err);
                continue;
            }
        };

        while match fh.read_until(b'\n', &mut buf) {
            Ok(s) => s > 0,
            Err(_) => !buf.is_empty(),
        } {
            unexpand_line(&mut buf, &mut output, options, lastcol, ts)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::is_digit_or_comma;

    #[test]
    fn test_is_digit_or_comma() {
        assert!(is_digit_or_comma('1'));
        assert!(is_digit_or_comma(','));
        assert!(!is_digit_or_comma('a'));
    }
}
