// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nbbbb ncccc hexdigit getmaxstdio

mod filenames;
mod number;
mod platform;
mod strategy;

use crate::filenames::{FilenameIterator, Suffix, SuffixError};
use crate::strategy::{NumberType, Strategy, StrategyError};
use clap::{crate_version, parser::ValueSource, Arg, ArgAction, ArgMatches, Command, ValueHint};
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs::{metadata, File};
use std::io;
use std::io::{stdin, BufRead, BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UIoError, UResult, USimpleError, UUsageError};
use uucore::parse_size::parse_size_u64;

use uucore::uio_error;
use uucore::{format_usage, help_about, help_section, help_usage};

static OPT_BYTES: &str = "bytes";
static OPT_LINE_BYTES: &str = "line-bytes";
static OPT_LINES: &str = "lines";
static OPT_ADDITIONAL_SUFFIX: &str = "additional-suffix";
static OPT_FILTER: &str = "filter";
static OPT_NUMBER: &str = "number";
static OPT_NUMERIC_SUFFIXES: &str = "numeric-suffixes";
static OPT_NUMERIC_SUFFIXES_SHORT: &str = "-d";
static OPT_HEX_SUFFIXES: &str = "hex-suffixes";
static OPT_HEX_SUFFIXES_SHORT: &str = "-x";
static OPT_SUFFIX_LENGTH: &str = "suffix-length";
static OPT_VERBOSE: &str = "verbose";
static OPT_SEPARATOR: &str = "separator";
static OPT_ELIDE_EMPTY_FILES: &str = "elide-empty-files";
static OPT_IO_BLKSIZE: &str = "-io-blksize";

static ARG_INPUT: &str = "input";
static ARG_PREFIX: &str = "prefix";

const ABOUT: &str = help_about!("split.md");
const USAGE: &str = help_usage!("split.md");
const AFTER_HELP: &str = help_section!("after help", "split.md");

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (args, obs_lines) = handle_obsolete(args);
    let matches = uu_app().try_get_matches_from(args)?;

    match Settings::from(&matches, &obs_lines) {
        Ok(settings) => split(&settings),
        Err(e) if e.requires_usage() => Err(UUsageError::new(1, format!("{e}"))),
        Err(e) => Err(USimpleError::new(1, format!("{e}"))),
    }
}

/// Extract obsolete shorthand (if any) for specifying lines in following scenarios (and similar)
/// `split -22 file` would mean `split -l 22 file`
/// `split -2de file` would mean `split -l 2 -d -e file`
/// `split -x300e file` would mean `split -x -l 300 -e file`
/// `split -x300e -22 file` would mean `split -x -e -l 22 file` (last obsolete lines option wins)
/// following GNU `split` behavior
fn handle_obsolete(args: impl uucore::Args) -> (Vec<OsString>, Option<String>) {
    let mut obs_lines = None;
    let mut preceding_long_opt_req_value = false;
    let mut preceding_short_opt_req_value = false;

    let filtered_args = args
        .filter_map(|os_slice| {
            filter_args(
                os_slice,
                &mut obs_lines,
                &mut preceding_long_opt_req_value,
                &mut preceding_short_opt_req_value,
            )
        })
        .collect();

    (filtered_args, obs_lines)
}

/// Helper function to [`handle_obsolete`]
/// Filters out obsolete lines option from args
fn filter_args(
    os_slice: OsString,
    obs_lines: &mut Option<String>,
    preceding_long_opt_req_value: &mut bool,
    preceding_short_opt_req_value: &mut bool,
) -> Option<OsString> {
    let filter: Option<OsString>;
    if let Some(slice) = os_slice.to_str() {
        if should_extract_obs_lines(
            slice,
            preceding_long_opt_req_value,
            preceding_short_opt_req_value,
        ) {
            // start of the short option string
            // that can have obsolete lines option value in it
            filter = handle_extract_obs_lines(slice, obs_lines);
        } else {
            // either not a short option
            // or a short option that cannot have obsolete lines value in it
            filter = Some(OsString::from(slice));
        }
        handle_preceding_options(
            slice,
            preceding_long_opt_req_value,
            preceding_short_opt_req_value,
        );
    } else {
        // Cannot cleanly convert os_slice to UTF-8
        // Do not process and return as-is
        // This will cause failure later on, but we should not handle it here
        // and let clap panic on invalid UTF-8 argument
        filter = Some(os_slice);
    }
    filter
}

/// Helper function to [`filter_args`]
/// Checks if the slice is a true short option (and not hyphen prefixed value of an option)
/// and if so, a short option that can contain obsolete lines value
fn should_extract_obs_lines(
    slice: &str,
    preceding_long_opt_req_value: &bool,
    preceding_short_opt_req_value: &bool,
) -> bool {
    slice.starts_with('-')
        && !slice.starts_with("--")
        && !preceding_long_opt_req_value
        && !preceding_short_opt_req_value
        && !slice.starts_with("-a")
        && !slice.starts_with("-b")
        && !slice.starts_with("-C")
        && !slice.starts_with("-l")
        && !slice.starts_with("-n")
        && !slice.starts_with("-t")
}

/// Helper function to [`filter_args`]
/// Extracts obsolete lines numeric part from argument slice
/// and filters it out
fn handle_extract_obs_lines(slice: &str, obs_lines: &mut Option<String>) -> Option<OsString> {
    let mut obs_lines_extracted: Vec<char> = vec![];
    let mut obs_lines_end_reached = false;
    let filtered_slice: Vec<char> = slice
        .chars()
        .filter(|c| {
            // To correctly process scenario like '-x200a4'
            // we need to stop extracting digits once alphabetic character is encountered
            // after we already have something in obs_lines_extracted
            if c.is_ascii_digit() && !obs_lines_end_reached {
                obs_lines_extracted.push(*c);
                false
            } else {
                if !obs_lines_extracted.is_empty() {
                    obs_lines_end_reached = true;
                }
                true
            }
        })
        .collect();

    if obs_lines_extracted.is_empty() {
        // no obsolete lines value found/extracted
        Some(OsString::from(slice))
    } else {
        // obsolete lines value was extracted
        let extracted: String = obs_lines_extracted.iter().collect();
        *obs_lines = Some(extracted);
        if filtered_slice.get(1).is_some() {
            // there were some short options in front of or after obsolete lines value
            // i.e. '-xd100' or '-100de' or similar, which after extraction of obsolete lines value
            // would look like '-xd' or '-de' or similar
            let filtered_slice: String = filtered_slice.iter().collect();
            Some(OsString::from(filtered_slice))
        } else {
            None
        }
    }
}

