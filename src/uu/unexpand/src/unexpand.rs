// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) nums aflag uflag scol prevtab amode ctype cwidth nbytes lastcol pctype Preprocess

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Stdout, Write, stdin, stdout};
use std::num::IntErrorKind;
use std::path::Path;
use std::str::from_utf8;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError, set_exit_code};
use uucore::translate;
use uucore::{format_usage, show};

const DEFAULT_TABSTOP: usize = 8;

#[derive(Debug, Error)]
enum ParseError {
    #[error("{}", translate!("unexpand-error-invalid-character", "char" => _0.quote()))]
    InvalidCharacter(String),
    #[error("{}", translate!("unexpand-error-tab-size-cannot-be-zero"))]
    TabSizeCannotBeZero,
    #[error("{}", translate!("unexpand-error-tab-size-too-large"))]
    TabSizeTooLarge,
    #[error("{}", translate!("unexpand-error-tab-sizes-must-be-ascending"))]
    TabSizesMustBeAscending,
}

impl UError for ParseError {}

fn parse_tab_num(word: &str, allow_zero: bool) -> Result<usize, ParseError> {
    match word.parse::<usize>() {
        Ok(0) if !allow_zero => Err(ParseError::TabSizeCannotBeZero),
        Ok(num) => Ok(num),
        Err(e) => match e.kind() {
            IntErrorKind::PosOverflow => Err(ParseError::TabSizeTooLarge),
            _ => Err(ParseError::InvalidCharacter(
                word.trim_start_matches(char::is_numeric).to_string(),
            )),
        },
    }
}

fn parse_tabstops(s: &str) -> Result<TabConfig, ParseError> {
    let words = s.split(',');

    let mut nums = Vec::new();
    let mut increment_size: Option<usize> = None;
    let mut extend_size: Option<usize> = None;

    for word in words {
        if word.is_empty() {
            continue;
        }

        // Handle extended syntax: +N (increment) and /N (repeat)
        if let Some(word) = word.strip_prefix('+') {
            // +N means N positions after the last tab stop (only allowed at end)
            if increment_size.is_some() || extend_size.is_some() {
                return Err(ParseError::InvalidCharacter("+".to_string()));
            }
            let value = parse_tab_num(word, true)?;
            if nums.is_empty() {
                // Standalone +N: treat as tab stops at multiples of N
                if value == 0 {
                    return Err(ParseError::TabSizeCannotBeZero);
                }
                return Ok(TabConfig {
                    tabstops: vec![value],
                    increment_size: None,
                    extend_size: None,
                });
            }
            increment_size = Some(value);
        } else if let Some(word) = word.strip_prefix('/') {
            // /N means repeat every N positions after the last tab stop
            if increment_size.is_some() || extend_size.is_some() {
                return Err(ParseError::InvalidCharacter("/".to_string()));
            }
            let value = parse_tab_num(word, true)?;
            if nums.is_empty() {
                // Standalone /N: treat as tab stops at multiples of N
                if value == 0 {
                    return Err(ParseError::TabSizeCannotBeZero);
                }
                return Ok(TabConfig {
                    tabstops: vec![value],
                    increment_size: None,
                    extend_size: None,
                });
            }
            extend_size = Some(value);
        } else {
            // Regular number
            if increment_size.is_some() || extend_size.is_some() {
                return Err(ParseError::InvalidCharacter(word.to_string()));
            }
            nums.push(parse_tab_num(word, false)?);
        }
    }

    if nums.is_empty() && increment_size.is_none() && extend_size.is_none() {
        return Ok(TabConfig {
            tabstops: vec![DEFAULT_TABSTOP],
            increment_size: None,
            extend_size: None,
        });
    }

    // Handle the increment if specified
    // Only add an extra tab stop if increment is non-zero
    if let Some(inc) = increment_size {
        if inc > 0 {
            let last = *nums.last().unwrap();
            nums.push(last + inc);
        }
    }

    if let (false, _) = nums
        .iter()
        .fold((true, 0), |(acc, last), &n| (acc && last < n, n))
    {
        return Err(ParseError::TabSizesMustBeAscending);
    }

    Ok(TabConfig {
        tabstops: nums,
        increment_size,
        extend_size,
    })
}

