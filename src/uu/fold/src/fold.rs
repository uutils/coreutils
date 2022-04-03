//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDOs) ncount routput

use clap::{crate_version, Arg, Command};
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::{format_usage, InvalidEncodingHandling};

const TAB_WIDTH: usize = 8;

static NAME: &str = "fold";
static USAGE: &str = "{} [OPTION]... [FILE]...";
static SUMMARY: &str = "Writes each file (or standard input if no files are given)
 to standard output whilst breaking long lines";

mod options {
    pub const BYTES: &str = "bytes";
    pub const SPACES: &str = "spaces";
    pub const WIDTH: &str = "width";
    pub const FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let (args, obs_width) = handle_obsolete(&args[..]);
    let matches = uu_app().get_matches_from(args);

    let bytes = matches.is_present(options::BYTES);
    let spaces = matches.is_present(options::SPACES);
    let poss_width = match matches.value_of(options::WIDTH) {
        Some(v) => Some(v.to_owned()),
        None => obs_width,
    };

    let width = match poss_width {
        Some(inp_width) => inp_width.parse::<usize>().map_err(|e| {
            USimpleError::new(
                1,
                format!("illegal width value ({}): {}", inp_width.quote(), e),
            )
        })?,
        None => 80,
    };

    let files = match matches.values_of(options::FILE) {
        Some(v) => v.map(|v| v.to_owned()).collect(),
        None => vec!["-".to_owned()],
    };

    fold(&files, bytes, spaces, width)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .long(options::BYTES)
                .short('b')
                .help(
                    "count using bytes rather than columns (meaning control characters \
                     such as newline are not treated specially)",
                )
                .takes_value(false),
        )
        .arg(
            Arg::new(options::SPACES)
                .long(options::SPACES)
                .short('s')
                .help("break lines at word boundaries rather than a hard cut-off")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::WIDTH)
                .long(options::WIDTH)
                .short('w')
                .help("set WIDTH as the maximum line width rather than 80")
                .value_name("WIDTH")
                .allow_hyphen_values(true)
                .takes_value(true),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .multiple_occurrences(true),
        )
}

fn handle_obsolete(args: &[String]) -> (Vec<String>, Option<String>) {
    for (i, arg) in args.iter().enumerate() {
        let slice = &arg;
        if slice.starts_with('-') && slice.chars().nth(1).map_or(false, |c| c.is_digit(10)) {
            let mut v = args.to_vec();
            v.remove(i);
            return (v, Some(slice[1..].to_owned()));
        }
    }
    (args.to_vec(), None)
}

fn fold(filenames: &[String], bytes: bool, spaces: bool, width: usize) -> UResult<()> {
    for filename in filenames {
        let filename: &str = filename;
        let mut stdin_buf;
        let mut file_buf;
        let buffer = BufReader::new(if filename == "-" {
            stdin_buf = stdin();
            &mut stdin_buf as &mut dyn Read
        } else {
            file_buf = File::open(Path::new(filename)).map_err_context(|| filename.to_string())?;
            &mut file_buf as &mut dyn Read
        });

        if bytes {
            fold_file_bytewise(buffer, spaces, width)?;
        } else {
            fold_file(buffer, spaces, width)?;
        }
    }
    Ok(())
}

/// Fold `file` to fit `width` (number of columns), counting all characters as
/// one column.
///
/// This function handles folding for the `-b`/`--bytes` option, counting
/// tab, backspace, and carriage return as occupying one column, identically
/// to all other characters in the stream.
///
///  If `spaces` is `true`, attempt to break lines at whitespace boundaries.
fn fold_file_bytewise<T: Read>(mut file: BufReader<T>, spaces: bool, width: usize) -> UResult<()> {
    let mut line = String::new();

    loop {
        if file
            .read_line(&mut line)
            .map_err_context(|| "failed to read line".to_string())?
            == 0
        {
            break;
        }

        if line == "\n" {
            println!();
            line.truncate(0);
            continue;
        }

        let len = line.len();
        let mut i = 0;

        while i < len {
            let width = if len - i >= width { width } else { len - i };
            let slice = {
                let slice = &line[i..i + width];
                if spaces && i + width < len {
                    match slice.rfind(|c: char| c.is_whitespace() && c != '\r') {
                        Some(m) => &slice[..=m],
                        None => slice,
                    }
                } else {
                    slice
                }
            };

            // Don't duplicate trailing newlines: if the slice is "\n", the
            // previous iteration folded just before the end of the line and
            // has already printed this newline.
            if slice == "\n" {
                break;
            }

            i += slice.len();

            let at_eol = i >= len;

            if at_eol {
                print!("{}", slice);
            } else {
                println!("{}", slice);
            }
        }

        line.truncate(0);
    }

    Ok(())
}

/// Fold `file` to fit `width` (number of columns).
///
/// By default `fold` treats tab, backspace, and carriage return specially:
/// tab characters count as 8 columns, backspace decreases the
/// column count, and carriage return resets the column count to 0.
///
/// If `spaces` is `true`, attempt to break lines at whitespace boundaries.
#[allow(unused_assignments)]
fn fold_file<T: Read>(mut file: BufReader<T>, spaces: bool, width: usize) -> UResult<()> {
    let mut line = String::new();
    let mut output = String::new();
    let mut col_count = 0;
    let mut last_space = None;

    /// Print the output line, resetting the column and character counts.
    ///
    /// If `spaces` is `true`, print the output line up to the last
    /// encountered whitespace character (inclusive) and set the remaining
    /// characters as the start of the next line.
    macro_rules! emit_output {
        () => {
            let consume = match last_space {
                Some(i) => i + 1,
                None => output.len(),
            };

            println!("{}", &output[..consume]);
            output.replace_range(..consume, "");

            // we know there are no tabs left in output, so each char counts
            // as 1 column
            col_count = output.len();

            last_space = None;
        };
    }

    loop {
        if file
            .read_line(&mut line)
            .map_err_context(|| "failed to read line".to_string())?
            == 0
        {
            break;
        }

        for ch in line.chars() {
            if ch == '\n' {
                // make sure to _not_ split output at whitespace, since we
                // know the entire output will fit
                last_space = None;
                emit_output!();
                break;
            }

            if col_count >= width {
                emit_output!();
            }

            match ch {
                '\r' => col_count = 0,
                '\t' => {
                    let next_tab_stop = col_count + TAB_WIDTH - col_count % TAB_WIDTH;

                    if next_tab_stop > width && !output.is_empty() {
                        emit_output!();
                    }

                    col_count = next_tab_stop;
                    last_space = if spaces { Some(output.len()) } else { None };
                }
                '\x08' => {
                    if col_count > 0 {
                        col_count -= 1;
                    }
                }
                _ if spaces && ch.is_whitespace() => {
                    last_space = Some(output.len());
                    col_count += 1;
                }
                _ => col_count += 1,
            };

            output.push(ch);
        }

        if !output.is_empty() {
            print!("{}", output);
            output.truncate(0);
        }

        line.truncate(0);
    }

    Ok(())
}
