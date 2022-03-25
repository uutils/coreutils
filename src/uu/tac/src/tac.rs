//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) sbytes slen dlen memmem memmap Mmap mmap SIGBUS
mod error;

use clap::{crate_version, Arg, Command};
use memchr::memmem;
use memmap2::Mmap;
use std::io::{stdin, stdout, BufWriter, Read, Write};
use std::{
    fs::{read, File},
    path::Path,
};
use uucore::display::Quotable;
use uucore::error::UError;
use uucore::error::UResult;
use uucore::InvalidEncodingHandling;
use uucore::{format_usage, show};

use crate::error::TacError;

static NAME: &str = "tac";
static USAGE: &str = "{} [OPTION]... [FILE]...";
static SUMMARY: &str = "Write each file to standard output, last line first.";

mod options {
    pub static BEFORE: &str = "before";
    pub static REGEX: &str = "regex";
    pub static SEPARATOR: &str = "separator";
    pub static FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let before = matches.is_present(options::BEFORE);
    let regex = matches.is_present(options::REGEX);
    let raw_separator = matches.value_of(options::SEPARATOR).unwrap_or("\n");
    let separator = if raw_separator.is_empty() {
        "\0"
    } else {
        raw_separator
    };

    let files: Vec<&str> = match matches.values_of(options::FILE) {
        Some(v) => v.collect(),
        None => vec!["-"],
    };

    tac(&files, before, regex, separator)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(
            Arg::new(options::BEFORE)
                .short('b')
                .long(options::BEFORE)
                .help("attach the separator before instead of after")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::REGEX)
                .short('r')
                .long(options::REGEX)
                .help("interpret the sequence as a regular expression")
                .takes_value(false),
        )
        .arg(
            Arg::new(options::SEPARATOR)
                .short('s')
                .long(options::SEPARATOR)
                .help("use STRING as the separator instead of newline")
                .takes_value(true),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .multiple_occurrences(true),
        )
}

/// Print lines of a buffer in reverse, with line separator given as a regex.
///
/// `data` contains the bytes of the file.
///
/// `pattern` is the regular expression given as a
/// [`regex::bytes::Regex`] (not a [`regex::Regex`], since the input is
/// given as a slice of bytes). If `before` is `true`, then each match
/// of this pattern in `data` is interpreted as the start of a line. If
/// `before` is `false`, then each match of this pattern is interpreted
/// as the end of a line.
///
/// This function writes each line in `data` to [`std::io::Stdout`] in
/// reverse.
///
/// # Errors
///
/// If there is a problem writing to `stdout`, then this function
/// returns [`std::io::Error`].
fn buffer_tac_regex(
    data: &[u8],
    pattern: &regex::bytes::Regex,
    before: bool,
) -> std::io::Result<()> {
    let out = stdout();
    let mut out = BufWriter::new(out.lock());

    // The index of the line separator for the current line.
    //
    // As we scan through the `data` from right to left, we update this
    // variable each time we find a new line separator. We restrict our
    // regular expression search to only those bytes up to the line
    // separator.
    let mut this_line_end = data.len();

    // The index of the start of the next line in the `data`.
    //
    // As we scan through the `data` from right to left, we update this
    // variable each time we find a new line.
    //
    // If `before` is `true`, then each line starts immediately before
    // the line separator. Otherwise, each line starts immediately after
    // the line separator.
    let mut following_line_start = data.len();

    // Iterate over each byte in the buffer in reverse. When we find a
    // line separator, write the line to stdout.
    //
    // The `before` flag controls whether the line separator appears at
    // the end of the line (as in "abc\ndef\n") or at the beginning of
    // the line (as in "/abc/def").
    for i in (0..data.len()).rev() {
        // Determine if there is a match for `pattern` starting at index
        // `i` in `data`. Only search up to the line ending that was
        // found previously.
        if let Some(match_) = pattern.find_at(&data[..this_line_end], i) {
            // Record this index as the ending of the current line.
            this_line_end = i;

            // The length of the match (that is, the line separator), in bytes.
            let slen = match_.end() - match_.start();

            if before {
                out.write_all(&data[i..following_line_start])?;
                following_line_start = i;
            } else {
                out.write_all(&data[i + slen..following_line_start])?;
                following_line_start = i + slen;
            }
        }
    }

    // After the loop terminates, write whatever bytes are remaining at
    // the beginning of the buffer.
    out.write_all(&data[0..following_line_start])?;
    Ok(())
}

