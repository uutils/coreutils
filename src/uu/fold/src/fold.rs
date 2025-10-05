// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDOs) ncount routput

use clap::{Arg, ArgAction, Command};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write, stdin, stdout};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::format_usage;
use uucore::translate;

const TAB_WIDTH: usize = 8;
const NL: u8 = b'\n';
const CR: u8 = b'\r';
const TAB: u8 = b'\t';

mod options {
    pub const BYTES: &str = "bytes";
    pub const SPACES: &str = "spaces";
    pub const WIDTH: &str = "width";
    pub const FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_lossy();

    let (args, obs_width) = handle_obsolete(&args[..]);
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let bytes = matches.get_flag(options::BYTES);
    let spaces = matches.get_flag(options::SPACES);
    let poss_width = match matches.get_one::<String>(options::WIDTH) {
        Some(v) => Some(v.clone()),
        None => obs_width,
    };

    let width = match poss_width {
        Some(inp_width) => inp_width.parse::<usize>().map_err(|e| {
            USimpleError::new(
                1,
                translate!("fold-error-illegal-width", "width" => inp_width.quote(), "error" => e),
            )
        })?,
        None => 80,
    };

    let files = match matches.get_many::<String>(options::FILE) {
        Some(v) => v.cloned().collect(),
        None => vec!["-".to_owned()],
    };

    fold(&files, bytes, spaces, width)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("fold-usage")))
        .about(translate!("fold-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .long(options::BYTES)
                .short('b')
                .help(translate!("fold-bytes-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SPACES)
                .long(options::SPACES)
                .short('s')
                .help(translate!("fold-spaces-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .long(options::WIDTH)
                .short('w')
                .help(translate!("fold-width-help"))
                .value_name("WIDTH")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn handle_obsolete(args: &[String]) -> (Vec<String>, Option<String>) {
    for (i, arg) in args.iter().enumerate() {
        let slice = &arg;
        if slice.starts_with('-') && slice.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) {
            let mut v = args.to_vec();
            v.remove(i);
            return (v, Some(slice[1..].to_owned()));
        }
    }
    (args.to_vec(), None)
}

fn fold(filenames: &[String], bytes: bool, spaces: bool, width: usize) -> UResult<()> {
    let mut output = BufWriter::new(stdout());

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
            fold_file_bytewise(buffer, spaces, width, &mut output)?;
        } else {
            fold_file(buffer, spaces, width, &mut output)?;
        }
    }

    output
        .flush()
        .map_err_context(|| translate!("fold-error-failed-to-write"))?;
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
fn fold_file_bytewise<T: Read, W: Write>(
    mut file: BufReader<T>,
    spaces: bool,
    width: usize,
    output: &mut W,
) -> UResult<()> {
    let mut line = Vec::new();

    loop {
        if file
            .read_until(NL, &mut line)
            .map_err_context(|| translate!("fold-error-readline"))?
            == 0
        {
            break;
        }

        if line == [NL] {
            output.write_all(&[NL])?;
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
                    match slice
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, c)| c.is_ascii_whitespace() && **c != CR)
                    {
                        Some((m, _)) => &slice[..=m],
                        None => slice,
                    }
                } else {
                    slice
                }
            };

            // Don't duplicate trailing newlines: if the slice is "\n", the
            // previous iteration folded just before the end of the line and
            // has already printed this newline.
            if slice == [NL] {
                break;
            }

            i += slice.len();

            let at_eol = i >= len;

            if at_eol {
                output.write_all(slice)?;
            } else {
                output.write_all(slice)?;
                output.write_all(&[NL])?;
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
#[allow(clippy::cognitive_complexity)]
fn fold_file<T: Read, W: Write>(
    mut file: BufReader<T>,
    spaces: bool,
    width: usize,
    writer: &mut W,
) -> UResult<()> {
    let mut line = Vec::new();
    let mut output = Vec::new();
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

            writer.write_all(&output[..consume])?;
            writer.write_all(&[NL])?;
            output.drain(..consume);

            // we know there are no tabs left in output, so each char counts
            // as 1 column
            col_count = output.len();

            last_space = None;
        };
    }

    loop {
        if file
            .read_until(NL, &mut line)
            .map_err_context(|| translate!("fold-error-readline"))?
            == 0
        {
            break;
        }

        for ch in &line {
            if *ch == NL {
                // make sure to _not_ split output at whitespace, since we
                // know the entire output will fit
                last_space = None;
                emit_output!();
                break;
            }

            if col_count >= width {
                emit_output!();
            }

            match *ch {
                CR => col_count = 0,
                TAB => {
                    let next_tab_stop = col_count + TAB_WIDTH - col_count % TAB_WIDTH;

                    if next_tab_stop > width && !output.is_empty() {
                        emit_output!();
                    }

                    col_count = next_tab_stop;
                    last_space = if spaces { Some(output.len()) } else { None };
                }
                0x08 => {
                    col_count = col_count.saturating_sub(1);
                }
                _ if spaces && ch.is_ascii_whitespace() => {
                    last_space = Some(output.len());
                    col_count += 1;
                }
                _ => col_count += 1,
            }

            output.push(*ch);
        }

        if !output.is_empty() {
            writer.write_all(&output)?;
            output.truncate(0);
        }

        line.truncate(0);
    }

    Ok(())
}
