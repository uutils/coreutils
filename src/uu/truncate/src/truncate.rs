// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) RFILE refsize rfilename fsize tsize
use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::{metadata, OpenOptions};
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::parse_size::{parse_size_u64, ParseSizeError};
use uucore::{format_usage, help_about, help_section, help_usage};

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
    /// If the mode is [`TruncateMode::Reduce`] and the value to
    /// reduce by is greater than `fsize`, then this function returns
    /// 0 (since it cannot return a negative number).
    ///
    /// # Examples
    ///
    /// Extending a file of 10 bytes by 5 bytes:
    ///
    /// ```rust,ignore
    /// let mode = TruncateMode::Extend(5);
    /// let fsize = 10;
    /// assert_eq!(mode.to_size(fsize), 15);
    /// ```
    ///
    /// Reducing a file by more than its size results in 0:
    ///
    /// ```rust,ignore
    /// let mode = TruncateMode::Reduce(5);
    /// let fsize = 3;
    /// assert_eq!(mode.to_size(fsize), 0);
    /// ```
    fn to_size(&self, fsize: u64) -> u64 {
        match self {
            Self::Absolute(size) => *size,
            Self::Extend(size) => fsize + size,
            Self::Reduce(size) => {
                if *size > fsize {
                    0
                } else {
                    fsize - size
                }
            }
            Self::AtMost(size) => fsize.min(*size),
            Self::AtLeast(size) => fsize.max(*size),
            Self::RoundDown(size) => fsize - fsize % size,
            Self::RoundUp(size) => fsize + fsize % size,
        }
    }
}

const ABOUT: &str = help_about!("truncate.md");
const AFTER_HELP: &str = help_section!("after help", "truncate.md");
const USAGE: &str = help_usage!("truncate.md");