mod options {
    pub const FILE: &str = "file";
    pub const ALL: &str = "all";
    pub const FIRST_ONLY: &str = "first-only";
    pub const TABS: &str = "tabs";
    pub const NO_UTF8: &str = "no-utf8";
}

struct TabConfig {
    tabstops: Vec<usize>,
    increment_size: Option<usize>,
    extend_size: Option<usize>,
}

struct Options {
    files: Vec<OsString>,
    tab_config: TabConfig,
    aflag: bool,
    uflag: bool,
}

impl Options {
    fn new(matches: &clap::ArgMatches) -> Result<Self, ParseError> {
        let tab_config = match matches.get_many::<String>(options::TABS) {
            None => TabConfig {
                tabstops: vec![DEFAULT_TABSTOP],
                increment_size: None,
                extend_size: None,
            },
            Some(s) => parse_tabstops(&s.map(String::as_str).collect::<Vec<_>>().join(","))?,
        };

        let aflag = (matches.get_flag(options::ALL) || matches.contains_id(options::TABS))
            && !matches.get_flag(options::FIRST_ONLY);
        let uflag = !matches.get_flag(options::NO_UTF8);

        let files = match matches.get_many::<OsString>(options::FILE) {
            Some(v) => v.cloned().collect(),
            None => vec![OsString::from("-")],
        };

        Ok(Self {
            files,
            tab_config,
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
fn expand_shortcuts(args: Vec<OsString>) -> Vec<OsString> {
    let mut processed_args = Vec::with_capacity(args.len());
    let mut is_all_arg_provided = false;
    let mut has_shortcuts = false;

    for arg in args {
        if let Some(arg) = arg.to_str() {
            if arg.starts_with('-') && arg[1..].chars().all(is_digit_or_comma) {
                arg[1..]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .for_each(|s| processed_args.push(OsString::from(format!("--tabs={s}"))));
                has_shortcuts = true;
            } else {
                processed_args.push(arg.into());

                if arg == "--all" || arg == "-a" {
                    is_all_arg_provided = true;
                }
            }
        } else {
            processed_args.push(arg);
        }
    }

    if has_shortcuts && !is_all_arg_provided {
        processed_args.push("--first-only".into());
    }

    processed_args
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches =
        uucore::clap_localization::handle_clap_result(uu_app(), expand_shortcuts(args.collect()))?;

    unexpand(&Options::new(&matches)?)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("unexpand-usage")))
        .about(translate!("unexpand-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help(translate!("unexpand-help-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FIRST_ONLY)
                .short('f')
                .long(options::FIRST_ONLY)
                .help(translate!("unexpand-help-first-only"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TABS)
                .short('t')
                .long(options::TABS)
                .help(translate!("unexpand-help-tabs"))
                .action(ArgAction::Append)
                .value_name("N, LIST"),
        )
        .arg(
            Arg::new(options::NO_UTF8)
                .short('U')
                .long(options::NO_UTF8)
                .help(translate!("unexpand-help-no-utf8"))
                .action(ArgAction::SetTrue),
        )
}

fn open(path: &OsString) -> UResult<BufReader<Box<dyn Read + 'static>>> {
    let file_buf;
    let filename = Path::new(path);
    if filename.is_dir() {
        Err(Box::new(USimpleError {
            code: 1,
            message: translate!("unexpand-error-is-directory", "path" => filename.maybe_quote()),
        }))
    } else if path == "-" {
        Ok(BufReader::new(Box::new(stdin()) as Box<dyn Read>))
    } else {
        file_buf = File::open(path).map_err_context(|| path.maybe_quote().to_string())?;
        Ok(BufReader::new(Box::new(file_buf) as Box<dyn Read>))
    }
}

fn next_tabstop(tab_config: &TabConfig, col: usize) -> Option<usize> {
    let tabstops = &tab_config.tabstops;

    if tabstops.is_empty() {
        return None;
    }

    if tabstops.len() == 1
        && !matches!(tab_config.increment_size, Some(n) if n > 0)
        && !matches!(tab_config.extend_size, Some(n) if n > 0)
    {
        // Simple case: single tab stop, repeat at that interval
        Some(tabstops[0] - col % tabstops[0])
    } else {
        // Find next larger tab
        if let Some(&next_tab) = tabstops.iter().find(|&&t| t > col) {
            Some(next_tab - col)
        } else {
            // We're past the last explicit tab stop
            if let Some(&last_tab) = tabstops.last() {
                if let Some(extend_size) = tab_config.extend_size {
                    // /N: tab stops at multiples of N
                    if extend_size == 0 {
                        return None;
                    }
                    Some(extend_size - (col % extend_size))
                } else if let Some(increment_size) = tab_config.increment_size {
                    // +N: continue with increment after last tab stop
                    if increment_size == 0 || col < last_tab {
                        return None;
                    }
                    let distance_from_last = col - last_tab;
                    let remainder = distance_from_last % increment_size;
                    Some(if remainder == 0 {
                        increment_size
                    } else {
                        increment_size - remainder
                    })
                } else {
                    // No more tabs
                    None
                }
            } else {
                None
            }
        }
    }
}

fn write_tabs(
    output: &mut BufWriter<Stdout>,
    tab_config: &TabConfig,
    print_state: &mut PrintState,
    amode: bool,
) -> UResult<()> {
    // This conditional establishes the following:
    // We never turn a single space before a non-blank into
    // a tab, unless it's at the start of the line.
    let ai = print_state.leading || amode;
    if (ai && print_state.pctype != CharType::Tab && print_state.col > print_state.scol + 1)
        || (print_state.col > print_state.scol
            && (print_state.leading || ai && print_state.pctype == CharType::Tab))
    {
        while let Some(nts) = next_tabstop(tab_config, print_state.scol) {
            if print_state.col < print_state.scol + nts {
                break;
            }

            output.write_all(b"\t")?;
            print_state.scol += nts;
        }
    }

    while print_state.col > print_state.scol {
        output.write_all(b" ")?;
        print_state.scol += 1;
    }
    Ok(())
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

        let Some(slice) = buf.get(byte..byte + nbytes) else {
            // don't overrun buffer because of invalid UTF-8
            return (CharType::Other, 1, 1);
        };

        if let Ok(t) = from_utf8(slice) {
            // Now that we think it's UTF-8, figure out what kind of char it is
            match t.chars().next() {
                Some(' ') => (CharType::Space, 0, 1),
                Some('\t') => (CharType::Tab, 0, 1),
                Some('\x08') => (CharType::Backspace, 0, 1),
                Some(_) => (CharType::Other, nbytes, nbytes),
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

// This struct is used to store the current state of printing the input buf.
// Things that need to be tracked are
// - are we in the leading whitespaces before any other chars of this line
// - what columns have already been printed
// - how many whitespaces have been found but not yet been printed
// - what type was the last Char
struct PrintState {
    col: usize,
    scol: usize,
    leading: bool,
    pctype: CharType,
}

impl PrintState {
    // reinitializes the PrintState struct to beginning of line values
    fn new_line(&mut self) {
        self.col = 0;
        self.scol = 0;
        self.leading = true;
        self.pctype = CharType::Other;
    }
}

#[allow(clippy::cognitive_complexity)]
fn unexpand_buf(
    buf: &[u8],
    output: &mut BufWriter<Stdout>,
    options: &Options,
    lastcol: usize,
    tab_config: &TabConfig,
    print_state: &mut PrintState,
) -> UResult<()> {
    // We can only fast forward if we don't need to calculate col/scol
    if let Some(b'\n') = buf.last() {
        // Fast path: if we're not converting all spaces (-a flag not set)
        // and the line doesn't start with spaces, just write it directly
        if !options.aflag
            && !buf.is_empty()
            && ((buf[0] != b' ' && buf[0] != b'\t') || !print_state.leading)
        {
            write_tabs(output, tab_config, print_state, options.aflag)?;
            print_state.scol = print_state.col;
            print_state.col += buf.len();
            output.write_all(buf)?;
            return Ok(());
        }
    }

    let mut byte = 0; // offset into the buffer

    // We can only fast forward if we don't need to calculate col/scol
    if let Some(b'\n') = buf.last() {
        // Fast path for leading spaces in non-UTF8 mode: count consecutive spaces/tabs at start
        if !options.uflag && !options.aflag && print_state.leading {
            // In default mode (not -a), we only convert leading spaces
            // So we can batch process them and then copy the rest
            while byte < buf.len() {
                match buf[byte] {
                    b' ' => {
                        print_state.col += 1;
                        byte += 1;
                    }
                    b'\t' => {
                        print_state.col += next_tabstop(tab_config, print_state.col).unwrap_or(1);
                        byte += 1;
                        print_state.pctype = CharType::Tab;
                    }
                    _ => break,
                }
            }

            // If we found spaces/tabs, write them as tabs
            if byte > 0 {
                write_tabs(output, tab_config, print_state, options.aflag)?;
            }

            // Write the rest of the line directly (no more tab conversion needed)
            if byte < buf.len() {
                print_state.leading = false;
                output.write_all(&buf[byte..])?;
            }
            return Ok(());
        }
    }

    while byte < buf.len() {
        // when we have a finite number of columns, never convert past the last column
        if lastcol > 0 && print_state.col >= lastcol {
            write_tabs(output, tab_config, print_state, true)?;
            output.write_all(&buf[byte..])?;
            print_state.scol = print_state.col;
            break;
        }

        // figure out how big the next char is, if it's UTF-8
        let (ctype, cwidth, nbytes) = next_char_info(options.uflag, buf, byte);

        // now figure out how many columns this char takes up, and maybe print it
        let tabs_buffered = print_state.leading || options.aflag;
        match ctype {
            CharType::Space | CharType::Tab => {
                // compute next col, but only write space or tab chars if not buffering
                print_state.col += if ctype == CharType::Space {
                    1
                } else {
                    next_tabstop(tab_config, print_state.col).unwrap_or(1)
                };

                if !tabs_buffered {
                    output.write_all(&buf[byte..byte + nbytes])?;
                    print_state.scol = print_state.col; // now printed up to this column
                }
            }
            CharType::Other | CharType::Backspace => {
                // always
                write_tabs(output, tab_config, print_state, options.aflag)?;
                print_state.leading = false; // no longer at the start of a line
                print_state.col = if ctype == CharType::Other {
                    // use computed width
                    print_state.col + cwidth
                } else if print_state.col > 0 {
                    // Backspace case, but only if col > 0
                    print_state.col - 1
                } else {
                    0
                };
                output.write_all(&buf[byte..byte + nbytes])?;
                print_state.scol = print_state.col; // we've now printed up to this column
            }
        }

        byte += nbytes; // move on to next char
        print_state.pctype = ctype; // save the previous type
    }

    Ok(())
}

fn unexpand_file(
    file: &OsString,
    output: &mut BufWriter<Stdout>,
    options: &Options,
    lastcol: usize,
    tab_config: &TabConfig,
) -> UResult<()> {
    let mut buf = [0u8; 4096];
    let mut input = open(file)?;
    let mut print_state = PrintState {
        col: 0,
        scol: 0,
        leading: true,
        pctype: CharType::Other,
    };

    loop {
        match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for line in buf[..n].split_inclusive(|b| *b == b'\n') {
                    unexpand_buf(line, output, options, lastcol, tab_config, &mut print_state)?;
                    if let Some(b'\n') = line.last() {
                        print_state.new_line();
                    }
                }
            }
            Err(e) => return Err(e.map_err_context(|| file.maybe_quote().to_string())),
        }
    }
    // write out anything remaining
    write_tabs(output, tab_config, &mut print_state, options.aflag)?;
    Ok(())
}

fn unexpand(options: &Options) -> UResult<()> {
    let mut output = BufWriter::new(stdout());
    let tab_config = &options.tab_config;
    let lastcol = if tab_config.tabstops.len() > 1
        && tab_config.increment_size.is_none()
        && tab_config.extend_size.is_none()
    {
        *tab_config.tabstops.last().unwrap()
    } else {
        0
    };

    for file in &options.files {
        if let Err(e) = unexpand_file(file, &mut output, options, lastcol, tab_config) {
            show!(e);
            set_exit_code(1);
        }
    }
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{ParseError, is_digit_or_comma, parse_tab_num, parse_tabstops};

    #[test]
    fn test_is_digit_or_comma() {
        assert!(is_digit_or_comma('1'));
        assert!(is_digit_or_comma(','));
        assert!(!is_digit_or_comma('a'));
    }

    #[test]
    fn test_parse_tab_num() {
        assert_eq!(parse_tab_num("6", false).unwrap(), 6);
        assert_eq!(parse_tab_num("12", false).unwrap(), 12);
        assert_eq!(parse_tab_num("9", false).unwrap(), 9);
        assert_eq!(parse_tab_num("4", false).unwrap(), 4);
    }

    #[test]
    fn test_parse_tab_num_errors() {
        // Zero is not allowed when allow_zero is false
        assert!(matches!(
            parse_tab_num("0", false),
            Err(ParseError::TabSizeCannotBeZero)
        ));

        // Zero is allowed when allow_zero is true
        assert_eq!(parse_tab_num("0", true).unwrap(), 0);

        // Invalid character
        assert!(matches!(
            parse_tab_num("6x", false),
            Err(ParseError::InvalidCharacter(_))
        ));

        // Invalid character
        assert!(matches!(
            parse_tab_num("9y", false),
            Err(ParseError::InvalidCharacter(_))
        ));
    }

    #[test]
    fn test_parse_tabstops_extended_syntax() {
        // Standalone +N is now allowed (treated as multiples of N)
        let config = parse_tabstops("+6").unwrap();
        assert_eq!(config.tabstops, vec![6]);
        assert_eq!(config.increment_size, None);
        assert_eq!(config.extend_size, None);

        // Standalone /N is now allowed (treated as multiples of N)
        let config = parse_tabstops("/9").unwrap();
        assert_eq!(config.tabstops, vec![9]);
        assert_eq!(config.increment_size, None);
        assert_eq!(config.extend_size, None);

        // +0 and /0 are not allowed as standalone
        assert!(matches!(
            parse_tabstops("+0"),
            Err(ParseError::TabSizeCannotBeZero)
        ));
        assert!(matches!(
            parse_tabstops("/0"),
            Err(ParseError::TabSizeCannotBeZero)
        ));

        // Valid +N with previous tab stop
        let config = parse_tabstops("3,+6").unwrap();
        assert_eq!(config.tabstops, vec![3, 9]);
        assert_eq!(config.increment_size, Some(6));

        // Valid /N with previous tab stop
        let config = parse_tabstops("3,/4").unwrap();
        assert_eq!(config.tabstops, vec![3]);
        assert_eq!(config.extend_size, Some(4));

        // +0 with previous tab stop should be allowed
        let config = parse_tabstops("3,+0").unwrap();
        assert_eq!(config.tabstops, vec![3]);
        assert_eq!(config.increment_size, Some(0));

        // /0 with previous tab stop should be allowed
        let config = parse_tabstops("3,/0").unwrap();
        assert_eq!(config.tabstops, vec![3]);
        assert_eq!(config.extend_size, Some(0));
    }

    #[test]
    fn test_next_tabstop_with_increment() {
        use crate::{next_tabstop, parse_tabstops};

        // Test with "3,+6" configuration
        let config = parse_tabstops("3,+6").unwrap();

        // Verify the parsed configuration
        assert_eq!(config.tabstops, vec![3, 9]);
        assert_eq!(config.increment_size, Some(6));

        // Tab stops should be at 3, 9, 15, 21, ...
        assert_eq!(next_tabstop(&config, 0), Some(3)); // 0 → 3
        assert_eq!(next_tabstop(&config, 1), Some(2)); // 1 → 3
        assert_eq!(next_tabstop(&config, 2), Some(1)); // 2 → 3
        assert_eq!(next_tabstop(&config, 3), Some(6)); // 3 → 9
        assert_eq!(next_tabstop(&config, 4), Some(5)); // 4 → 9
        assert_eq!(next_tabstop(&config, 8), Some(1)); // 8 → 9
        assert_eq!(next_tabstop(&config, 9), Some(6)); // 9 → 15
        assert_eq!(next_tabstop(&config, 15), Some(6)); // 15 → 21
    }
}