/// Helper function to [`handle_extract_obs_lines`]
/// Captures if current slice is a preceding option
/// that requires value
fn handle_preceding_options(
    slice: &str,
    preceding_long_opt_req_value: &mut bool,
    preceding_short_opt_req_value: &mut bool,
) {
    // capture if current slice is a preceding long option that requires value and does not use '=' to assign that value
    // following slice should be treaded as value for this option
    // even if it starts with '-' (which would be treated as hyphen prefixed value)
    if slice.starts_with("--") {
        *preceding_long_opt_req_value = &slice[2..] == OPT_BYTES
            || &slice[2..] == OPT_LINE_BYTES
            || &slice[2..] == OPT_LINES
            || &slice[2..] == OPT_ADDITIONAL_SUFFIX
            || &slice[2..] == OPT_FILTER
            || &slice[2..] == OPT_NUMBER
            || &slice[2..] == OPT_SUFFIX_LENGTH
            || &slice[2..] == OPT_SEPARATOR;
    }
    // capture if current slice is a preceding short option that requires value and does not have value in the same slice (value separated by whitespace)
    // following slice should be treaded as value for this option
    // even if it starts with '-' (which would be treated as hyphen prefixed value)
    *preceding_short_opt_req_value = slice == "-b"
        || slice == "-C"
        || slice == "-l"
        || slice == "-n"
        || slice == "-a"
        || slice == "-t";
    // slice is a value
    // reset preceding option flags
    if !slice.starts_with('-') {
        *preceding_short_opt_req_value = false;
        *preceding_long_opt_req_value = false;
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        // strategy (mutually exclusive)
        .arg(
            Arg::new(OPT_BYTES)
                .short('b')
                .long(OPT_BYTES)
                .allow_hyphen_values(true)
                .value_name("SIZE")
                .help("put SIZE bytes per output file"),
        )
        .arg(
            Arg::new(OPT_LINE_BYTES)
                .short('C')
                .long(OPT_LINE_BYTES)
                .allow_hyphen_values(true)
                .value_name("SIZE")
                .help("put at most SIZE bytes of lines per output file"),
        )
        .arg(
            Arg::new(OPT_LINES)
                .short('l')
                .long(OPT_LINES)
                .allow_hyphen_values(true)
                .value_name("NUMBER")
                .default_value("1000")
                .help("put NUMBER lines/records per output file"),
        )
        .arg(
            Arg::new(OPT_NUMBER)
                .short('n')
                .long(OPT_NUMBER)
                .allow_hyphen_values(true)
                .value_name("CHUNKS")
                .help("generate CHUNKS output files; see explanation below"),
        )
        // rest of the arguments
        .arg(
            Arg::new(OPT_ADDITIONAL_SUFFIX)
                .long(OPT_ADDITIONAL_SUFFIX)
                .allow_hyphen_values(true)
                .value_name("SUFFIX")
                .default_value("")
                .help("additional SUFFIX to append to output file names"),
        )
        .arg(
            Arg::new(OPT_FILTER)
                .long(OPT_FILTER)
                .allow_hyphen_values(true)
                .value_name("COMMAND")
                .value_hint(ValueHint::CommandName)
                .help(
                    "write to shell COMMAND; file name is $FILE (Currently not implemented for Windows)",
                ),
        )
        .arg(
            Arg::new(OPT_ELIDE_EMPTY_FILES)
                .long(OPT_ELIDE_EMPTY_FILES)
                .short('e')
                .help("do not generate empty output files with '-n'")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NUMERIC_SUFFIXES_SHORT)
                .short('d')
                .action(ArgAction::SetTrue)
                .overrides_with_all([
                    OPT_NUMERIC_SUFFIXES,
                    OPT_NUMERIC_SUFFIXES_SHORT,
                    OPT_HEX_SUFFIXES,
                    OPT_HEX_SUFFIXES_SHORT
                ])
                .help("use numeric suffixes starting at 0, not alphabetic"),
        )
        .arg(
            Arg::new(OPT_NUMERIC_SUFFIXES)
                .long(OPT_NUMERIC_SUFFIXES)
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    OPT_NUMERIC_SUFFIXES,
                    OPT_NUMERIC_SUFFIXES_SHORT,
                    OPT_HEX_SUFFIXES,
                    OPT_HEX_SUFFIXES_SHORT
                ])
                .value_name("FROM")
                .help("same as -d, but allow setting the start value"),
        )
        .arg(
            Arg::new(OPT_HEX_SUFFIXES_SHORT)
                .short('x')
                .action(ArgAction::SetTrue)
                .overrides_with_all([
                    OPT_NUMERIC_SUFFIXES,
                    OPT_NUMERIC_SUFFIXES_SHORT,
                    OPT_HEX_SUFFIXES,
                    OPT_HEX_SUFFIXES_SHORT
                ])
                .help("use hex suffixes starting at 0, not alphabetic"),
        )
        .arg(
            Arg::new(OPT_HEX_SUFFIXES)
                .long(OPT_HEX_SUFFIXES)
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    OPT_NUMERIC_SUFFIXES,
                    OPT_NUMERIC_SUFFIXES_SHORT,
                    OPT_HEX_SUFFIXES,
                    OPT_HEX_SUFFIXES_SHORT
                ])
                .value_name("FROM")
                .help("same as -x, but allow setting the start value"),
        )
        .arg(
            Arg::new(OPT_SUFFIX_LENGTH)
                .short('a')
                .long(OPT_SUFFIX_LENGTH)
                .allow_hyphen_values(true)
                .value_name("N")
                .help("generate suffixes of length N (default 2)"),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .long(OPT_VERBOSE)
                .help("print a diagnostic just before each output file is opened")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_SEPARATOR)
                .short('t')
                .long(OPT_SEPARATOR)
                .allow_hyphen_values(true)
                .value_name("SEP")
                .action(ArgAction::Append)
                .help("use SEP instead of newline as the record separator; '\\0' (zero) specifies the NUL character"),
        )
        .arg(
            Arg::new(OPT_IO_BLKSIZE)
                .long("io-blksize")
                .alias(OPT_IO_BLKSIZE)
                .hide(true),
        )
        .arg(
            Arg::new(ARG_INPUT)
                .default_value("-")
                .value_hint(ValueHint::FilePath),
        )
        .arg(
            Arg::new(ARG_PREFIX)
                .default_value("x")
        )
}

/// Parameters that control how a file gets split.
///
/// You can convert an [`ArgMatches`] instance into a [`Settings`]
/// instance by calling [`Settings::from`].
struct Settings {
    prefix: String,
    suffix: Suffix,
    input: String,
    /// When supplied, a shell command to output to instead of xaa, xab …
    filter: Option<String>,
    strategy: Strategy,
    verbose: bool,
    separator: u8,

    /// Whether to *not* produce empty files when using `-n`.
    ///
    /// The `-n` command-line argument gives a specific number of
    /// chunks into which the input files will be split. If the number
    /// of chunks is greater than the number of bytes, and this is
    /// `false`, then empty files will be created for the excess
    /// chunks. If this is `false`, then empty files will not be
    /// created.
    elide_empty_files: bool,
    io_blksize: Option<u64>,
}

/// An error when parsing settings from command-line arguments.
enum SettingsError {
    /// Invalid chunking strategy.
    Strategy(StrategyError),

    /// Invalid suffix length parameter.
    Suffix(SuffixError),

    /// Multi-character (Invalid) separator
    MultiCharacterSeparator(String),

    /// Multiple different separator characters
    MultipleSeparatorCharacters,

    /// Using `--filter` with `--number` option sub-strategies that print Kth chunk out of N chunks to stdout
    /// K/N
    /// l/K/N
    /// r/K/N
    FilterWithKthChunkNumber,

    /// Invalid IO block size
    InvalidIOBlockSize(String),

    /// The `--filter` option is not supported on Windows.
    #[cfg(windows)]
    NotSupported,
}

impl SettingsError {
    /// Whether the error demands a usage message.
    fn requires_usage(&self) -> bool {
        matches!(
            self,
            Self::Strategy(StrategyError::MultipleWays)
                | Self::Suffix(SuffixError::ContainsSeparator(_))
        )
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Strategy(e) => e.fmt(f),
            Self::Suffix(e) => e.fmt(f),
            Self::MultiCharacterSeparator(s) => {
                write!(f, "multi-character separator {}", s.quote())
            }
            Self::MultipleSeparatorCharacters => {
                write!(f, "multiple separator characters specified")
            }
            Self::FilterWithKthChunkNumber => {
                write!(f, "--filter does not process a chunk extracted to stdout")
            }
            Self::InvalidIOBlockSize(s) => write!(f, "invalid IO block size: {}", s.quote()),
            #[cfg(windows)]
            Self::NotSupported => write!(
                f,
                "{OPT_FILTER} is currently not supported in this platform"
            ),
        }
    }
}