pub mod options {
    pub static IO_BLOCKS: &str = "io-blocks";
    pub static NO_CREATE: &str = "no-create";
    pub static REFERENCE: &str = "reference";
    pub static SIZE: &str = "size";
    pub static ARG_FILES: &str = "files";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(AFTER_HELP)
        .try_get_matches_from(args)
        .map_err(|e| {
            e.print().expect("Error writing clap::Error");
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => 0,
                _ => 1,
            }
        })?;

    let files: Vec<String> = matches
        .get_many::<String>(options::ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if files.is_empty() {
        Err(UUsageError::new(1, "missing file operand"))
    } else {
        let io_blocks = matches.get_flag(options::IO_BLOCKS);
        let no_create = matches.get_flag(options::NO_CREATE);
        let reference = matches
            .get_one::<String>(options::REFERENCE)
            .map(String::from);
        let size = matches.get_one::<String>(options::SIZE).map(String::from);
        truncate(no_create, io_blocks, reference, size, &files)
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::IO_BLOCKS)
                .short('o')
                .long(options::IO_BLOCKS)
                .help(
                    "treat SIZE as the number of I/O blocks of the file rather than bytes \
            (NOT IMPLEMENTED)",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CREATE)
                .short('c')
                .long(options::NO_CREATE)
                .help("do not create files that do not exist")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .short('r')
                .long(options::REFERENCE)
                .required_unless_present(options::SIZE)
                .help("base the size of each file on the size of RFILE")
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::SIZE)
                .short('s')
                .long(options::SIZE)
                .required_unless_present(options::REFERENCE)
                .help(
                    "set or adjust the size of each file according to SIZE, which is in \
            bytes unless --io-blocks is specified",
                )
                .value_name("SIZE"),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .value_name("FILE")
                .action(ArgAction::Append)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
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
fn file_truncate(filename: &str, create: bool, size: u64) -> UResult<()> {
    #[cfg(unix)]
    if let Ok(metadata) = std::fs::metadata(filename) {
        if metadata.file_type().is_fifo() {
            return Err(USimpleError::new(
                1,
                format!(
                    "cannot open {} for writing: No such device or address",
                    filename.quote()
                ),
            ));
        }
    }
    let path = Path::new(filename);
    match OpenOptions::new().write(true).create(create).open(path) {
        Ok(file) => file.set_len(size),
        Err(e) if e.kind() == ErrorKind::NotFound && !create => Ok(()),
        Err(e) => Err(e),
    }
    .map_err_context(|| format!("cannot open {} for writing", filename.quote()))
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
/// If any file could not be opened, or there was a problem setting
/// the size of at least one file.
///
/// If at least one file is a named pipe (also known as a fifo).
fn truncate_reference_and_size(
    rfilename: &str,
    size_string: &str,
    filenames: &[String],
    create: bool,
) -> UResult<()> {
    let mode = match parse_mode_and_size(size_string) {
        Err(e) => return Err(USimpleError::new(1, format!("Invalid number: {e}"))),
        Ok(TruncateMode::Absolute(_)) => {
            return Err(USimpleError::new(
                1,
                String::from("you must specify a relative '--size' with '--reference'"),
            ))
        }
        Ok(m) => m,
    };
    if let TruncateMode::RoundDown(0) | TruncateMode::RoundUp(0) = mode {
        return Err(USimpleError::new(1, "division by zero"));
    }
    let metadata = metadata(rfilename).map_err(|e| match e.kind() {
        ErrorKind::NotFound => USimpleError::new(
            1,
            format!(
                "cannot stat {}: No such file or directory",
                rfilename.quote()
            ),
        ),
        _ => e.map_err_context(String::new),
    })?;
    let fsize = metadata.len();
    let tsize = mode.to_size(fsize);
    for filename in filenames {
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
/// If any file could not be opened, or there was a problem setting
/// the size of at least one file.
///
/// If at least one file is a named pipe (also known as a fifo).
fn truncate_reference_file_only(
    rfilename: &str,
    filenames: &[String],
    create: bool,
) -> UResult<()> {
    let metadata = metadata(rfilename).map_err(|e| match e.kind() {
        ErrorKind::NotFound => USimpleError::new(
            1,
            format!(
                "cannot stat {}: No such file or directory",
                rfilename.quote()
            ),
        ),
        _ => e.map_err_context(String::new),
    })?;
    let tsize = metadata.len();
    for filename in filenames {
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
/// If any file could not be opened, or there was a problem setting
/// the size of at least one file.
///
/// If at least one file is a named pipe (also known as a fifo).
fn truncate_size_only(size_string: &str, filenames: &[String], create: bool) -> UResult<()> {
    let mode = parse_mode_and_size(size_string)
        .map_err(|e| USimpleError::new(1, format!("Invalid number: {e}")))?;
    if let TruncateMode::RoundDown(0) | TruncateMode::RoundUp(0) = mode {
        return Err(USimpleError::new(1, "division by zero"));
    }
    for filename in filenames {
        let fsize = match metadata(filename) {
            Ok(m) => {
                #[cfg(unix)]
                if m.file_type().is_fifo() {
                    return Err(USimpleError::new(
                        1,
                        format!(
                            "cannot open {} for writing: No such device or address",
                            filename.quote()
                        ),
                    ));
                }
                m.len()
            }
            Err(_) => 0,
        };
        let tsize = mode.to_size(fsize);
        // TODO: Fix duplicate call to stat
        file_truncate(filename, create, tsize)?;
    }
    Ok(())
}

fn truncate(
    no_create: bool,
    _: bool,
    reference: Option<String>,
    size: Option<String>,
    filenames: &[String],
) -> UResult<()> {
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
/// A size string is as described in [`parse_size_u64`]. The first character
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
        parse_size_u64(size_string).map(match c {
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

    #[test]
    fn test_to_size() {
        assert_eq!(TruncateMode::Extend(5).to_size(10), 15);
        assert_eq!(TruncateMode::Reduce(5).to_size(10), 5);
        assert_eq!(TruncateMode::Reduce(5).to_size(3), 0);
    }
}
