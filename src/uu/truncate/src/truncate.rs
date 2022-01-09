//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) RFILE refsize rfilename fsize tsize

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::convert::TryFrom;
use std::fs::{metadata, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{UIoError, UResult, USimpleError, UUsageError};
use uucore::parse_size::{parse_size, ParseSizeError};

#[derive(Debug, Eq, PartialEq)]
enum TruncateMode {
    Absolute(usize),
    Extend(usize),
    Reduce(usize),
    AtMost(usize),
    AtLeast(usize),
    RoundDown(usize),
    RoundUp(usize),
}

impl TruncateMode {
    /// Compute a target size in bytes for this truncate mode.
    ///
    /// `fsize` is the size of the reference file, in bytes.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mode = TruncateMode::Extend(5);
    /// let fsize = 10;
    /// assert_eq!(mode.to_size(fsize), 15);
    /// ```
    fn to_size(&self, fsize: usize) -> usize {
        match self {
            TruncateMode::Absolute(size) => *size,
            TruncateMode::Extend(size) => fsize + size,
            TruncateMode::Reduce(size) => fsize - size,
            TruncateMode::AtMost(size) => fsize.min(*size),
            TruncateMode::AtLeast(size) => fsize.max(*size),
            TruncateMode::RoundDown(size) => fsize - fsize % size,
            TruncateMode::RoundUp(size) => fsize + fsize % size,
        }
    }
}

static ABOUT: &str = "Shrink or extend the size of each file to the specified size.";

pub mod options {
    pub static IO_BLOCKS: &str = "io-blocks";
    pub static NO_CREATE: &str = "no-create";
    pub static REFERENCE: &str = "reference";
    pub static SIZE: &str = "size";
    pub static ARG_FILES: &str = "files";
}

fn usage() -> String {
    format!("{0} [OPTION]... [FILE]...", uucore::execution_phrase())
}

fn get_long_usage() -> String {
    String::from(
        "
    SIZE is an integer with an optional prefix and optional unit.
    The available units (K, M, G, T, P, E, Z, and Y) use the following format:
        'KB' =>           1000 (kilobytes)
        'K'  =>           1024 (kibibytes)
        'MB' =>      1000*1000 (megabytes)
        'M'  =>      1024*1024 (mebibytes)
        'GB' => 1000*1000*1000 (gigabytes)
        'G'  => 1024*1024*1024 (gibibytes)
    SIZE may also be prefixed by one of the following to adjust the size of each
    file based on its current size:
        '+'  => extend by
        '-'  => reduce by
        '<'  => at most
        '>'  => at least
        '/'  => round down to multiple of
        '%'  => round up to multiple of",
    )
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();
    let long_usage = get_long_usage();

    let matches = uu_app()
        .usage(&usage[..])
        .after_help(&long_usage[..])
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(options::ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        return Err(UUsageError::new(1, "missing file operand"));
    } else {
        let io_blocks = matches.is_present(options::IO_BLOCKS);
        let no_create = matches.is_present(options::NO_CREATE);
        let reference = matches.value_of(options::REFERENCE).map(String::from);
        let size = matches.value_of(options::SIZE).map(String::from);
        truncate(no_create, io_blocks, reference, size, files).map_err(|e| {
            match e.kind() {
                ErrorKind::NotFound => {
                    // TODO Improve error-handling so that the error
                    // returned by `truncate()` provides the necessary
                    // parameter for formatting the error message.
                    let reference = matches.value_of(options::REFERENCE).map(String::from);
                    USimpleError::new(
                        1,
                        format!(
                            "cannot stat {}: No such file or directory",
                            reference.as_deref().unwrap_or("").quote()
                        ),
                    ) // TODO: fix '--no-create' see test_reference and test_truncate_bytes_size
                }
                _ => uio_error!(e, ""),
            }
        })
    }
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::IO_BLOCKS)
            .short("o")
            .long(options::IO_BLOCKS)
            .help("treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)")
        )
        .arg(
            Arg::with_name(options::NO_CREATE)
            .short("c")
            .long(options::NO_CREATE)
            .help("do not create files that do not exist")
        )
        .arg(
            Arg::with_name(options::REFERENCE)
            .short("r")
            .long(options::REFERENCE)
            .required_unless(options::SIZE)
            .help("base the size of each file on the size of RFILE")
            .value_name("RFILE")
        )
        .arg(
            Arg::with_name(options::SIZE)
            .short("s")
            .long(options::SIZE)
            .required_unless(options::REFERENCE)
            .help("set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified")
            .value_name("SIZE")
        )
        .arg(Arg::with_name(options::ARG_FILES)
             .value_name("FILE")
             .multiple(true)
             .takes_value(true)
             .required(true)
             .min_values(1))
}

/// Truncate the named file to the specified size.
///
/// If `create` is true, then the file will be created if it does not
/// already exist. If `size` is larger than the number of bytes in the
/// file, then the file will be padded with zeros. If `size` is smaller
/// than the number of bytes in the file, then the file will be
/// truncated and any bytes beyond `size` will be lost.
///
/// # Errors
///
/// If the file could not be opened, or there was a problem setting the
/// size of the file.
fn file_truncate(filename: &str, create: bool, size: usize) -> std::io::Result<()> {
    let path = Path::new(filename);
    let f = OpenOptions::new().write(true).create(create).open(path)?;
    f.set_len(u64::try_from(size).unwrap())
}

