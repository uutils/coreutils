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
use std::fs::{metadata, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
enum TruncateMode {
    Absolute(u64),
    Extend(u64),
    Reduce(u64),
    AtMost(u64),
    AtLeast(u64),
    RoundDown(u64),
    RoundUp(u64),
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
    fn to_size(&self, fsize: u64) -> u64 {
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
}

static ARG_FILES: &str = "files";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
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

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let long_usage = get_long_usage();

    let matches = App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&long_usage[..])
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
            .help("base the size of each file on the size of RFILE")
            .value_name("RFILE")
        )
        .arg(
            Arg::with_name(options::SIZE)
            .short("s")
            .long("size")
            .help("set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified")
            .value_name("SIZE")
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true).min_values(1))
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        show_error!("Missing an argument");
        return 1;
    } else {
        let io_blocks = matches.is_present(options::IO_BLOCKS);
        let no_create = matches.is_present(options::NO_CREATE);
        let reference = matches.value_of(options::REFERENCE).map(String::from);
        let size = matches.value_of(options::SIZE).map(String::from);
        if let Err(e) = truncate(no_create, io_blocks, reference, size, files) {
            match e.kind() {
                ErrorKind::NotFound => {
                    // TODO Improve error-handling so that the error
                    // returned by `truncate()` provides the necessary
                    // parameter for formatting the error message.
                    let reference = matches.value_of(options::REFERENCE).map(String::from);
                    crash!(
                        1,
                        "cannot stat '{}': No such file or directory",
                        reference.unwrap()
                    );
                }
                _ => crash!(1, "{}", e.to_string()),
            }
        }
    }

    0
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
fn file_truncate(filename: &str, create: bool, size: u64) -> std::io::Result<()> {
    let path = Path::new(filename);
    let f = OpenOptions::new().write(true).create(create).open(path)?;
    f.set_len(size)
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
                crash!(1, "you must specify a relative ‘--size’ with ‘--reference’")
            }
            _ => m,
        },
        Err(_) => crash!(1, "Invalid number: ‘{}’", size_string),
    };
    let fsize = metadata(rfilename)?.len();
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
    let tsize = metadata(rfilename)?.len();
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
        Err(_) => crash!(1, "Invalid number: ‘{}’", size_string),
    };
    for filename in &filenames {
        let fsize = metadata(filename).map(|m| m.len()).unwrap_or(0);
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
        (None, None) => crash!(1, "you must specify either --reference or --size"),
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
fn parse_mode_and_size(size_string: &str) -> Result<TruncateMode, ()> {
    // Trim any whitespace.
    let size_string = size_string.trim();

    // Get the modifier character from the size string, if any. For
    // example, if the argument is "+123", then the modifier is '+'.
    let c = size_string.chars().next().unwrap();
    let size_string = if is_modifier(c) {
        &size_string[1..]
    } else {
        size_string
    };
    parse_size(size_string).map(match c {
        '+' => TruncateMode::Extend,
        '-' => TruncateMode::Reduce,
        '<' => TruncateMode::AtMost,
        '>' => TruncateMode::AtLeast,
        '/' => TruncateMode::RoundDown,
        '%' => TruncateMode::RoundUp,
        _ => TruncateMode::Absolute,
    })
}

/// Parse a size string into a number of bytes.
///
/// A size string comprises an integer and an optional unit. The unit
/// may be K, M, G, T, P, E, Z, or Y (powers of 1024) or KB, MB,
/// etc. (powers of 1000).
///
/// # Errors
///
/// This function returns an error if the string does not begin with a
/// numeral, or if the unit is not one of the supported units described
/// in the preceding section.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_size("123").unwrap(), 123);
/// assert_eq!(parse_size("123K").unwrap(), 123 * 1024);
/// assert_eq!(parse_size("123KB").unwrap(), 123 * 1000);
/// ```
fn parse_size(size: &str) -> Result<u64, ()> {
    // Get the numeric part of the size argument. For example, if the
    // argument is "123K", then the numeric part is "123".
    let numeric_string: String = size.chars().take_while(|c| c.is_digit(10)).collect();
    let number: u64 = numeric_string.parse().map_err(|_| ())?;

    // Get the alphabetic units part of the size argument and compute
    // the factor it represents. For example, if the argument is "123K",
    // then the unit part is "K" and the factor is 1024. This may be the
    // empty string, in which case, the factor is 1.
    let n = numeric_string.len();
    let (base, exponent): (u64, u32) = match &size[n..] {
        "" => (1, 0),
        "K" | "k" => (1024, 1),
        "M" | "m" => (1024, 2),
        "G" | "g" => (1024, 3),
        "T" | "t" => (1024, 4),
        "P" | "p" => (1024, 5),
        "E" | "e" => (1024, 6),
        "Z" | "z" => (1024, 7),
        "Y" | "y" => (1024, 8),
        "KB" | "kB" => (1000, 1),
        "MB" | "mB" => (1000, 2),
        "GB" | "gB" => (1000, 3),
        "TB" | "tB" => (1000, 4),
        "PB" | "pB" => (1000, 5),
        "EB" | "eB" => (1000, 6),
        "ZB" | "zB" => (1000, 7),
        "YB" | "yB" => (1000, 8),
        _ => return Err(()),
    };
    let factor = base.pow(exponent);
    Ok(number * factor)
}

#[cfg(test)]
mod tests {
    use crate::parse_mode_and_size;
    use crate::parse_size;
    use crate::TruncateMode;

    #[test]
    fn test_parse_size_zero() {
        assert_eq!(parse_size("0").unwrap(), 0);
        assert_eq!(parse_size("0K").unwrap(), 0);
        assert_eq!(parse_size("0KB").unwrap(), 0);
    }

    #[test]
    fn test_parse_size_without_factor() {
        assert_eq!(parse_size("123").unwrap(), 123);
    }

    #[test]
    fn test_parse_size_kilobytes() {
        assert_eq!(parse_size("123K").unwrap(), 123 * 1024);
        assert_eq!(parse_size("123KB").unwrap(), 123 * 1000);
    }

    #[test]
    fn test_parse_size_megabytes() {
        assert_eq!(parse_size("123").unwrap(), 123);
        assert_eq!(parse_size("123M").unwrap(), 123 * 1024 * 1024);
        assert_eq!(parse_size("123MB").unwrap(), 123 * 1000 * 1000);
    }

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
