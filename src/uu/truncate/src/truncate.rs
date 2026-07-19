// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) RFILE fsize

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs::{OpenOptions, metadata};
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::show_if_err;
use uucore::translate;

use uucore::parser::parse_size::{ParseSizeError, Parser, allow_list_with_all_suffixes};

#[derive(Clone, Debug, Eq, PartialEq)]
enum TruncateMode {
    Absolute(u64),
    Extend(u64),
    Reduce(u64),
    AtMost(u64),
    AtLeast(u64),
    RoundDown(u64),
    RoundUp(u64),
}

#[derive(Debug, Eq, PartialEq)]
enum SizeCalculationError {
    DivisionByZero,
    Overflow,
}

impl TruncateMode {
    fn scaled_by(&self, factor: u64) -> Result<Self, SizeCalculationError> {
        let scale = |size: u64| {
            size.checked_mul(factor)
                .ok_or(SizeCalculationError::Overflow)
        };

        Ok(match self {
            Self::Absolute(size) => Self::Absolute(scale(*size)?),
            Self::Extend(size) => Self::Extend(scale(*size)?),
            Self::Reduce(size) => Self::Reduce(scale(*size)?),
            Self::AtMost(size) => Self::AtMost(scale(*size)?),
            Self::AtLeast(size) => Self::AtLeast(scale(*size)?),
            Self::RoundDown(size) => Self::RoundDown(scale(*size)?),
            Self::RoundUp(size) => Self::RoundUp(scale(*size)?),
        })
    }

    fn value(&self) -> u64 {
        match self {
            Self::Absolute(size)
            | Self::Extend(size)
            | Self::Reduce(size)
            | Self::AtMost(size)
            | Self::AtLeast(size)
            | Self::RoundDown(size)
            | Self::RoundUp(size) => *size,
        }
    }

    /// Compute a target size in bytes for this truncate mode.
    ///
    /// `fsize` is the size of the reference file, in bytes.
    ///
    /// If the mode is [`TruncateMode::Reduce`] and the value to
    /// reduce by is greater than `fsize`, then this function returns
    /// 0 (since it cannot return a negative number).
    ///
    /// # Returns
    ///
    /// An error if rounding by 0 or if the target size overflows, else the
    /// target size.
    ///
    /// # Examples
    ///
    /// Extending a file of 10 bytes by 5 bytes:
    ///
    /// ```rust,ignore
    /// let mode = TruncateMode::Extend(5);
    /// let fsize = 10;
    /// assert_eq!(mode.to_size(fsize), Ok(15));
    /// ```
    ///
    /// Reducing a file by more than its size results in 0:
    ///
    /// ```rust,ignore
    /// let mode = TruncateMode::Reduce(5);
    /// let fsize = 3;
    /// assert_eq!(mode.to_size(fsize), Ok(0));
    /// ```
    ///
    /// Rounding a file by 0:
    ///
    /// ```rust,ignore
    /// let mode = TruncateMode::RoundDown(0);
    /// let fsize = 17;
    /// assert_eq!(
    ///     mode.to_size(fsize),
    ///     Err(SizeCalculationError::DivisionByZero),
    /// );
    /// ```
    fn to_size(&self, fsize: u64) -> Result<u64, SizeCalculationError> {
        match self {
            Self::Absolute(size) => Ok(*size),
            Self::Extend(size) => fsize
                .checked_add(*size)
                .ok_or(SizeCalculationError::Overflow),
            Self::Reduce(size) => Ok(fsize.saturating_sub(*size)),
            Self::AtMost(size) => Ok(fsize.min(*size)),
            Self::AtLeast(size) => Ok(fsize.max(*size)),
            Self::RoundDown(size) => fsize
                .checked_rem(*size)
                .map(|remainder| fsize - remainder)
                .ok_or(SizeCalculationError::DivisionByZero),
            Self::RoundUp(0) => Err(SizeCalculationError::DivisionByZero),
            Self::RoundUp(size) => fsize
                .checked_next_multiple_of(*size)
                .ok_or(SizeCalculationError::Overflow),
        }
    }