/// Write lines from `data` to stdout in reverse.
///
/// This function writes to [`stdout`] each line appearing in `data`,
/// starting with the last line and ending with the first line. The
/// `separator` parameter defines what characters to use as a line
/// separator.
///
/// If `before` is `false`, then this function assumes that the
/// `separator` appears at the end of each line, as in `"abc\ndef\n"`.
/// If `before` is `true`, then this function assumes that the
/// `separator` appears at the beginning of each line, as in
/// `"/abc/def"`.
fn buffer_tac(data: &[u8], before: bool, separator: &str) -> std::io::Result<()> {
    let out = stdout();
    let mut out = BufWriter::new(out.lock());

    // The number of bytes in the line separator.
    let slen = separator.as_bytes().len();

    // The index of the start of the next line in the `data`.
    //
    // As we scan through the `data` from right to left, we update this
    // variable each time we find a new line.
    //
    // If `before` is `true`, then each line starts immediately before
    // the line separator. Otherwise, each line starts immediately after
    // the line separator.
    let mut following_line_start = data.len();

    // Iterate over each byte in the buffer in reverse. When we find a
    // line separator, write the line to stdout.
    //
    // The `before` flag controls whether the line separator appears at
    // the end of the line (as in "abc\ndef\n") or at the beginning of
    // the line (as in "/abc/def").
    for i in memmem::rfind_iter(data, separator) {
        if before {
            out.write_all(&data[i..following_line_start])?;
            following_line_start = i;
        } else {
            out.write_all(&data[i + slen..following_line_start])?;
            following_line_start = i + slen;
        }
    }

    // After the loop terminates, write whatever bytes are remaining at
    // the beginning of the buffer.
    out.write_all(&data[0..following_line_start])?;
    Ok(())
}

fn tac(filenames: &[&str], before: bool, regex: bool, separator: &str) -> UResult<()> {
    // Compile the regular expression pattern if it is provided.
    let maybe_pattern = if regex {
        match regex::bytes::Regex::new(separator) {
            Ok(p) => Some(p),
            Err(e) => return Err(TacError::InvalidRegex(e).into()),
        }
    } else {
        None
    };

    for &filename in filenames {
        let mmap;
        let buf;

        let data: &[u8] = if filename == "-" {
            if let Some(mmap1) = try_mmap_stdin() {
                mmap = mmap1;
                &mmap
            } else {
                let mut buf1 = Vec::new();
                if let Err(e) = stdin().read_to_end(&mut buf1) {
                    let e: Box<dyn UError> = TacError::ReadError("stdin".to_string(), e).into();
                    show!(e);
                    continue;
                }
                buf = buf1;
                &buf
            }
        } else {
            let path = Path::new(filename);
            if path.is_dir() {
                let e: Box<dyn UError> = TacError::InvalidArgument(String::from(filename)).into();
                show!(e);
                continue;
            }

            if path.metadata().is_err() {
                let e: Box<dyn UError> = TacError::FileNotFound(String::from(filename)).into();
                show!(e);
                continue;
            }

            if let Some(mmap1) = try_mmap_path(path) {
                mmap = mmap1;
                &mmap
            } else {
                match read(path) {
                    Ok(buf1) => {
                        buf = buf1;
                        &buf
                    }
                    Err(e) => {
                        let s = format!("{}", filename.quote());
                        let e: Box<dyn UError> = TacError::ReadError(s.to_string(), e).into();
                        show!(e);
                        continue;
                    }
                }
            }
        };

        // Select the appropriate `tac` algorithm based on whether the
        // separator is given as a regular expression or a fixed string.
        let result = match maybe_pattern {
            Some(ref pattern) => buffer_tac_regex(data, pattern, before),
            None => buffer_tac(data, before, separator),
        };

        // If there is any error in writing the output, terminate immediately.
        if let Err(e) = result {
            return Err(TacError::WriteError(e).into());
        }
    }
    Ok(())
}

fn try_mmap_stdin() -> Option<Mmap> {
    // SAFETY: If the file is truncated while we map it, SIGBUS will be raised
    // and our process will be terminated, thus preventing access of invalid memory.
    unsafe { Mmap::map(&stdin()).ok() }
}

fn try_mmap_path(path: &Path) -> Option<Mmap> {
    let file = File::open(path).ok()?;

    // SAFETY: If the file is truncated while we map it, SIGBUS will be raised
    // and our process will be terminated, thus preventing access of invalid memory.
    let mmap = unsafe { Mmap::map(&file).ok()? };

    Some(mmap)
}