/// Truncate files to a size relative to a given file.
///
/// `rfilename` is the name of the reference file.
///
/// `size_string` gives the size relative to the reference file to which
/// to set the target files. For example, "+3K" means "set each file to
/// be three kilobytes larger than the size of the reference file".
///
/// If `create` is true, then each file will be created if it does not
/// already exist.
///
/// # Errors
///
/// If the any file could not be opened, or there was a problem setting
/// the size of at least one file.
fn truncate_reference_and_size(
    rfilename: &str,
    size_string: &str,
    filenames: Vec<String>,
    create: bool,
) -> std::io::Result<()> {
    let mode = match parse_mode_and_size(size_string) {
        Ok(m) => match m {
            TruncateMode::Absolute(_) => {
                crash!(1, "you must specify a relative '--size' with '--reference'")
            }
            _ => m,
        },
        Err(e) => crash!(1, "Invalid number: {}", e.to_string()),
    };
    let fsize = usize::try_from(metadata(rfilename)?.len()).unwrap();
    let tsize = mode.to_size(fsize);
    for filename in &filenames {
        file_truncate(filename, create, tsize)?;
    }
    Ok(())
}

/// Truncate files to match the size of a given reference file.
///
/// `rfilename` is the name of the reference file.
///
/// If `create` is true, then each file will be created if it does not
/// already exist.
///
/// # Errors
///
/// If the any file could not be opened, or there was a problem setting
/// the size of at least one file.
fn truncate_reference_file_only(
    rfilename: &str,
    filenames: Vec<String>,
    create: bool,
) -> std::io::Result<()> {
    let tsize = usize::try_from(metadata(rfilename)?.len()).unwrap();
    for filename in &filenames {
        file_truncate(filename, create, tsize)?;
    }
    Ok(())
}

/// Truncate files to a specified size.
///
/// `size_string` gives either an absolute size or a relative size. A
/// relative size adjusts the size of each file relative to its current
/// size. For example, "3K" means "set each file to be three kilobytes"
/// whereas "+3K" means "set each file to be three kilobytes larger than
/// its current size".
///
/// If `create` is true, then each file will be created if it does not
/// already exist.
///
/// # Errors
///
/// If the any file could not be opened, or there was a problem setting
/// the size of at least one file.
fn truncate_size_only(
    size_string: &str,
    filenames: Vec<String>,
    create: bool,
) -> std::io::Result<()> {
    let mode = match parse_mode_and_size(size_string) {
        Ok(m) => m,
        Err(e) => crash!(1, "Invalid number: {}", e.to_string()),
    };
    for filename in &filenames {
        let fsize = usize::try_from(metadata(filename)?.len()).unwrap();
        let tsize = mode.to_size(fsize);
        file_truncate(filename, create, tsize)?;
    }
    Ok(())
}

fn truncate(
    no_create: bool,
    _: bool,
    reference: Option<String>,
    size: Option<String>,
    filenames: Vec<String>,
) -> std::io::Result<()> {
    let create = !no_create;
    // There are four possibilities
    // - reference file given and size given,
    // - reference file given but no size given,
    // - no reference file given but size given,
    // - no reference file given and no size given,
    match (reference, size) {
        (Some(rfilename), Some(size_string)) => {
            truncate_reference_and_size(&rfilename, &size_string, filenames, create)
        }
        (Some(rfilename), None) => truncate_reference_file_only(&rfilename, filenames, create),
        (None, Some(size_string)) => truncate_size_only(&size_string, filenames, create),
        (None, None) => unreachable!(), // this case cannot happen anymore because it's handled by clap
    }
}

/// Decide whether a character is one of the size modifiers, like '+' or '<'.
fn is_modifier(c: char) -> bool {
    c == '+' || c == '-' || c == '<' || c == '>' || c == '/' || c == '%'
}

/// Parse a size string with optional modifier symbol as its first character.
///
/// A size string is as described in [`parse_size`]. The first character
/// of `size_string` might be a modifier symbol, like `'+'` or
/// `'<'`. The first element of the pair returned by this function
/// indicates which modifier symbol was present, or
/// [`TruncateMode::Absolute`] if none.
///
/// # Panics
///
/// If `size_string` is empty, or if no number could be parsed from the
/// given string (for example, if the string were `"abc"`).
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_mode_and_size("+123"), (TruncateMode::Extend, 123));
/// ```
fn parse_mode_and_size(size_string: &str) -> Result<TruncateMode, ParseSizeError> {
    // Trim any whitespace.
    let mut size_string = size_string.trim();

    // Get the modifier character from the size string, if any. For
    // example, if the argument is "+123", then the modifier is '+'.
    if let Some(c) = size_string.chars().next() {
        if is_modifier(c) {
            size_string = &size_string[1..];
        }
        parse_size(size_string).map(match c {
            '+' => TruncateMode::Extend,
            '-' => TruncateMode::Reduce,
            '<' => TruncateMode::AtMost,
            '>' => TruncateMode::AtLeast,
            '/' => TruncateMode::RoundDown,
            '%' => TruncateMode::RoundUp,
            _ => TruncateMode::Absolute,
        })
    } else {
        Err(ParseSizeError::ParseFailure(size_string.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_mode_and_size;
    use crate::TruncateMode;

    #[test]
    fn test_parse_mode_and_size() {
        assert_eq!(parse_mode_and_size("10"), Ok(TruncateMode::Absolute(10)));
        assert_eq!(parse_mode_and_size("+10"), Ok(TruncateMode::Extend(10)));
        assert_eq!(parse_mode_and_size("-10"), Ok(TruncateMode::Reduce(10)));
        assert_eq!(parse_mode_and_size("<10"), Ok(TruncateMode::AtMost(10)));
        assert_eq!(parse_mode_and_size(">10"), Ok(TruncateMode::AtLeast(10)));
        assert_eq!(parse_mode_and_size("/10"), Ok(TruncateMode::RoundDown(10)));
        assert_eq!(parse_mode_and_size("%10"), Ok(TruncateMode::RoundUp(10)));
    }
}
