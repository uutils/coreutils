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
use memchr::memchr;

const DEFAULT_TABSTOP: usize = 8;
const READ_BUF_SIZE: usize = 8192;

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
            Some(s) => parse_tabstops(&s.map(|s| s.as_str()).collect::<Vec<_>>().join(","))?,
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
    mut scol: usize,
    col: usize,
    prevtab: bool,
    init: bool,
    amode: bool,
) -> UResult<()> {
    // This conditional establishes the following:
    // We never turn a single space before a non-blank into
    // a tab, unless it's at the start of the line.
    let ai = init || amode;
    if (ai && !prevtab && col > scol + 1) || (col > scol && (init || ai && prevtab)) {
        while let Some(nts) = next_tabstop(tab_config, scol) {
            if col < scol + nts {
                break;
            }

            output.write_all(b"\t")?;
            scol += nts;
        }
    }

    while col > scol {
        output.write_all(b" ")?;
        scol += 1;
    }
    Ok(())
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum CharType {
    Backspace,
    Space,
    Tab,
    Other,
}

#[derive(Debug)]
struct LineState {
    col: usize,
    scol: usize,
    init: bool,
    pctype: CharType,
    convert: bool,
}

impl LineState {
    fn new() -> Self {
        Self {
            col: 0,
            scol: 0,
            init: true,
            pctype: CharType::Other,
            convert: true,
        }
    }

    fn reset_line(&mut self) {
        self.col = 0;
        self.scol = 0;
        self.init = true;
        self.pctype = CharType::Other;
        self.convert = true;
    }
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

fn utf8_expected_len(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => 1,
    }
}

fn utf8_incomplete_tail(buf: &[u8]) -> usize {
    if buf.is_empty() {
        return 0;
    }

    let len = buf.len();
    let mut cont = 0usize;
    let mut i = len;

    while i > 0 {
        let b = buf[i - 1];
        if (b & 0b1100_0000) == 0b1000_0000 {
            cont += 1;
            i -= 1;
            if cont == 3 {
                break;
            }
            continue;
        }
        break;
    }

    if cont == 0 {
        let expected = utf8_expected_len(buf[len - 1]);
        return if expected > 1 { 1 } else { 0 };
    }

    if i == 0 {
        return 0;
    }

    let lead = buf[i - 1];
    let expected = utf8_expected_len(lead);
    if expected <= 1 {
        return 0;
    }

    let actual = cont + 1;
    if actual < expected {
        actual
    } else {
        0
    }
}

#[inline]
fn is_space_or_tab(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

#[allow(clippy::cognitive_complexity)]
fn unexpand_chunk(
    buf: &[u8],
    output: &mut BufWriter<Stdout>,
    options: &Options,
    lastcol: usize,
    tab_config: &TabConfig,
    state: &mut LineState,
) -> UResult<()> {
    let mut byte = 0; // offset into the buffer

    while byte < buf.len() {
        if state.init && !options.aflag && !is_space_or_tab(buf[byte]) {
            state.convert = false;
            continue;
        }

        if !state.convert {
            if let Some(pos) = memchr(b'\n', &buf[byte..]) {
                let end = byte + pos + 1;
                output.write_all(&buf[byte..end])?;
                state.reset_line();
                byte = end;
                continue;
            } else {
                output.write_all(&buf[byte..])?;
                return Ok(());
            }
        }

        // when we have a finite number of columns, never convert past the last column
        if lastcol > 0 && state.col >= lastcol {
            write_tabs(
                output,
                tab_config,
                state.scol,
                state.col,
                state.pctype == CharType::Tab,
                state.init,
                true,
            )?;
            state.scol = state.col;
            state.convert = false;
            continue;
        }

        // figure out how big the next char is, if it's UTF-8
        let (ctype, cwidth, nbytes) = next_char_info(options.uflag, buf, byte);
        let is_newline = nbytes == 1 && buf[byte] == b'\n';

        // now figure out how many columns this char takes up, and maybe print it
        let tabs_buffered = state.init || options.aflag;
        match ctype {
            CharType::Space | CharType::Tab => {
                // compute next col, but only write space or tab chars if not buffering
                state.col += if ctype == CharType::Space {
                    1
                } else {
                    next_tabstop(tab_config, state.col).unwrap_or(1)
                };

                if !tabs_buffered {
                    output.write_all(&buf[byte..byte + nbytes])?;
                    state.scol = state.col; // now printed up to this column
                }
            }
            CharType::Other | CharType::Backspace => {
                // always
                write_tabs(
                    output,
                    tab_config,
                    state.scol,
                    state.col,
                    state.pctype == CharType::Tab,
                    state.init,
                    options.aflag,
                )?;
                state.init = false; // no longer at the start of a line
                state.col = if ctype == CharType::Other {
                    // use computed width
                    state.col + cwidth
                } else if state.col > 0 {
                    // Backspace case, but only if col > 0
                    state.col - 1
                } else {
                    0
                };
                output.write_all(&buf[byte..byte + nbytes])?;
                state.scol = state.col; // we've now printed up to this column
                if !options.aflag {
                    state.convert = false;
                }
            }
        }

        byte += nbytes; // move on to next char
        state.pctype = ctype; // save the previous type

        if is_newline {
            state.reset_line();
        }
    }

    Ok(())
}

fn finish_line(
    output: &mut BufWriter<Stdout>,
    tab_config: &TabConfig,
    state: &mut LineState,
) -> UResult<()> {
    if state.convert && state.col != state.scol {
        write_tabs(
            output,
            tab_config,
            state.scol,
            state.col,
            state.pctype == CharType::Tab,
            state.init,
            true,
        )?;
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
    let mut input = open(file)?;
    let mut state = LineState::new();
    let mut pending = [0u8; 4];
    let mut pending_len: usize = 0;
    let mut read_buf = [0u8; READ_BUF_SIZE];

    loop {
        let n = match input.read(&mut read_buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => return Err(e.map_err_context(|| file.maybe_quote().to_string())),
        };

        let slice = &read_buf[..n];
        if options.uflag && pending_len > 0 {
            let head_len = slice.len().min(4);
            let mut small = [0u8; 8];
            small[..pending_len].copy_from_slice(&pending[..pending_len]);
            small[pending_len..pending_len + head_len].copy_from_slice(&slice[..head_len]);
            let small_len = pending_len + head_len;

            let carry_small = utf8_incomplete_tail(&small[..small_len]);
            let process_len = small_len.saturating_sub(carry_small);
            if process_len > 0 {
                unexpand_chunk(
                    &small[..process_len],
                    output,
                    options,
                    lastcol,
                    tab_config,
                    &mut state,
                )?;
            }

            if carry_small > 0 {
                let start = small_len - carry_small;
                pending_len = carry_small;
                pending[..pending_len].copy_from_slice(&small[start..small_len]);
                continue;
            }

            pending_len = 0;
            if head_len >= slice.len() {
                continue;
            }

            let rest = &slice[head_len..];
            let carry = utf8_incomplete_tail(rest);
            let process_len = rest.len().saturating_sub(carry);
            if process_len > 0 {
                unexpand_chunk(
                    &rest[..process_len],
                    output,
                    options,
                    lastcol,
                    tab_config,
                    &mut state,
                )?;
            }
            if carry > 0 {
                pending_len = carry;
                pending[..pending_len].copy_from_slice(&rest[process_len..]);
            }
            continue;
        }

        let carry = if options.uflag {
            utf8_incomplete_tail(slice)
        } else {
            0
        };
        let process_len = slice.len().saturating_sub(carry);
        if process_len > 0 {
            unexpand_chunk(
                &slice[..process_len],
                output,
                options,
                lastcol,
                tab_config,
                &mut state,
            )?;
        }
        if carry > 0 {
            pending_len = carry;
            pending[..pending_len].copy_from_slice(&slice[process_len..]);
        }
    }

    if pending_len > 0 {
        unexpand_chunk(&pending[..pending_len], output, options, lastcol, tab_config, &mut state)?;
    }

    finish_line(output, tab_config, &mut state)?;
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