impl Settings {
    /// Parse a strategy from the command-line arguments.
    fn from(matches: &ArgMatches, obs_lines: &Option<String>) -> Result<Self, SettingsError> {
        let strategy = Strategy::from(matches, obs_lines).map_err(SettingsError::Strategy)?;
        let suffix = Suffix::from(matches, &strategy).map_err(SettingsError::Suffix)?;

        // Make sure that separator is only one UTF8 character (if specified)
        // defaults to '\n' - newline character
        // If the same separator (the same value) was used multiple times - `split` should NOT fail
        // If the separator was used multiple times but with different values (not all values are the same) - `split` should fail
        let separator = match matches.get_many::<String>(OPT_SEPARATOR) {
            Some(mut sep_values) => {
                let first = sep_values.next().unwrap(); // it is safe to just unwrap here since Clap should not return empty ValuesRef<'_,String> in the option from get_many() call
                if !sep_values.all(|s| s == first) {
                    return Err(SettingsError::MultipleSeparatorCharacters);
                }
                match first.as_str() {
                    "\\0" => b'\0',
                    s if s.as_bytes().len() == 1 => s.as_bytes()[0],
                    s => return Err(SettingsError::MultiCharacterSeparator(s.to_string())),
                }
            }
            None => b'\n',
        };

        let io_blksize: Option<u64> = if let Some(s) = matches.get_one::<String>(OPT_IO_BLKSIZE) {
            match parse_size_u64(s) {
                Ok(0) => return Err(SettingsError::InvalidIOBlockSize(s.to_string())),
                Ok(n) if n <= uucore::fs::sane_blksize::MAX => Some(n),
                _ => return Err(SettingsError::InvalidIOBlockSize(s.to_string())),
            }
        } else {
            None
        };

        let result = Self {
            prefix: matches.get_one::<String>(ARG_PREFIX).unwrap().clone(),
            suffix,
            input: matches.get_one::<String>(ARG_INPUT).unwrap().clone(),
            filter: matches.get_one::<String>(OPT_FILTER).cloned(),
            strategy,
            verbose: matches.value_source(OPT_VERBOSE) == Some(ValueSource::CommandLine),
            separator,
            elide_empty_files: matches.get_flag(OPT_ELIDE_EMPTY_FILES),
            io_blksize,
        };

        #[cfg(windows)]
        if result.filter.is_some() {
            // see https://github.com/rust-lang/rust/issues/29494
            return Err(SettingsError::NotSupported);
        }

        // Return an error if `--filter` option is used with any of the
        // Kth chunk sub-strategies of `--number` option
        // As those are writing to stdout of `split` and cannot write to filter command child process
        let kth_chunk = matches!(
            result.strategy,
            Strategy::Number(NumberType::KthBytes(_, _))
                | Strategy::Number(NumberType::KthLines(_, _))
                | Strategy::Number(NumberType::KthRoundRobin(_, _))
        );
        if kth_chunk && result.filter.is_some() {
            return Err(SettingsError::FilterWithKthChunkNumber);
        }

        Ok(result)
    }

    fn instantiate_current_writer(
        &self,
        filename: &str,
        is_new: bool,
    ) -> io::Result<BufWriter<Box<dyn Write>>> {
        if platform::paths_refer_to_same_file(&self.input, filename) {
            return Err(io::Error::new(
                ErrorKind::Other,
                format!("'{filename}' would overwrite input; aborting"),
            ));
        }

        platform::instantiate_current_writer(&self.filter, filename, is_new)
    }
}

/// When using `--filter` option, writing to child command process stdin
/// could fail with BrokenPipe error
/// It can be safely ignored
fn ignorable_io_error(error: &std::io::Error, settings: &Settings) -> bool {
    error.kind() == ErrorKind::BrokenPipe && settings.filter.is_some()
}

/// Custom wrapper for `write()` method
/// Follows similar approach to GNU implementation
/// If ignorable io error occurs, return number of bytes as if all bytes written
/// Should not be used for Kth chunk number sub-strategies
/// as those do not work with `--filter` option
fn custom_write<T: Write>(
    bytes: &[u8],
    writer: &mut T,
    settings: &Settings,
) -> std::io::Result<usize> {
    match writer.write(bytes) {
        Ok(n) => Ok(n),
        Err(e) if ignorable_io_error(&e, settings) => Ok(bytes.len()),
        Err(e) => Err(e),
    }
}