    /// Determine if mode is absolute
    ///
    /// # Returns
    ///
    /// `true` is self matches Self::Absolute(_), `false` otherwise.
    fn is_absolute(&self) -> bool {
        matches!(self, Self::Absolute(_))
    }
}

pub mod options {
    pub static IO_BLOCKS: &str = "io-blocks";
    pub static NO_CREATE: &str = "no-create";
    pub static REFERENCE: &str = "reference";
    pub static SIZE: &str = "size";
    pub static ARG_FILES: &str = "files";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let files: Vec<OsString> = matches
        .get_many::<OsString>(options::ARG_FILES)
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    if files.is_empty() {
        return Err(UUsageError::new(
            1,
            translate!("truncate-error-missing-file-operand"),
        ));
    }

    let io_blocks = matches.get_flag(options::IO_BLOCKS);
    let no_create = matches.get_flag(options::NO_CREATE);
    let reference = matches
        .get_one::<String>(options::REFERENCE)
        .map(String::from);
    let size = matches.get_one::<String>(options::SIZE).map(String::from);

    truncate(&files, no_create, io_blocks, reference, size)
}

pub fn uu_app() -> Command {
    let cmd = Command::new("truncate")
        .version(uucore::crate_version!())
        .about(translate!("truncate-about"))
        .override_usage(format_usage(&translate!("truncate-usage")))
        .after_help(translate!("truncate-after-help"))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        .arg(
            Arg::new(options::IO_BLOCKS)
                .short('o')
                .long(options::IO_BLOCKS)
                .requires(options::SIZE)
                .help(translate!("truncate-help-io-blocks"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CREATE)
                .short('c')
                .long(options::NO_CREATE)
                .help(translate!("truncate-help-no-create"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .short('r')
                .long(options::REFERENCE)
                .required_unless_present(options::SIZE)
                .help(translate!("truncate-help-reference"))
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::SIZE)
                .short('s')
                .long(options::SIZE)
                .required_unless_present(options::REFERENCE)
                .help(translate!("truncate-help-size"))
                .allow_hyphen_values(true)
                .value_name("SIZE"),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .value_name("FILE")
                .action(ArgAction::Append)
                .required(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
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
fn do_file_truncate(filename: &Path, create: bool, size: u64) -> UResult<()> {
    match OpenOptions::new().write(true).create(create).open(filename) {
        Ok(file) => file.set_len(size),
        Err(e) if e.kind() == ErrorKind::NotFound && !create => Ok(()),
        Err(e) => Err(e),
    }
    .map_err_context(
        || translate!("truncate-error-cannot-open-for-writing", "filename" => filename.quote()),
    )
}

/// Block size for file in question, or if file does not yet exist, for the
/// parent directory of the file.
fn io_block_size(path: &Path, metadata: Option<&std::fs::Metadata>) -> u64 {
    use std::os::unix::fs::MetadataExt;
    eprintln!(
        "DEBUG: getting block size {path:?} {metadata:?} {:?}",
        metadata.map(|v| v.blksize())
    );
    metadata.map_or_else(
        || {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            eprintln!("DEBUG: parent is {parent:?}");
            uucore::fs::sane_blksize::sane_blksize_from_path(parent)
        },
        uucore::fs::sane_blksize::sane_blksize_from_metadata,
    )
}

fn file_truncate(
    filename: &OsString,
    no_create: bool,
    io_blocks: bool,
    reference_size: Option<u64>,
    mode: &TruncateMode,
    size_argument: Option<&str>,
) -> UResult<()> {
    let path = Path::new(filename);

    // Get the length of the file.
    let file_metadata = metadata(path);
    let file_size = match file_metadata.as_ref() {
        Ok(metadata) => {
            // A pipe has no length. Do this check here to avoid duplicate `stat()` syscall.
            #[cfg(unix)]
            if metadata.file_type().is_fifo() {
                return Err(USimpleError::new(
                    1,
                    translate!("truncate-error-cannot-open-no-device", "filename" => filename.to_string_lossy().quote()),
                ));
            }
            metadata.len()
        }
        Err(_) => 0,
    };

    // The reference size can be either:
    //
    // 1. The size of a given file
    // 2. The size of the file to be truncated if no reference has been provided.
    let actual_reference_size = reference_size.unwrap_or(file_size);

    let mode = if io_blocks {
        let factor = io_block_size(path, file_metadata.as_ref().ok());
        mode.scaled_by(factor).map_err(|error| match error {
            SizeCalculationError::Overflow => USimpleError::new(
                1,
                translate!(
                    "truncate-error-io-block-mul-overflow",
                    "num" => mode.value(),
                    "factor" => factor
                ),
            ),
            SizeCalculationError::DivisionByZero => unreachable!(),
        })?
    } else {
        mode.clone()
    };

    let truncate_size = mode
        .to_size(actual_reference_size)
        .map_err(|error| match error {
            SizeCalculationError::DivisionByZero => {
                USimpleError::new(1, translate!("truncate-error-division-by-zero"))
            }
            SizeCalculationError::Overflow => {
                let error = match size_argument {
                    None => translate!("truncate-error-value-too-large"),
                    Some(arg) => {
                        translate!("truncate-error-value-too-large-arg", "arg" => arg.quote())
                    }
                };
                USimpleError::new(
                    1,
                    translate!("truncate-error-invalid-number", "error" => error),
                )
            }
        })?;

    do_file_truncate(path, !no_create, truncate_size)
}

fn truncate(
    filenames: &[OsString],
    no_create: bool,
    io_blocks: bool,
    reference: Option<String>,
    size: Option<String>,
) -> UResult<()> {
    let reference_size = match reference {
        Some(reference_path) => {
            let reference_metadata = metadata(&reference_path).map_err(|error| match error.kind() {
                ErrorKind::NotFound => USimpleError::new(
                    1,
                    translate!("truncate-error-cannot-stat-no-such-file", "filename" => reference_path.quote()),
                ),
                _ => error.map_err_context(String::new),
            })?;

            Some(reference_metadata.len())
        }
        None => None,
    };

    let size_string = size.as_deref();

    // Omitting the mode is equivalent to extending a file by 0 bytes.
    let mode = match size_string {
        Some(string) => match parse_mode_and_size(string) {
            Err(error) => {
                return Err(USimpleError::new(
                    1,
                    translate!("truncate-error-invalid-number", "error" => error),
                ));
            }
            Ok(mode) => mode,
        },
        None => TruncateMode::Extend(0),
    };

    // If a reference file has been given, the truncate mode cannot be absolute.
    if reference_size.is_some() && mode.is_absolute() {
        return Err(USimpleError::new(
            1,
            translate!("truncate-error-must-specify-relative-size"),
        ));
    }

    // Process every file: a failure on one (e.g. a directory) must not
    // prevent the remaining files from being truncated.
    for filename in filenames {
        show_if_err!(file_truncate(
            filename,
            no_create,
            io_blocks,
            reference_size,
            &mode,
            size_string,
        ));
    }

    Ok(())
}

/// Decide whether a character is one of the size modifiers, like '+' or '<'.
fn is_modifier(c: char) -> bool {
    "+-<>/%".contains(c)
}

/// Parse a size string with optional modifier symbol as its first character.
///
/// A size string is as described in [`Parser::parse_u64`]. The first character
/// of `size_string` might be a modifier symbol, like `'+'` or
/// `'<'`. The first element of the pair returned by this function
/// indicates which modifier symbol was present, or
/// [`TruncateMode::Absolute`] if none.
///
/// # Panics
///
/// If `size_string` is empty.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_mode_and_size("+123"), Ok(TruncateMode::Extend(123)));
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
        let allow_list = allow_list_with_all_suffixes("EgGkKmMPQRtTYZ");
        let allow_list_ref = allow_list.iter().map(AsRef::as_ref).collect::<Vec<&str>>();
        Parser::default()
            .with_allow_list(&allow_list_ref)
            .parse_u64(size_string)
            .map(match c {
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
    use crate::SizeCalculationError;
    use crate::TruncateMode;
    use crate::parse_mode_and_size;

    #[test]
    fn test_parse_mode_and_size() {
        assert_eq!(parse_mode_and_size("10"), Ok(TruncateMode::Absolute(10)));
        assert_eq!(parse_mode_and_size("+10"), Ok(TruncateMode::Extend(10)));
        assert_eq!(parse_mode_and_size("-10"), Ok(TruncateMode::Reduce(10)));
        assert_eq!(parse_mode_and_size("<10"), Ok(TruncateMode::AtMost(10)));
        assert_eq!(parse_mode_and_size(">10"), Ok(TruncateMode::AtLeast(10)));
        assert_eq!(parse_mode_and_size("/10"), Ok(TruncateMode::RoundDown(10)));
        assert_eq!(parse_mode_and_size("%10"), Ok(TruncateMode::RoundUp(10)));
        assert_eq!(parse_mode_and_size("1kB"), Ok(TruncateMode::Absolute(1000)));
        assert_eq!(parse_mode_and_size("1kD"), Ok(TruncateMode::Absolute(1000)));
        assert!(parse_mode_and_size("1b").is_err());
    }

    #[test]
    fn test_to_size() {
        assert_eq!(TruncateMode::Extend(5).to_size(10), Ok(15));
        assert_eq!(TruncateMode::Reduce(5).to_size(10), Ok(5));
        assert_eq!(TruncateMode::Reduce(5).to_size(3), Ok(0));
        assert_eq!(TruncateMode::RoundDown(4).to_size(13), Ok(12));
        assert_eq!(TruncateMode::RoundDown(4).to_size(16), Ok(16));
        assert_eq!(TruncateMode::RoundUp(8).to_size(10), Ok(16));
        assert_eq!(TruncateMode::RoundUp(8).to_size(16), Ok(16));
        assert_eq!(
            TruncateMode::RoundDown(0).to_size(123),
            Err(SizeCalculationError::DivisionByZero)
        );
        assert_eq!(
            TruncateMode::RoundUp(0).to_size(123),
            Err(SizeCalculationError::DivisionByZero)
        );
        assert_eq!(
            TruncateMode::Extend(u64::MAX).to_size(1),
            Err(SizeCalculationError::Overflow)
        );
        assert_eq!(
            TruncateMode::RoundUp(u64::MAX - 1).to_size(u64::MAX),
            Err(SizeCalculationError::Overflow)
        );
    }

    #[test]
    fn test_scale_mode_by_io_block_size() {
        assert_eq!(
            TruncateMode::Extend(2).scaled_by(4096),
            Ok(TruncateMode::Extend(8192))
        );
        assert_eq!(
            TruncateMode::Absolute(u64::MAX).scaled_by(4096),
            Err(SizeCalculationError::Overflow)
        );
    }

    #[test]
    fn test_round_up_when_file_smaller_than_size() {
        // fsize < size: must round up to size itself
        assert_eq!(TruncateMode::RoundUp(131_072).to_size(24_696), Ok(131_072));
        assert_eq!(TruncateMode::RoundUp(4096).to_size(1), Ok(4096));
        assert_eq!(TruncateMode::RoundUp(100).to_size(50), Ok(100));
    }

    #[test]
    fn test_round_up_already_aligned() {
        assert_eq!(TruncateMode::RoundUp(4096).to_size(0), Ok(0));
        assert_eq!(TruncateMode::RoundUp(4096).to_size(4096), Ok(4096));
        assert_eq!(TruncateMode::RoundUp(4096).to_size(8192), Ok(8192));
    }

    #[test]
    fn test_round_up_not_aligned() {
        // fsize > size but not a multiple: must round up to next multiple
        assert_eq!(TruncateMode::RoundUp(4096).to_size(5000), Ok(8192));
        assert_eq!(TruncateMode::RoundUp(8).to_size(13), Ok(16));
    }
}