/// Custom wrapper for `write_all()` method
/// Similar to [`custom_write`], but returns true or false
/// depending on if `--filter` stdin is still open (no BrokenPipe error)
/// Should not be used for Kth chunk number sub-strategies
/// as those do not work with `--filter` option
fn custom_write_all<T: Write>(
    bytes: &[u8],
    writer: &mut T,
    settings: &Settings,
) -> std::io::Result<bool> {
    match writer.write_all(bytes) {
        Ok(()) => Ok(true),
        Err(e) if ignorable_io_error(&e, settings) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Get the size of the input file in bytes
/// Used only for subset of `--number=CHUNKS` strategy, as there is a need
/// to determine input file size upfront in order to estimate the chunk size
/// to be written into each of N files/chunks:
/// * N       split into N files based on size of input
/// * K/N     output Kth of N to stdout
/// * l/N     split into N files without splitting lines/records
/// * l/K/N   output Kth of N to stdout without splitting lines/records
///
/// For most files the size will be determined by either reading entire file content into a buffer
/// or by `len()` function of [`std::fs::metadata`].
///
/// However, for some files which report filesystem metadata size that does not match
/// their actual content size, we will need to attempt to find the end of file
/// with direct `seek()` on [`std::fs::File`].
///
/// For STDIN stream - read into a buffer up to a limit
/// If input stream does not EOF before that - return an error
/// (i.e. "infinite" input as in `cat /dev/zero | split ...`, `yes | split ...` etc.).
///
/// Note: The `buf` might end up with either partial or entire input content.
fn get_input_size<R>(
    input: &String,
    reader: &mut R,
    buf: &mut Vec<u8>,
    io_blksize: &Option<u64>,
) -> std::io::Result<u64>
where
    R: BufRead,
{
    // Set read limit to io_blksize if specified
    let read_limit: u64 = if let Some(custom_blksize) = io_blksize {
        *custom_blksize
    } else {
        // otherwise try to get it from filesystem, or use default
        uucore::fs::sane_blksize::sane_blksize_from_path(Path::new(input))
    };

    // Try to read into buffer up to a limit
    let num_bytes = reader
        .by_ref()
        .take(read_limit)
        .read_to_end(buf)
        .map(|n| n as u64)?;

    if num_bytes < read_limit {
        // Finite file or STDIN stream that fits entirely
        // into a buffer within the limit
        // Note: files like /dev/null or similar,
        // empty STDIN stream,
        // and files with true file size 0
        // will also fit here
        Ok(num_bytes)
    } else if input == "-" {
        // STDIN stream that did not fit all content into a buffer
        // Most likely continuous/infinite input stream
        return Err(io::Error::new(
            ErrorKind::Other,
            format!("{}: cannot determine input size", input),
        ));
    } else {
        // Could be that file size is larger than set read limit
        // Get the file size from filesystem metadata
        let metadata = metadata(input)?;
        let metadata_size = metadata.len();
        if num_bytes <= metadata_size {
            Ok(metadata_size)
        } else {
            // Could be a file from locations like /dev, /sys, /proc or similar
            // which report filesystem metadata size that does not match
            // their actual content size
            // Attempt direct `seek()` for the end of a file
            let mut tmp_fd = File::open(Path::new(input))?;
            let end = tmp_fd.seek(SeekFrom::End(0))?;
            if end > 0 {
                Ok(end)
            } else {
                // Edge case of either "infinite" file (i.e. /dev/zero)
                // or some other "special" non-standard file type
                // Give up and return an error
                // TODO It might be possible to do more here
                // to address all possible file types and edge cases
                return Err(io::Error::new(
                    ErrorKind::Other,
                    format!("{}: cannot determine file size", input),
                ));
            }
        }
    }
}

/// Write a certain number of bytes to one file, then move on to another one.
///
/// This struct maintains an underlying writer representing the
/// current chunk of the output. If a call to [`write`] would cause
/// the underlying writer to write more than the allowed number of
/// bytes, a new writer is created and the excess bytes are written to
/// that one instead. As many new underlying writers are created as
/// needed to write all the bytes in the input buffer.
struct ByteChunkWriter<'a> {
    /// Parameters for creating the underlying writer for each new chunk.
    settings: &'a Settings,

    /// The maximum number of bytes allowed for a single chunk of output.
    chunk_size: u64,

    /// Running total of number of chunks that have been completed.
    num_chunks_written: u64,

    /// Remaining capacity in number of bytes in the current chunk.
    ///
    /// This number starts at `chunk_size` and decreases as bytes are
    /// written. Once it reaches zero, a writer for a new chunk is
    /// initialized and this number gets reset to `chunk_size`.
    num_bytes_remaining_in_current_chunk: u64,

    /// The underlying writer for the current chunk.
    ///
    /// Once the number of bytes written to this writer exceeds
    /// `chunk_size`, a new writer is initialized and assigned to this
    /// field.
    inner: BufWriter<Box<dyn Write>>,

    /// Iterator that yields filenames for each chunk.
    filename_iterator: FilenameIterator<'a>,
}

impl<'a> ByteChunkWriter<'a> {
    fn new(chunk_size: u64, settings: &'a Settings) -> UResult<Self> {
        let mut filename_iterator = FilenameIterator::new(&settings.prefix, &settings.suffix)?;
        let filename = filename_iterator
            .next()
            .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
        if settings.verbose {
            println!("creating file {}", filename.quote());
        }
        let inner = settings.instantiate_current_writer(&filename, true)?;
        Ok(ByteChunkWriter {
            settings,
            chunk_size,
            num_bytes_remaining_in_current_chunk: chunk_size,
            num_chunks_written: 0,
            inner,
            filename_iterator,
        })
    }
}

impl<'a> Write for ByteChunkWriter<'a> {
    /// Implements `--bytes=SIZE`
    fn write(&mut self, mut buf: &[u8]) -> std::io::Result<usize> {
        // If the length of `buf` exceeds the number of bytes remaining
        // in the current chunk, we will need to write to multiple
        // different underlying writers. In that case, each iteration of
        // this loop writes to the underlying writer that corresponds to
        // the current chunk number.
        let mut carryover_bytes_written: usize = 0;
        loop {
            if buf.is_empty() {
                return Ok(carryover_bytes_written);
            }

            if self.num_bytes_remaining_in_current_chunk == 0 {
                // Increment the chunk number, reset the number of bytes remaining, and instantiate the new underlying writer.
                self.num_chunks_written += 1;
                self.num_bytes_remaining_in_current_chunk = self.chunk_size;

                // Allocate the new file, since at this point we know there are bytes to be written to it.
                let filename = self.filename_iterator.next().ok_or_else(|| {
                    std::io::Error::new(ErrorKind::Other, "output file suffixes exhausted")
                })?;
                if self.settings.verbose {
                    println!("creating file {}", filename.quote());
                }
                self.inner = self.settings.instantiate_current_writer(&filename, true)?;
            }

            // If the capacity of this chunk is greater than the number of
            // bytes in `buf`, then write all the bytes in `buf`. Otherwise,
            // write enough bytes to fill the current chunk, then increment
            // the chunk number and repeat.
            let buf_len = buf.len();
            if (buf_len as u64) < self.num_bytes_remaining_in_current_chunk {
                let num_bytes_written = custom_write(buf, &mut self.inner, self.settings)?;
                self.num_bytes_remaining_in_current_chunk -= num_bytes_written as u64;
                return Ok(carryover_bytes_written + num_bytes_written);
            } else {
                // Write enough bytes to fill the current chunk.
                //
                // Conversion to usize is safe because we checked that
                // self.num_bytes_remaining_in_current_chunk is lower than
                // n, which is already usize.
                let i = self.num_bytes_remaining_in_current_chunk as usize;
                let num_bytes_written = custom_write(&buf[..i], &mut self.inner, self.settings)?;
                self.num_bytes_remaining_in_current_chunk -= num_bytes_written as u64;

                // It's possible that the underlying writer did not
                // write all the bytes.
                if num_bytes_written < i {
                    return Ok(carryover_bytes_written + num_bytes_written);
                } else {
                    // Move the window to look at only the remaining bytes.
                    buf = &buf[i..];

                    // Remember for the next iteration that we wrote these bytes.
                    carryover_bytes_written += num_bytes_written;
                }
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Write a certain number of lines to one file, then move on to another one.
///
/// This struct maintains an underlying writer representing the
/// current chunk of the output. If a call to [`write`] would cause
/// the underlying writer to write more than the allowed number of
/// lines, a new writer is created and the excess lines are written to
/// that one instead. As many new underlying writers are created as
/// needed to write all the lines in the input buffer.
struct LineChunkWriter<'a> {
    /// Parameters for creating the underlying writer for each new chunk.
    settings: &'a Settings,

    /// The maximum number of lines allowed for a single chunk of output.
    chunk_size: u64,

    /// Running total of number of chunks that have been completed.
    num_chunks_written: u64,

    /// Remaining capacity in number of lines in the current chunk.
    ///
    /// This number starts at `chunk_size` and decreases as lines are
    /// written. Once it reaches zero, a writer for a new chunk is
    /// initialized and this number gets reset to `chunk_size`.
    num_lines_remaining_in_current_chunk: u64,

    /// The underlying writer for the current chunk.
    ///
    /// Once the number of lines written to this writer exceeds
    /// `chunk_size`, a new writer is initialized and assigned to this
    /// field.
    inner: BufWriter<Box<dyn Write>>,

    /// Iterator that yields filenames for each chunk.
    filename_iterator: FilenameIterator<'a>,
}

impl<'a> LineChunkWriter<'a> {
    fn new(chunk_size: u64, settings: &'a Settings) -> UResult<Self> {
        let mut filename_iterator = FilenameIterator::new(&settings.prefix, &settings.suffix)?;
        let filename = filename_iterator
            .next()
            .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
        if settings.verbose {
            println!("creating file {}", filename.quote());
        }
        let inner = settings.instantiate_current_writer(&filename, true)?;
        Ok(LineChunkWriter {
            settings,
            chunk_size,
            num_lines_remaining_in_current_chunk: chunk_size,
            num_chunks_written: 0,
            inner,
            filename_iterator,
        })
    }
}

impl<'a> Write for LineChunkWriter<'a> {
    /// Implements `--lines=NUMBER`
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // If the number of lines in `buf` exceeds the number of lines
        // remaining in the current chunk, we will need to write to
        // multiple different underlying writers. In that case, each
        // iteration of this loop writes to the underlying writer that
        // corresponds to the current chunk number.
        let mut prev = 0;
        let mut total_bytes_written = 0;
        let sep = self.settings.separator;
        for i in memchr::memchr_iter(sep, buf) {
            // If we have exceeded the number of lines to write in the
            // current chunk, then start a new chunk and its
            // corresponding writer.
            if self.num_lines_remaining_in_current_chunk == 0 {
                self.num_chunks_written += 1;
                let filename = self.filename_iterator.next().ok_or_else(|| {
                    std::io::Error::new(ErrorKind::Other, "output file suffixes exhausted")
                })?;
                if self.settings.verbose {
                    println!("creating file {}", filename.quote());
                }
                self.inner = self.settings.instantiate_current_writer(&filename, true)?;
                self.num_lines_remaining_in_current_chunk = self.chunk_size;
            }

            // Write the line, starting from *after* the previous
            // separator character and ending *after* the current
            // separator character.
            let num_bytes_written = custom_write(&buf[prev..=i], &mut self.inner, self.settings)?;
            total_bytes_written += num_bytes_written;
            prev = i + 1;
            self.num_lines_remaining_in_current_chunk -= 1;
        }

        let num_bytes_written =
            custom_write(&buf[prev..buf.len()], &mut self.inner, self.settings)?;
        total_bytes_written += num_bytes_written;
        Ok(total_bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Write lines to each sequential output files, limited by bytes.
///
/// This struct maintains an underlying writer representing the
/// current chunk of the output. On each call to [`write`], it writes
/// as many lines as possible to the current chunk without exceeding
/// the specified byte limit. If a single line has more bytes than the
/// limit, then fill an entire single chunk with those bytes and
/// handle the remainder of the line as if it were its own distinct
/// line. As many new underlying writers are created as needed to
/// write all the data in the input buffer.
struct LineBytesChunkWriter<'a> {
    /// Parameters for creating the underlying writer for each new chunk.
    settings: &'a Settings,

    /// The maximum number of bytes allowed for a single chunk of output.
    chunk_size: u64,

    /// Running total of number of chunks that have been completed.
    num_chunks_written: usize,

    /// Remaining capacity in number of bytes in the current chunk.
    ///
    /// This number starts at `chunk_size` and decreases as lines are
    /// written. Once it reaches zero, a writer for a new chunk is
    /// initialized and this number gets reset to `chunk_size`.
    num_bytes_remaining_in_current_chunk: usize,

    /// The underlying writer for the current chunk.
    ///
    /// Once the number of bytes written to this writer exceeds
    /// `chunk_size`, a new writer is initialized and assigned to this
    /// field.
    inner: BufWriter<Box<dyn Write>>,

    /// Iterator that yields filenames for each chunk.
    filename_iterator: FilenameIterator<'a>,
}

impl<'a> LineBytesChunkWriter<'a> {
    fn new(chunk_size: u64, settings: &'a Settings) -> UResult<Self> {
        let mut filename_iterator = FilenameIterator::new(&settings.prefix, &settings.suffix)?;
        let filename = filename_iterator
            .next()
            .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
        if settings.verbose {
            println!("creating file {}", filename.quote());
        }
        let inner = settings.instantiate_current_writer(&filename, true)?;
        Ok(LineBytesChunkWriter {
            settings,
            chunk_size,
            num_bytes_remaining_in_current_chunk: usize::try_from(chunk_size).unwrap(),
            num_chunks_written: 0,
            inner,
            filename_iterator,
        })
    }
}

impl<'a> Write for LineBytesChunkWriter<'a> {
    /// Write as many lines to a chunk as possible without
    /// exceeding the byte limit. If a single line has more bytes
    /// than the limit, then fill an entire single chunk with those
    /// bytes and handle the remainder of the line as if it were
    /// its own distinct line.
    ///
    /// For example: if the `chunk_size` is 8 and the input is:
    ///
    /// ```text
    /// aaaaaaaaa\nbbbb\ncccc\ndd\nee\n
    /// ```
    ///
    /// then the output gets broken into chunks like this:
    ///
    /// ```text
    /// chunk 0    chunk 1    chunk 2    chunk 3
    ///
    /// 0            1             2
    /// 01234567  89 01234   56789 012   345 6
    /// |------|  |-------|  |--------|  |---|
    /// aaaaaaaa  a\nbbbb\n  cccc\ndd\n  ee\n
    /// ```
    ///
    /// Implements `--line-bytes=SIZE`
    fn write(&mut self, mut buf: &[u8]) -> std::io::Result<usize> {
        // The total number of bytes written during the loop below.
        //
        // It is necessary to keep this running total because we may
        // be making multiple calls to `write()` on multiple different
        // underlying writers and we want the final reported number of
        // bytes written to reflect the total number of bytes written
        // to all of the underlying writers.
        let mut total_bytes_written = 0;

        // Loop until we have written all bytes in the input buffer
        // (or an IO error occurs).
        loop {
            // If the buffer is empty, then we are done writing.
            if buf.is_empty() {
                return Ok(total_bytes_written);
            }

            // If we have filled the current chunk with bytes, then
            // start a new chunk and initialize its corresponding
            // writer.
            if self.num_bytes_remaining_in_current_chunk == 0 {
                self.num_chunks_written += 1;
                let filename = self.filename_iterator.next().ok_or_else(|| {
                    std::io::Error::new(ErrorKind::Other, "output file suffixes exhausted")
                })?;
                if self.settings.verbose {
                    println!("creating file {}", filename.quote());
                }
                self.inner = self.settings.instantiate_current_writer(&filename, true)?;
                self.num_bytes_remaining_in_current_chunk = self.chunk_size.try_into().unwrap();
            }

            // Find the first separator (default - newline character) in the buffer.
            let sep = self.settings.separator;
            match memchr::memchr(sep, buf) {
                // If there is no separator character and the buffer is
                // not empty, then write as many bytes as we can and
                // then move on to the next chunk if necessary.
                None => {
                    let end = self.num_bytes_remaining_in_current_chunk;

                    // This is ugly but here to match GNU behavior. If the input
                    // doesn't end with a separator, pretend that it does for handling
                    // the second to last segment chunk. See `line-bytes.sh`.
                    if end == buf.len()
                        && self.num_bytes_remaining_in_current_chunk
                            < self.chunk_size.try_into().unwrap()
                        && buf[buf.len() - 1] != sep
                    {
                        self.num_bytes_remaining_in_current_chunk = 0;
                    } else {
                        let num_bytes_written = custom_write(
                            &buf[..end.min(buf.len())],
                            &mut self.inner,
                            self.settings,
                        )?;
                        self.num_bytes_remaining_in_current_chunk -= num_bytes_written;
                        total_bytes_written += num_bytes_written;
                        buf = &buf[num_bytes_written..];
                    }
                }

                // If there is a separator character and the line
                // (including the separator character) will fit in the
                // current chunk, then write the entire line and
                // continue to the next iteration. (See chunk 1 in the
                // example comment above.)
                Some(i) if i < self.num_bytes_remaining_in_current_chunk => {
                    let num_bytes_written =
                        custom_write(&buf[..=i], &mut self.inner, self.settings)?;
                    self.num_bytes_remaining_in_current_chunk -= num_bytes_written;
                    total_bytes_written += num_bytes_written;
                    buf = &buf[num_bytes_written..];
                }

                // If there is a separator character, the line
                // (including the separator character) will not fit in
                // the current chunk, *and* no other lines have been
                // written to the current chunk, then write as many
                // bytes as we can and continue to the next
                // iteration. (See chunk 0 in the example comment
                // above.)
                Some(_)
                    if self.num_bytes_remaining_in_current_chunk
                        == self.chunk_size.try_into().unwrap() =>
                {
                    let end = self.num_bytes_remaining_in_current_chunk;
                    let num_bytes_written =
                        custom_write(&buf[..end], &mut self.inner, self.settings)?;
                    self.num_bytes_remaining_in_current_chunk -= num_bytes_written;
                    total_bytes_written += num_bytes_written;
                    buf = &buf[num_bytes_written..];
                }

                // If there is a separator character, the line
                // (including the separator character) will not fit in
                // the current chunk, and at least one other line has
                // been written to the current chunk, then signal to
                // the next iteration that a new chunk needs to be
                // created and continue to the next iteration of the
                // loop to try writing the line there.
                Some(_) => {
                    self.num_bytes_remaining_in_current_chunk = 0;
                }
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Output file parameters
struct OutFile {
    filename: String,
    maybe_writer: Option<BufWriter<Box<dyn Write>>>,
    is_new: bool,
}

/// A set of output files
/// Used in [`n_chunks_by_byte`], [`n_chunks_by_line`]
/// and [`n_chunks_by_line_round_robin`] functions.
type OutFiles = Vec<OutFile>;
trait ManageOutFiles {
    fn instantiate_writer(
        &mut self,
        idx: usize,
        settings: &Settings,
    ) -> UResult<&mut BufWriter<Box<dyn Write>>>;
    /// Initialize a new set of output files
    /// Each OutFile is generated with filename, while the writer for it could be
    /// optional, to be instantiated later by the calling function as needed.
    /// Optional writers could happen in the following situations:
    /// * in [`n_chunks_by_line`] and [`n_chunks_by_line_round_robin`] if `elide_empty_files` parameter is set to `true`
    /// * if the number of files is greater than system limit for open files
    fn init(num_files: u64, settings: &Settings, is_writer_optional: bool) -> UResult<Self>
    where
        Self: Sized;
    /// Get the writer for the output file by index.
    /// If system limit of open files has been reached
    /// it will try to close one of previously instantiated writers
    /// to free up resources and re-try instantiating current writer,
    /// except for `--filter` mode.
    /// The writers that get closed to free up resources for the current writer
    /// are flagged as `is_new=false`, so they can be re-opened for appending
    /// instead of created anew if we need to keep writing into them later,
    /// i.e. in case of round robin distribution as in [`n_chunks_by_line_round_robin`]
    fn get_writer(
        &mut self,
        idx: usize,
        settings: &Settings,
    ) -> UResult<&mut BufWriter<Box<dyn Write>>>;
}

impl ManageOutFiles for OutFiles {
    fn init(num_files: u64, settings: &Settings, is_writer_optional: bool) -> UResult<Self> {
        // This object is responsible for creating the filename for each chunk
        let mut filename_iterator: FilenameIterator<'_> =
            FilenameIterator::new(&settings.prefix, &settings.suffix)
                .map_err(|e| io::Error::new(ErrorKind::Other, format!("{e}")))?;
        let mut out_files: Self = Self::new();
        for _ in 0..num_files {
            let filename = filename_iterator
                .next()
                .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
            let maybe_writer = if is_writer_optional {
                None
            } else {
                let instantiated = settings.instantiate_current_writer(filename.as_str(), true);
                // If there was an error instantiating the writer for a file,
                // it could be due to hitting the system limit of open files,
                // so record it as None and let [`get_writer`] function handle closing/re-opening
                // of writers as needed within system limits.
                // However, for `--filter` child process writers - propagate the error,
                // as working around system limits of open files for child shell processes
                // is currently not supported (same as in GNU)
                match instantiated {
                    Ok(writer) => Some(writer),
                    Err(e) if settings.filter.is_some() => {
                        return Err(e.into());
                    }
                    Err(_) => None,
                }
            };
            out_files.push(OutFile {
                filename,
                maybe_writer,
                is_new: true,
            });
        }
        Ok(out_files)
    }

    fn instantiate_writer(
        &mut self,
        idx: usize,
        settings: &Settings,
    ) -> UResult<&mut BufWriter<Box<dyn Write>>> {
        let mut count = 0;
        // Use-case for doing multiple tries of closing fds:
        // E.g. split running in parallel to other processes (e.g. another split) doing similar stuff,
        // sharing the same limits. In this scenario, after closing one fd, the other process
        // might "steel" the freed fd and open a file on its side. Then it would be beneficial
        // if split would be able to close another fd before cancellation.
        'loop1: loop {
            let filename_to_open = self[idx].filename.as_str();
            let file_to_open_is_new = self[idx].is_new;
            let maybe_writer =
                settings.instantiate_current_writer(filename_to_open, file_to_open_is_new);
            if let Ok(writer) = maybe_writer {
                self[idx].maybe_writer = Some(writer);
                return Ok(self[idx].maybe_writer.as_mut().unwrap());
            }

            if settings.filter.is_some() {
                // Propagate error if in `--filter` mode
                return Err(maybe_writer.err().unwrap().into());
            }

            // Could have hit system limit for open files.
            // Try to close one previously instantiated writer first
            for (i, out_file) in self.iter_mut().enumerate() {
                if i != idx && out_file.maybe_writer.is_some() {
                    out_file.maybe_writer.as_mut().unwrap().flush()?;
                    out_file.maybe_writer = None;
                    out_file.is_new = false;
                    count += 1;

                    // And then try to instantiate the writer again
                    continue 'loop1;
                }
            }

            // If this fails - give up and propagate the error
            uucore::show_error!("at file descriptor limit, but no file descriptor left to close. Closed {count} writers before.");
            return Err(maybe_writer.err().unwrap().into());
        }
    }

    fn get_writer(
        &mut self,
        idx: usize,
        settings: &Settings,
    ) -> UResult<&mut BufWriter<Box<dyn Write>>> {
        if self[idx].maybe_writer.is_some() {
            Ok(self[idx].maybe_writer.as_mut().unwrap())
        } else {
            // Writer was not instantiated upfront or was temporarily closed due to system resources constraints.
            // Instantiate it and record for future use.
            self.instantiate_writer(idx, settings)
        }
    }
}

/// Split a file or STDIN into a specific number of chunks by byte.
///
/// When file size cannot be evenly divided into the number of chunks of the same size,
/// the first X chunks are 1 byte longer than the rest,
/// where X is a modulus reminder of (file size % number of chunks)
///
/// In Kth chunk of N mode - writes to STDOUT the contents of the chunk identified by `kth_chunk`
///
/// In N chunks mode - this function always creates one output file for each chunk, even
/// if there is an error reading or writing one of the chunks or if
/// the input file is truncated. However, if the `--filter` option is
/// being used, then files will only be created if `$FILE` variable was used
/// in filter command,
/// i.e. `split -n 10 --filter='head -c1 > $FILE' in`
///
/// # Errors
///
/// This function returns an error if there is a problem reading from
/// `reader` or writing to one of the output files or stdout.
///
/// # See also
///
/// * [`n_chunks_by_line`], which splits its input into a specific number of chunks by line.
///
/// Implements `--number=CHUNKS`
/// Where CHUNKS
/// * N
/// * K/N
fn n_chunks_by_byte<R>(
    settings: &Settings,
    reader: &mut R,
    num_chunks: u64,
    kth_chunk: Option<u64>,
) -> UResult<()>
where
    R: BufRead,
{
    // Get the size of the input in bytes
    let initial_buf = &mut Vec::new();
    let mut num_bytes = get_input_size(&settings.input, reader, initial_buf, &settings.io_blksize)?;
    let mut reader = initial_buf.chain(reader);

    // If input file is empty and we would not have determined the Kth chunk
    // in the Kth chunk of N chunk mode, then terminate immediately.
    // This happens on `split -n 3/10 /dev/null`, for example.
    if kth_chunk.is_some() && num_bytes == 0 {
        return Ok(());
    }

    // If the requested number of chunks exceeds the number of bytes
    // in the input:
    // * in Kth chunk of N mode - just write empty byte string to stdout
    //   NOTE: the `elide_empty_files` parameter is ignored here
    //   as we do not generate any files
    //   and instead writing to stdout
    // * In N chunks mode - if the `elide_empty_files` parameter is enabled,
    //   then behave as if the number of chunks was set to the number of
    //   bytes in the file. This ensures that we don't write empty
    //   files. Otherwise, just write the `num_chunks - num_bytes` empty files.
    let num_chunks = if kth_chunk.is_none() && settings.elide_empty_files && num_chunks > num_bytes
    {
        num_bytes
    } else {
        num_chunks
    };

    // If we would have written zero chunks of output, then terminate
    // immediately. This happens on `split -e -n 3 /dev/null`, for
    // example.
    if num_chunks == 0 {
        return Ok(());
    }

    // In Kth chunk of N mode - we will write to stdout instead of to a file.
    let mut stdout_writer = std::io::stdout().lock();
    // In N chunks mode - we will write to `num_chunks` files
    let mut out_files: OutFiles = OutFiles::new();

    // Calculate chunk size base and modulo reminder
    // to be used in calculating chunk_size later on
    let chunk_size_base = num_bytes / num_chunks;
    let chunk_size_reminder = num_bytes % num_chunks;

    // If in N chunks mode
    // Create one writer for each chunk.
    // This will create each of the underlying files
    // or stdin pipes to child shell/command processes if in `--filter` mode
    if kth_chunk.is_none() {
        out_files = OutFiles::init(num_chunks, settings, false)?;
    }

    for i in 1_u64..=num_chunks {
        let chunk_size = chunk_size_base + (chunk_size_reminder > i - 1) as u64;
        let buf = &mut Vec::new();
        if num_bytes > 0 {
            // Read `chunk_size` bytes from the reader into `buf`
            // except the last.
            //
            // The last chunk gets all remaining bytes so that if the number
            // of bytes in the input file was not evenly divisible by
            // `num_chunks`, we don't leave any bytes behind.
            let limit = {
                if i == num_chunks {
                    num_bytes
                } else {
                    chunk_size
                }
            };

            let n_bytes_read = reader.by_ref().take(limit).read_to_end(buf);

            match n_bytes_read {
                Ok(n_bytes) => {
                    num_bytes -= n_bytes as u64;
                }
                Err(error) => {
                    return Err(USimpleError::new(
                        1,
                        format!("{}: cannot read from input : {}", settings.input, error),
                    ));
                }
            }

            match kth_chunk {
                Some(chunk_number) => {
                    if i == chunk_number {
                        stdout_writer.write_all(buf)?;
                        break;
                    }
                }
                None => {
                    let idx = (i - 1) as usize;
                    let writer = out_files.get_writer(idx, settings)?;
                    writer.write_all(buf)?;
                }
            }
        } else {
            break;
        }
    }
    Ok(())
}

/// Split a file or STDIN into a specific number of chunks by line.
///
/// It is most likely that input cannot be evenly divided into the number of chunks
/// of the same size in bytes or number of lines, since we cannot break lines.
/// It is also likely that there could be empty files (having `elide_empty_files` is disabled)
/// when a long line overlaps one or more chunks.
///
/// In Kth chunk of N mode - writes to STDOUT the contents of the chunk identified by `kth_chunk`
/// Note: the `elide_empty_files` flag is ignored in this mode
///
/// In N chunks mode - this function always creates one output file for each chunk, even
/// if there is an error reading or writing one of the chunks or if
/// the input file is truncated. However, if the `--filter` option is
/// being used, then files will only be created if `$FILE` variable was used
/// in filter command,
/// i.e. `split -n l/10 --filter='head -c1 > $FILE' in`
///
/// # Errors
///
/// This function returns an error if there is a problem reading from
/// `reader` or writing to one of the output files.
///
/// # See also
///
/// * [`n_chunks_by_byte`], which splits its input into a specific number of chunks by byte.
///
/// Implements `--number=CHUNKS`
/// Where CHUNKS
/// * l/N
/// * l/K/N
fn n_chunks_by_line<R>(
    settings: &Settings,
    reader: &mut R,
    num_chunks: u64,
    kth_chunk: Option<u64>,
) -> UResult<()>
where
    R: BufRead,
{
    // Get the size of the input in bytes and compute the number
    // of bytes per chunk.
    let initial_buf = &mut Vec::new();
    let num_bytes = get_input_size(&settings.input, reader, initial_buf, &settings.io_blksize)?;
    let reader = initial_buf.chain(reader);

    // If input file is empty and we would not have determined the Kth chunk
    // in the Kth chunk of N chunk mode, then terminate immediately.
    // This happens on `split -n l/3/10 /dev/null`, for example.
    // Similarly, if input file is empty and `elide_empty_files` parameter is enabled,
    // then we would have written zero chunks of output,
    // so terminate immediately as well.
    // This happens on `split -e -n l/3 /dev/null`, for example.
    if num_bytes == 0 && (kth_chunk.is_some() || settings.elide_empty_files) {
        return Ok(());
    }

    // In Kth chunk of N mode - we will write to stdout instead of to a file.
    let mut stdout_writer = std::io::stdout().lock();
    // In N chunks mode - we will write to `num_chunks` files
    let mut out_files: OutFiles = OutFiles::new();

    // Calculate chunk size base and modulo reminder
    // to be used in calculating `num_bytes_should_be_written` later on
    let chunk_size_base = num_bytes / num_chunks;
    let chunk_size_reminder = num_bytes % num_chunks;

    // If in N chunks mode
    // Generate filenames for each file and
    // if `elide_empty_files` parameter is NOT enabled - instantiate the writer
    // which will create each of the underlying files or stdin pipes
    // to child shell/command processes if in `--filter` mode.
    // Otherwise keep writer optional, to be instantiated later if there is data
    // to write for the associated chunk.
    if kth_chunk.is_none() {
        out_files = OutFiles::init(num_chunks, settings, settings.elide_empty_files)?;
    }

    let mut chunk_number = 1;
    let sep = settings.separator;
    let mut num_bytes_should_be_written = chunk_size_base + (chunk_size_reminder > 0) as u64;
    let mut num_bytes_written = 0;

    for line_result in reader.split(sep) {
        let mut line = line_result?;
        // add separator back in at the end of the line,
        // since `reader.split(sep)` removes it,
        // except if the last line did not end with separator character
        if (num_bytes_written + line.len() as u64) < num_bytes {
            line.push(sep);
        }
        let bytes = line.as_slice();

        match kth_chunk {
            Some(kth) => {
                if chunk_number == kth {
                    stdout_writer.write_all(bytes)?;
                }
            }
            None => {
                // Should write into a file
                let idx = (chunk_number - 1) as usize;
                let writer = out_files.get_writer(idx, settings)?;
                custom_write_all(bytes, writer, settings)?;
            }
        }

        // Advance to the next chunk if the current one is filled.
        // There could be a situation when a long line, which started in current chunk,
        // would overlap the next chunk (or even several next chunks),
        // and since we cannot break lines for this split strategy, we could end up with
        // empty files in place(s) of skipped chunk(s)
        let num_line_bytes = bytes.len() as u64;
        num_bytes_written += num_line_bytes;
        let mut skipped = -1;
        while num_bytes_should_be_written <= num_bytes_written {
            num_bytes_should_be_written +=
                chunk_size_base + (chunk_size_reminder > chunk_number) as u64;
            chunk_number += 1;
            skipped += 1;
        }

        // If a chunk was skipped and `elide_empty_files` flag is set,
        // roll chunk_number back to preserve sequential continuity
        // of file names for files written to,
        // except for Kth chunk of N mode
        if settings.elide_empty_files && skipped > 0 && kth_chunk.is_none() {
            chunk_number -= skipped as u64;
        }

        if let Some(kth) = kth_chunk {
            if chunk_number > kth {
                break;
            }
        }
    }
    Ok(())
}

/// Split a file or STDIN into a specific number of chunks by line, but
/// assign lines via round-robin.
/// Note: There is no need to know the size of the input upfront for this method,
/// since the lines are assigned to chunks randomly and the size of each chunk
/// does not need to be estimated. As a result, "infinite" inputs are supported
/// for this method, i.e. `yes | split -n r/10` or `yes | split -n r/3/11`
///
/// In Kth chunk of N mode - writes to stdout the contents of the chunk identified by `kth_chunk`
///
/// In N chunks mode - this function always creates one output file for each chunk, even
/// if there is an error reading or writing one of the chunks or if
/// the input file is truncated. However, if the `--filter` option is
/// being used, then files will only be created if `$FILE` variable was used
/// in filter command,
/// i.e. `split -n r/10 --filter='head -c1 > $FILE' in`
///
/// # Errors
///
/// This function returns an error if there is a problem reading from
/// `reader` or writing to one of the output files.
///
/// # See also
///
/// * [`n_chunks_by_line`], which splits its input into a specific number of chunks by line.
///
/// Implements `--number=CHUNKS`
/// Where CHUNKS
/// * r/N
/// * r/K/N
fn n_chunks_by_line_round_robin<R>(
    settings: &Settings,
    reader: &mut R,
    num_chunks: u64,
    kth_chunk: Option<u64>,
) -> UResult<()>
where
    R: BufRead,
{
    // In Kth chunk of N mode - we will write to stdout instead of to a file.
    let mut stdout_writer = std::io::stdout().lock();
    // In N chunks mode - we will write to `num_chunks` files
    let mut out_files: OutFiles = OutFiles::new();

    // If in N chunks mode
    // Create one writer for each chunk.
    // This will create each of the underlying files
    // or stdin pipes to child shell/command processes if in `--filter` mode
    if kth_chunk.is_none() {
        out_files = OutFiles::init(num_chunks, settings, settings.elide_empty_files)?;
    }

    let num_chunks: usize = num_chunks.try_into().unwrap();
    let sep = settings.separator;
    let mut closed_writers = 0;

    let mut i = 0;
    loop {
        let line = &mut Vec::new();
        let num_bytes_read = reader.by_ref().read_until(sep, line)?;

        // if there is nothing else to read - exit the loop
        if num_bytes_read == 0 {
            break;
        };

        let bytes = line.as_slice();
        match kth_chunk {
            Some(chunk_number) => {
                if (i % num_chunks) == (chunk_number - 1) as usize {
                    stdout_writer.write_all(bytes)?;
                }
            }
            None => {
                let writer = out_files.get_writer(i % num_chunks, settings)?;
                let writer_stdin_open = custom_write_all(bytes, writer, settings)?;
                if !writer_stdin_open {
                    closed_writers += 1;
                }
            }
        }
        i += 1;
        if closed_writers == num_chunks {
            // all writers are closed - stop reading
            break;
        }
    }
    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn split(settings: &Settings) -> UResult<()> {
    let r_box = if settings.input == "-" {
        Box::new(stdin()) as Box<dyn Read>
    } else {
        let r = File::open(Path::new(&settings.input))
            .map_err_context(|| format!("cannot open {} for reading", settings.input.quote()))?;
        Box::new(r) as Box<dyn Read>
    };
    let mut reader = if let Some(c) = settings.io_blksize {
        BufReader::with_capacity(c.try_into().unwrap(), r_box)
    } else {
        BufReader::new(r_box)
    };

    match settings.strategy {
        Strategy::Number(NumberType::Bytes(num_chunks)) => {
            // split_into_n_chunks_by_byte(settings, &mut reader, num_chunks)
            n_chunks_by_byte(settings, &mut reader, num_chunks, None)
        }
        Strategy::Number(NumberType::KthBytes(chunk_number, num_chunks)) => {
            // kth_chunks_by_byte(settings, &mut reader, chunk_number, num_chunks)
            n_chunks_by_byte(settings, &mut reader, num_chunks, Some(chunk_number))
        }
        Strategy::Number(NumberType::Lines(num_chunks)) => {
            n_chunks_by_line(settings, &mut reader, num_chunks, None)
        }
        Strategy::Number(NumberType::KthLines(chunk_number, num_chunks)) => {
            n_chunks_by_line(settings, &mut reader, num_chunks, Some(chunk_number))
        }
        Strategy::Number(NumberType::RoundRobin(num_chunks)) => {
            n_chunks_by_line_round_robin(settings, &mut reader, num_chunks, None)
        }
        Strategy::Number(NumberType::KthRoundRobin(chunk_number, num_chunks)) => {
            n_chunks_by_line_round_robin(settings, &mut reader, num_chunks, Some(chunk_number))
        }
        Strategy::Lines(chunk_size) => {
            let mut writer = LineChunkWriter::new(chunk_size, settings)?;
            match std::io::copy(&mut reader, &mut writer) {
                Ok(_) => Ok(()),
                Err(e) => match e.kind() {
                    // TODO Since the writer object controls the creation of
                    // new files, we need to rely on the `std::io::Result`
                    // returned by its `write()` method to communicate any
                    // errors to this calling scope. If a new file cannot be
                    // created because we have exceeded the number of
                    // allowable filenames, we use `ErrorKind::Other` to
                    // indicate that. A special error message needs to be
                    // printed in that case.
                    ErrorKind::Other => Err(USimpleError::new(1, format!("{e}"))),
                    _ => Err(uio_error!(e, "input/output error")),
                },
            }
        }
        Strategy::Bytes(chunk_size) => {
            let mut writer = ByteChunkWriter::new(chunk_size, settings)?;
            match std::io::copy(&mut reader, &mut writer) {
                Ok(_) => Ok(()),
                Err(e) => match e.kind() {
                    // TODO Since the writer object controls the creation of
                    // new files, we need to rely on the `std::io::Result`
                    // returned by its `write()` method to communicate any
                    // errors to this calling scope. If a new file cannot be
                    // created because we have exceeded the number of
                    // allowable filenames, we use `ErrorKind::Other` to
                    // indicate that. A special error message needs to be
                    // printed in that case.
                    ErrorKind::Other => Err(USimpleError::new(1, format!("{e}"))),
                    _ => Err(uio_error!(e, "input/output error")),
                },
            }
        }
        Strategy::LineBytes(chunk_size) => {
            let mut writer = LineBytesChunkWriter::new(chunk_size, settings)?;
            match std::io::copy(&mut reader, &mut writer) {
                Ok(_) => Ok(()),
                Err(e) => match e.kind() {
                    // TODO Since the writer object controls the creation of
                    // new files, we need to rely on the `std::io::Result`
                    // returned by its `write()` method to communicate any
                    // errors to this calling scope. If a new file cannot be
                    // created because we have exceeded the number of
                    // allowable filenames, we use `ErrorKind::Other` to
                    // indicate that. A special error message needs to be
                    // printed in that case.
                    ErrorKind::Other => Err(USimpleError::new(1, format!("{e}"))),
                    _ => Err(uio_error!(e, "input/output error")),
                },
            }
        }
    }
}
