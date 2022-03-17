//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Akira Hayakawa <ruby.wktk@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) PREFIXaa nbbbb ncccc

mod filenames;
mod number;
mod platform;

use crate::filenames::FilenameIterator;
use crate::filenames::SuffixType;
use clap::{crate_version, Arg, ArgMatches, Command};
use std::convert::TryInto;
use std::env;
use std::fmt;
use std::fs::{metadata, File};
use std::io;
use std::io::{stdin, BufRead, BufReader, BufWriter, ErrorKind, Read, Write};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UIoError, UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::uio_error;

static OPT_BYTES: &str = "bytes";
static OPT_LINE_BYTES: &str = "line-bytes";
static OPT_LINES: &str = "lines";
static OPT_ADDITIONAL_SUFFIX: &str = "additional-suffix";
static OPT_FILTER: &str = "filter";
static OPT_NUMBER: &str = "number";
static OPT_NUMERIC_SUFFIXES: &str = "numeric-suffixes";
static OPT_HEX_SUFFIXES: &str = "hex-suffixes";
static OPT_SUFFIX_LENGTH: &str = "suffix-length";
static OPT_DEFAULT_SUFFIX_LENGTH: &str = "0";
static OPT_VERBOSE: &str = "verbose";
//The ---io-blksize parameter is consumed and ignored.
//The parameter is included to make GNU coreutils tests pass.
static OPT_IO_BLKSIZE: &str = "-io-blksize";
static OPT_ELIDE_EMPTY_FILES: &str = "elide-empty-files";

static ARG_INPUT: &str = "input";
static ARG_PREFIX: &str = "prefix";

const USAGE: &str = "{} [OPTION]... [INPUT [PREFIX]]";
const AFTER_HELP: &str = "\
    Output fixed-size pieces of INPUT to PREFIXaa, PREFIX ab, ...; default \
    size is 1000, and default PREFIX is 'x'. With no INPUT, or when INPUT is \
    -, read standard input.";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);
    match Settings::from(&matches) {
        Ok(settings) => split(&settings),
        Err(e) if e.requires_usage() => Err(UUsageError::new(1, format!("{}", e))),
        Err(e) => Err(USimpleError::new(1, format!("{}", e))),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about("Create output files containing consecutive or interleaved sections of input")
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        // strategy (mutually exclusive)
        .arg(
            Arg::new(OPT_BYTES)
                .short('b')
                .long(OPT_BYTES)
                .takes_value(true)
                .help("put SIZE bytes per output file"),
        )
        .arg(
            Arg::new(OPT_LINE_BYTES)
                .short('C')
                .long(OPT_LINE_BYTES)
                .takes_value(true)
                .default_value("2")
                .help("put at most SIZE bytes of lines per output file"),
        )
        .arg(
            Arg::new(OPT_LINES)
                .short('l')
                .long(OPT_LINES)
                .takes_value(true)
                .default_value("1000")
                .help("put NUMBER lines/records per output file"),
        )
        .arg(
            Arg::new(OPT_NUMBER)
                .short('n')
                .long(OPT_NUMBER)
                .takes_value(true)
                .help("generate CHUNKS output files; see explanation below"),
        )
        // rest of the arguments
        .arg(
            Arg::new(OPT_ADDITIONAL_SUFFIX)
                .long(OPT_ADDITIONAL_SUFFIX)
                .takes_value(true)
                .default_value("")
                .help("additional suffix to append to output file names"),
        )
        .arg(
            Arg::new(OPT_FILTER)
                .long(OPT_FILTER)
                .takes_value(true)
                .help(
                "write to shell COMMAND file name is $FILE (Currently not implemented for Windows)",
            ),
        )
        .arg(
            Arg::new(OPT_ELIDE_EMPTY_FILES)
                .long(OPT_ELIDE_EMPTY_FILES)
                .short('e')
                .takes_value(false)
                .help("do not generate empty output files with '-n'"),
        )
        .arg(
            Arg::new(OPT_NUMERIC_SUFFIXES)
                .short('d')
                .long(OPT_NUMERIC_SUFFIXES)
                .takes_value(true)
                .default_missing_value("0")
                .help("use numeric suffixes instead of alphabetic"),
        )
        .arg(
            Arg::new(OPT_SUFFIX_LENGTH)
                .short('a')
                .long(OPT_SUFFIX_LENGTH)
                .takes_value(true)
                .default_value(OPT_DEFAULT_SUFFIX_LENGTH)
                .help("use suffixes of length N (default 2)"),
        )
        .arg(
            Arg::new(OPT_HEX_SUFFIXES)
                .short('x')
                .long(OPT_HEX_SUFFIXES)
                .takes_value(true)
                .default_missing_value("0")
                .help("use hex suffixes instead of alphabetic"),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .long(OPT_VERBOSE)
                .help("print a diagnostic just before each output file is opened"),
        )
        .arg(
            Arg::new(OPT_IO_BLKSIZE)
                .long(OPT_IO_BLKSIZE)
                .alias(OPT_IO_BLKSIZE)
                .takes_value(true)
                .hide(true),
        )
        .arg(
            Arg::new(ARG_INPUT)
                .takes_value(true)
                .default_value("-")
                .index(1),
        )
        .arg(
            Arg::new(ARG_PREFIX)
                .takes_value(true)
                .default_value("x")
                .index(2),
        )
}

/// Sub-strategy to use when splitting a file into a specific number of chunks.
#[derive(Debug, PartialEq)]
enum NumberType {
    /// Split into a specific number of chunks by byte.
    Bytes(u64),

    /// Split into a specific number of chunks by line (approximately).
    Lines(u64),

    /// Split into a specific number of chunks by line
    /// (approximately), but output only the *k*th chunk.
    KthLines(u64, u64),

    /// Assign lines via round-robin to the specified number of output chunks.
    RoundRobin(u64),

    /// Assign lines via round-robin to the specified number of output
    /// chunks, but output only the *k*th chunk.
    KthRoundRobin(u64, u64),
}

impl NumberType {
    /// The number of chunks for this number type.
    fn num_chunks(&self) -> u64 {
        match self {
            Self::Bytes(n) => *n,
            Self::Lines(n) => *n,
            Self::KthLines(_, n) => *n,
            Self::RoundRobin(n) => *n,
            Self::KthRoundRobin(_, n) => *n,
        }
    }
}

/// An error due to an invalid parameter to the `-n` command-line option.
#[derive(Debug, PartialEq)]
enum NumberTypeError {
    /// The number of chunks was invalid.
    ///
    /// This can happen if the value of `N` in any of the following
    /// command-line options is not a positive integer:
    ///
    /// ```ignore
    /// -n N
    /// -n l/N
    /// -n l/K/N
    /// -n r/N
    /// -n r/K/N
    /// ```
    NumberOfChunks(String),

    /// The chunk number was invalid.
    ///
    /// This can happen if the value of `K` in any of the following
    /// command-line options is not a positive integer:
    ///
    /// ```ignore
    /// -n l/K/N
    /// -n r/K/N
    /// ```
    ChunkNumber(String),
}

impl NumberType {
    /// Parse a `NumberType` from a string.
    ///
    /// The following strings are valid arguments:
    ///
    /// ```ignore
    /// "N"
    /// "l/N"
    /// "l/K/N"
    /// "r/N"
    /// "r/K/N"
    /// ```
    ///
    /// The `N` represents the number of chunks and the `K` represents
    /// a chunk number.
    ///
    /// # Errors
    ///
    /// If the string is not one of the valid number types, if `K` is
    /// not a nonnegative integer, or if `N` is not a positive
    /// integer, then this function returns [`NumberTypeError`].
    fn from(s: &str) -> Result<Self, NumberTypeError> {
        let parts: Vec<&str> = s.split('/').collect();
        match &parts[..] {
            [n_str] => {
                let num_chunks = n_str
                    .parse()
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                Ok(Self::Bytes(num_chunks))
            }
            ["l", n_str] => {
                let num_chunks = n_str
                    .parse()
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                Ok(Self::Lines(num_chunks))
            }
            ["l", k_str, n_str] => {
                let num_chunks = n_str
                    .parse()
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                let chunk_number = k_str
                    .parse()
                    .map_err(|_| NumberTypeError::ChunkNumber(k_str.to_string()))?;
                Ok(Self::KthLines(chunk_number, num_chunks))
            }
            ["r", n_str] => {
                let num_chunks = n_str
                    .parse()
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                Ok(Self::RoundRobin(num_chunks))
            }
            ["r", k_str, n_str] => {
                let num_chunks = n_str
                    .parse()
                    .map_err(|_| NumberTypeError::NumberOfChunks(n_str.to_string()))?;
                let chunk_number = k_str
                    .parse()
                    .map_err(|_| NumberTypeError::ChunkNumber(k_str.to_string()))?;
                Ok(Self::KthRoundRobin(chunk_number, num_chunks))
            }
            _ => Err(NumberTypeError::NumberOfChunks(s.to_string())),
        }
    }
}

/// The strategy for breaking up the input file into chunks.
enum Strategy {
    /// Each chunk has the specified number of lines.
    Lines(u64),

    /// Each chunk has the specified number of bytes.
    Bytes(u64),

    /// Each chunk has as many lines as possible without exceeding the
    /// specified number of bytes.
    LineBytes(u64),

    /// Split the file into this many chunks.
    ///
    /// There are several sub-strategies available, as defined by
    /// [`NumberType`].
    Number(NumberType),
}

/// An error when parsing a chunking strategy from command-line arguments.
enum StrategyError {
    /// Invalid number of lines.
    Lines(ParseSizeError),

    /// Invalid number of bytes.
    Bytes(ParseSizeError),

    /// Invalid number type.
    NumberType(NumberTypeError),

    /// Multiple chunking strategies were specified (but only one should be).
    MultipleWays,
}

impl fmt::Display for StrategyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Lines(e) => write!(f, "invalid number of lines: {}", e),
            Self::Bytes(e) => write!(f, "invalid number of bytes: {}", e),
            Self::NumberType(NumberTypeError::NumberOfChunks(s)) => {
                write!(f, "invalid number of chunks: {}", s)
            }
            Self::NumberType(NumberTypeError::ChunkNumber(s)) => {
                write!(f, "invalid chunk number: {}", s)
            }
            Self::MultipleWays => write!(f, "cannot split in more than one way"),
        }
    }
}

impl Strategy {
    /// Parse a strategy from the command-line arguments.
    fn from(matches: &ArgMatches) -> Result<Self, StrategyError> {
        // Check that the user is not specifying more than one strategy.
        //
        // Note: right now, this exact behavior cannot be handled by
        // `ArgGroup` since `ArgGroup` considers a default value `Arg`
        // as "defined".
        match (
            matches.occurrences_of(OPT_LINES),
            matches.occurrences_of(OPT_BYTES),
            matches.occurrences_of(OPT_LINE_BYTES),
            matches.occurrences_of(OPT_NUMBER),
        ) {
            (0, 0, 0, 0) => Ok(Self::Lines(1000)),
            (1, 0, 0, 0) => {
                let s = matches.value_of(OPT_LINES).unwrap();
                let n = parse_size(s).map_err(StrategyError::Lines)?;
                Ok(Self::Lines(n))
            }
            (0, 1, 0, 0) => {
                let s = matches.value_of(OPT_BYTES).unwrap();
                let n = parse_size(s).map_err(StrategyError::Bytes)?;
                Ok(Self::Bytes(n))
            }
            (0, 0, 1, 0) => {
                let s = matches.value_of(OPT_LINE_BYTES).unwrap();
                let n = parse_size(s).map_err(StrategyError::Bytes)?;
                Ok(Self::LineBytes(n))
            }
            (0, 0, 0, 1) => {
                let s = matches.value_of(OPT_NUMBER).unwrap();
                let number_type = NumberType::from(s).map_err(StrategyError::NumberType)?;
                Ok(Self::Number(number_type))
            }
            _ => Err(StrategyError::MultipleWays),
        }
    }
}

/// Parse the suffix type from the command-line arguments.
fn suffix_type_from(matches: &ArgMatches) -> SuffixType {
    if matches.occurrences_of(OPT_NUMERIC_SUFFIXES) > 0 {
        SuffixType::Decimal
    } else if matches.occurrences_of(OPT_HEX_SUFFIXES) > 0 {
        SuffixType::Hexadecimal
    } else {
        SuffixType::Alphabetic
    }
}

/// Parameters that control how a file gets split.
///
/// You can convert an [`ArgMatches`] instance into a [`Settings`]
/// instance by calling [`Settings::from`].
struct Settings {
    prefix: String,
    suffix_type: SuffixType,
    suffix_length: usize,
    additional_suffix: String,
    input: String,
    /// When supplied, a shell command to output to instead of xaa, xab …
    filter: Option<String>,
    strategy: Strategy,
    verbose: bool,

    /// Whether to *not* produce empty files when using `-n`.
    ///
    /// The `-n` command-line argument gives a specific number of
    /// chunks into which the input files will be split. If the number
    /// of chunks is greater than the number of bytes, and this is
    /// `false`, then empty files will be created for the excess
    /// chunks. If this is `false`, then empty files will not be
    /// created.
    elide_empty_files: bool,
}

/// An error when parsing settings from command-line arguments.
enum SettingsError {
    /// Invalid chunking strategy.
    Strategy(StrategyError),

    /// Invalid suffix length parameter.
    SuffixNotParsable(String),

    /// Suffix contains a directory separator, which is not allowed.
    SuffixContainsSeparator(String),

    /// Suffix is not large enough to split into specified chunks
    SuffixTooSmall(usize),

    /// The `--filter` option is not supported on Windows.
    #[cfg(windows)]
    NotSupported,
}

impl SettingsError {
    /// Whether the error demands a usage message.
    fn requires_usage(&self) -> bool {
        matches!(
            self,
            Self::Strategy(StrategyError::MultipleWays) | Self::SuffixContainsSeparator(_)
        )
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Strategy(e) => e.fmt(f),
            Self::SuffixNotParsable(s) => write!(f, "invalid suffix length: {}", s.quote()),
            Self::SuffixTooSmall(i) => write!(f, "the suffix length needs to be at least {}", i),
            Self::SuffixContainsSeparator(s) => write!(
                f,
                "invalid suffix {}, contains directory separator",
                s.quote()
            ),
            #[cfg(windows)]
            Self::NotSupported => write!(
                f,
                "{} is currently not supported in this platform",
                OPT_FILTER
            ),
        }
    }
}

impl Settings {
    /// Parse a strategy from the command-line arguments.
    fn from(matches: &ArgMatches) -> Result<Self, SettingsError> {
        let additional_suffix = matches.value_of(OPT_ADDITIONAL_SUFFIX).unwrap().to_string();
        if additional_suffix.contains('/') {
            return Err(SettingsError::SuffixContainsSeparator(additional_suffix));
        }
        let strategy = Strategy::from(matches).map_err(SettingsError::Strategy)?;
        let suffix_type = suffix_type_from(matches);
        let suffix_length_str = matches.value_of(OPT_SUFFIX_LENGTH).unwrap();
        let suffix_length: usize = suffix_length_str
            .parse()
            .map_err(|_| SettingsError::SuffixNotParsable(suffix_length_str.to_string()))?;
        if let Strategy::Number(ref number_type) = strategy {
            let chunks = number_type.num_chunks();
            if suffix_length != 0 {
                let required_suffix_length =
                    (chunks as f64).log(suffix_type.radix() as f64).ceil() as usize;
                if suffix_length < required_suffix_length {
                    return Err(SettingsError::SuffixTooSmall(required_suffix_length));
                }
            }
        }
        let result = Self {
            suffix_length: suffix_length_str
                .parse()
                .map_err(|_| SettingsError::SuffixNotParsable(suffix_length_str.to_string()))?,
            suffix_type,
            additional_suffix,
            verbose: matches.occurrences_of("verbose") > 0,
            strategy,
            input: matches.value_of(ARG_INPUT).unwrap().to_owned(),
            prefix: matches.value_of(ARG_PREFIX).unwrap().to_owned(),
            filter: matches.value_of(OPT_FILTER).map(|s| s.to_owned()),
            elide_empty_files: matches.is_present(OPT_ELIDE_EMPTY_FILES),
        };
        #[cfg(windows)]
        if result.filter.is_some() {
            // see https://github.com/rust-lang/rust/issues/29494
            return Err(SettingsError::NotSupported);
        }

        Ok(result)
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
    fn new(chunk_size: u64, settings: &'a Settings) -> Option<ByteChunkWriter<'a>> {
        let mut filename_iterator = FilenameIterator::new(
            &settings.prefix,
            &settings.additional_suffix,
            settings.suffix_length,
            settings.suffix_type,
        );
        let filename = filename_iterator.next()?;
        if settings.verbose {
            println!("creating file {}", filename.quote());
        }
        let inner = platform::instantiate_current_writer(&settings.filter, &filename);
        Some(ByteChunkWriter {
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

            // If the capacity of this chunk is greater than the number of
            // bytes in `buf`, then write all the bytes in `buf`. Otherwise,
            // write enough bytes to fill the current chunk, then increment
            // the chunk number and repeat.
            let n = buf.len();
            if (n as u64) < self.num_bytes_remaining_in_current_chunk {
                let num_bytes_written = self.inner.write(buf)?;
                self.num_bytes_remaining_in_current_chunk -= num_bytes_written as u64;
                return Ok(carryover_bytes_written + num_bytes_written);
            } else {
                // Write enough bytes to fill the current chunk.
                //
                // Conversion to usize is safe because we checked that
                // self.num_bytes_remaining_in_current_chunk is lower than
                // n, which is already usize.
                let i = self.num_bytes_remaining_in_current_chunk as usize;
                let num_bytes_written = self.inner.write(&buf[..i])?;

                // It's possible that the underlying writer did not
                // write all the bytes.
                if num_bytes_written < i {
                    self.num_bytes_remaining_in_current_chunk -= num_bytes_written as u64;
                    return Ok(carryover_bytes_written + num_bytes_written);
                } else {
                    // Move the window to look at only the remaining bytes.
                    buf = &buf[i..];

                    // Increment the chunk number, reset the number of
                    // bytes remaining, and instantiate the new
                    // underlying writer.
                    self.num_chunks_written += 1;
                    self.num_bytes_remaining_in_current_chunk = self.chunk_size;

                    // Remember for the next iteration that we wrote these bytes.
                    carryover_bytes_written += num_bytes_written;

                    // Only create the writer for the next chunk if
                    // there are any remaining bytes to write. This
                    // check prevents us from creating a new empty
                    // file.
                    if !buf.is_empty() {
                        let filename = self.filename_iterator.next().ok_or_else(|| {
                            std::io::Error::new(ErrorKind::Other, "output file suffixes exhausted")
                        })?;
                        if self.settings.verbose {
                            println!("creating file {}", filename.quote());
                        }
                        self.inner =
                            platform::instantiate_current_writer(&self.settings.filter, &filename);
                    }
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
    fn new(chunk_size: u64, settings: &'a Settings) -> Option<LineChunkWriter<'a>> {
        let mut filename_iterator = FilenameIterator::new(
            &settings.prefix,
            &settings.additional_suffix,
            settings.suffix_length,
            settings.suffix_type,
        );
        let filename = filename_iterator.next()?;
        if settings.verbose {
            println!("creating file {}", filename.quote());
        }
        let inner = platform::instantiate_current_writer(&settings.filter, &filename);
        Some(LineChunkWriter {
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
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // If the number of lines in `buf` exceeds the number of lines
        // remaining in the current chunk, we will need to write to
        // multiple different underlying writers. In that case, each
        // iteration of this loop writes to the underlying writer that
        // corresponds to the current chunk number.
        let mut prev = 0;
        let mut total_bytes_written = 0;
        for i in memchr::memchr_iter(b'\n', buf) {
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
                self.inner = platform::instantiate_current_writer(&self.settings.filter, &filename);
                self.num_lines_remaining_in_current_chunk = self.chunk_size;
            }

            // Write the line, starting from *after* the previous
            // newline character and ending *after* the current
            // newline character.
            let n = self.inner.write(&buf[prev..i + 1])?;
            total_bytes_written += n;
            prev = i + 1;
            self.num_lines_remaining_in_current_chunk -= 1;
        }

        let n = self.inner.write(&buf[prev..buf.len()])?;
        total_bytes_written += n;
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
    fn new(chunk_size: u64, settings: &'a Settings) -> Option<LineBytesChunkWriter<'a>> {
        let mut filename_iterator = FilenameIterator::new(
            &settings.prefix,
            &settings.additional_suffix,
            settings.suffix_length,
            settings.suffix_type,
        );
        let filename = filename_iterator.next()?;
        if settings.verbose {
            println!("creating file {}", filename.quote());
        }
        let inner = platform::instantiate_current_writer(&settings.filter, &filename);
        Some(LineBytesChunkWriter {
            settings,
            chunk_size,
            num_bytes_remaining_in_current_chunk: chunk_size.try_into().unwrap(),
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
                self.inner = platform::instantiate_current_writer(&self.settings.filter, &filename);
                self.num_bytes_remaining_in_current_chunk = self.chunk_size.try_into().unwrap();
            }

            // Find the first newline character in the buffer.
            match memchr::memchr(b'\n', buf) {
                // If there is no newline character and the buffer is
                // empty, then we are done writing.
                None if buf.is_empty() => {
                    return Ok(total_bytes_written);
                }

                // If there is no newline character and the buffer is
                // not empty, then write as many bytes as we can and
                // then move on to the next chunk if necessary.
                None => {
                    let end = self.num_bytes_remaining_in_current_chunk;
                    let num_bytes_written = self.inner.write(&buf[..end])?;
                    self.num_bytes_remaining_in_current_chunk -= num_bytes_written;
                    total_bytes_written += num_bytes_written;
                    buf = &buf[num_bytes_written..];
                }

                // If there is a newline character and the line
                // (including the newline character) will fit in the
                // current chunk, then write the entire line and
                // continue to the next iteration. (See chunk 1 in the
                // example comment above.)
                Some(i) if i < self.num_bytes_remaining_in_current_chunk => {
                    let num_bytes_written = self.inner.write(&buf[..i + 1])?;
                    self.num_bytes_remaining_in_current_chunk -= num_bytes_written;
                    total_bytes_written += num_bytes_written;
                    buf = &buf[num_bytes_written..];
                }

                // If there is a newline character, the line
                // (including the newline character) will not fit in
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
                    let num_bytes_written = self.inner.write(&buf[..end])?;
                    self.num_bytes_remaining_in_current_chunk -= num_bytes_written;
                    total_bytes_written += num_bytes_written;
                    buf = &buf[num_bytes_written..];
                }

                // If there is a newline character, the line
                // (including the newline character) will not fit in
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

/// Split a file into a specific number of chunks by byte.
///
/// This function always creates one output file for each chunk, even
/// if there is an error reading or writing one of the chunks or if
/// the input file is truncated. However, if the `filter` option is
/// being used, then no files are created.
///
/// # Errors
///
/// This function returns an error if there is a problem reading from
/// `reader` or writing to one of the output files.
fn split_into_n_chunks_by_byte<R>(
    settings: &Settings,
    reader: &mut R,
    num_chunks: u64,
) -> UResult<()>
where
    R: Read,
{
    // Get the size of the input file in bytes and compute the number
    // of bytes per chunk.
    //
    // If the requested number of chunks exceeds the number of bytes
    // in the file *and* the `elide_empty_files` parameter is enabled,
    // then behave as if the number of chunks was set to the number of
    // bytes in the file. This ensures that we don't write empty
    // files. Otherwise, just write the `num_chunks - num_bytes` empty
    // files.
    let metadata = metadata(&settings.input).map_err(|_| {
        USimpleError::new(1, format!("{}: cannot determine file size", settings.input))
    })?;

    let num_bytes = metadata.len();
    let will_have_empty_files = settings.elide_empty_files && num_chunks > num_bytes;
    let (num_chunks, chunk_size) = if will_have_empty_files {
        let num_chunks = num_bytes;
        let chunk_size = 1;
        (num_chunks, chunk_size)
    } else {
        let chunk_size = (num_bytes / (num_chunks)).max(1);
        (num_chunks, chunk_size)
    };

    let num_chunks: usize = num_chunks
        .try_into()
        .map_err(|_| USimpleError::new(1, "Number of chunks too big"))?;

    // This object is responsible for creating the filename for each chunk.
    let mut filename_iterator = FilenameIterator::new(
        &settings.prefix,
        &settings.additional_suffix,
        settings.suffix_length,
        settings.suffix_type,
    );

    // Create one writer for each chunk. This will create each
    // of the underlying files (if not in `--filter` mode).
    let mut writers = vec![];
    for _ in 0..num_chunks {
        let filename = filename_iterator
            .next()
            .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
        let writer = platform::instantiate_current_writer(&settings.filter, filename.as_str());
        writers.push(writer);
    }

    // This block evaluates to an object of type `std::io::Result<()>`.
    {
        // Write `chunk_size` bytes from the reader into each writer
        // except the last.
        //
        // The last writer gets all remaining bytes so that if the number
        // of bytes in the input file was not evenly divisible by
        // `num_chunks`, we don't leave any bytes behind.
        for writer in writers.iter_mut().take(num_chunks - 1) {
            io::copy(&mut reader.by_ref().take(chunk_size), writer)?;
        }

        // Write all the remaining bytes to the last chunk.
        let i = num_chunks - 1;
        let last_chunk_size = num_bytes - (chunk_size * (num_chunks as u64 - 1));
        io::copy(&mut reader.by_ref().take(last_chunk_size), &mut writers[i])?;

        Ok(())
    }
    .map_err_context(|| "I/O error".to_string())
}

/// Split a file into a specific number of chunks by line.
///
/// This function always creates one output file for each chunk, even
/// if there is an error reading or writing one of the chunks or if
/// the input file is truncated. However, if the `filter` option is
/// being used, then no files are created.
///
/// # Errors
///
/// This function returns an error if there is a problem reading from
/// `reader` or writing to one of the output files.
///
/// # See also
///
/// * [`kth_chunk_by_line`], which splits its input in the same way,
///   but writes only one specified chunk to stdout.
fn split_into_n_chunks_by_line<R>(
    settings: &Settings,
    reader: &mut R,
    num_chunks: u64,
) -> UResult<()>
where
    R: BufRead,
{
    // Get the size of the input file in bytes and compute the number
    // of bytes per chunk.
    let metadata = metadata(&settings.input).unwrap();
    let num_bytes = metadata.len();
    let chunk_size = (num_bytes / (num_chunks as u64)) as usize;

    // This object is responsible for creating the filename for each chunk.
    let mut filename_iterator = FilenameIterator::new(
        &settings.prefix,
        &settings.additional_suffix,
        settings.suffix_length,
        settings.suffix_type,
    );

    // Create one writer for each chunk. This will create each
    // of the underlying files (if not in `--filter` mode).
    let mut writers = vec![];
    for _ in 0..num_chunks {
        let filename = filename_iterator
            .next()
            .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
        let writer = platform::instantiate_current_writer(&settings.filter, filename.as_str());
        writers.push(writer);
    }

    let mut num_bytes_remaining_in_current_chunk = chunk_size;
    let mut i = 0;
    for line_result in reader.lines() {
        let line = line_result.unwrap();
        let maybe_writer = writers.get_mut(i);
        let writer = maybe_writer.unwrap();
        let bytes = line.as_bytes();
        writer.write_all(bytes)?;
        writer.write_all(b"\n")?;

        // Add one byte for the newline character.
        let num_bytes = bytes.len() + 1;
        if num_bytes > num_bytes_remaining_in_current_chunk {
            num_bytes_remaining_in_current_chunk = chunk_size;
            i += 1;
        } else {
            num_bytes_remaining_in_current_chunk -= num_bytes;
        }
    }

    Ok(())
}

/// Print the k-th chunk of a file, splitting by line.
///
/// This function is like [`split_into_n_chunks_by_line`], but instead
/// of writing each chunk to its own file, it only writes to stdout
/// the contents of the chunk identified by `chunk_number`.
///
/// # Errors
///
/// This function returns an error if there is a problem reading from
/// `reader` or writing to one of the output files.
///
/// # See also
///
/// * [`split_into_n_chunks_by_line`], which splits its input in the
///   same way, but writes each chunk to its own file.
fn kth_chunk_by_line<R>(
    settings: &Settings,
    reader: &mut R,
    chunk_number: u64,
    num_chunks: u64,
) -> UResult<()>
where
    R: BufRead,
{
    // Get the size of the input file in bytes and compute the number
    // of bytes per chunk.
    let metadata = metadata(&settings.input).unwrap();
    let num_bytes = metadata.len();
    let chunk_size = (num_bytes / (num_chunks as u64)) as usize;

    // Write to stdout instead of to a file.
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();

    let mut num_bytes_remaining_in_current_chunk = chunk_size;
    let mut i = 0;
    for line_result in reader.lines() {
        let line = line_result?;
        let bytes = line.as_bytes();
        if i == chunk_number {
            writer.write_all(bytes)?;
            writer.write_all(b"\n")?;
        }

        // Add one byte for the newline character.
        let num_bytes = bytes.len() + 1;
        if num_bytes >= num_bytes_remaining_in_current_chunk {
            num_bytes_remaining_in_current_chunk = chunk_size;
            i += 1;
        } else {
            num_bytes_remaining_in_current_chunk -= num_bytes;
        }

        if i > chunk_number {
            break;
        }
    }

    Ok(())
}

fn split_into_n_chunks_by_line_round_robin<R>(
    settings: &Settings,
    reader: &mut R,
    num_chunks: u64,
) -> UResult<()>
where
    R: BufRead,
{
    // This object is responsible for creating the filename for each chunk.
    let mut filename_iterator = FilenameIterator::new(
        &settings.prefix,
        &settings.additional_suffix,
        settings.suffix_length,
        settings.suffix_type,
    );

    // Create one writer for each chunk. This will create each
    // of the underlying files (if not in `--filter` mode).
    let mut writers = vec![];
    for _ in 0..num_chunks {
        let filename = filename_iterator
            .next()
            .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
        let writer = platform::instantiate_current_writer(&settings.filter, filename.as_str());
        writers.push(writer);
    }

    let num_chunks: usize = num_chunks.try_into().unwrap();
    for (i, line_result) in reader.lines().enumerate() {
        let line = line_result.unwrap();
        let maybe_writer = writers.get_mut(i % num_chunks);
        let writer = maybe_writer.unwrap();
        let bytes = line.as_bytes();
        writer.write_all(bytes)?;
        writer.write_all(b"\n")?;
    }

    Ok(())
}

fn split(settings: &Settings) -> UResult<()> {
    let mut reader = BufReader::new(if settings.input == "-" {
        Box::new(stdin()) as Box<dyn Read>
    } else {
        let r = File::open(Path::new(&settings.input)).map_err_context(|| {
            format!(
                "cannot open {} for reading: No such file or directory",
                settings.input.quote()
            )
        })?;
        Box::new(r) as Box<dyn Read>
    });

    match settings.strategy {
        Strategy::Number(NumberType::Bytes(num_chunks)) => {
            split_into_n_chunks_by_byte(settings, &mut reader, num_chunks)
        }
        Strategy::Number(NumberType::Lines(num_chunks)) => {
            split_into_n_chunks_by_line(settings, &mut reader, num_chunks)
        }
        Strategy::Number(NumberType::KthLines(chunk_number, num_chunks)) => {
            // The chunk number is given as a 1-indexed number, but it
            // is a little easier to deal with a 0-indexed number.
            let chunk_number = chunk_number - 1;
            kth_chunk_by_line(settings, &mut reader, chunk_number, num_chunks)
        }
        Strategy::Number(NumberType::RoundRobin(num_chunks)) => {
            split_into_n_chunks_by_line_round_robin(settings, &mut reader, num_chunks)
        }
        Strategy::Number(_) => Err(USimpleError::new(1, "-n mode not yet fully implemented")),
        Strategy::Lines(chunk_size) => {
            let mut writer = LineChunkWriter::new(chunk_size, settings)
                .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
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
                    ErrorKind::Other => Err(USimpleError::new(1, "output file suffixes exhausted")),
                    _ => Err(uio_error!(e, "input/output error")),
                },
            }
        }
        Strategy::Bytes(chunk_size) => {
            let mut writer = ByteChunkWriter::new(chunk_size, settings)
                .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
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
                    ErrorKind::Other => Err(USimpleError::new(1, "output file suffixes exhausted")),
                    _ => Err(uio_error!(e, "input/output error")),
                },
            }
        }
        Strategy::LineBytes(chunk_size) => {
            let mut writer = LineBytesChunkWriter::new(chunk_size, settings)
                .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
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
                    ErrorKind::Other => Err(USimpleError::new(1, "output file suffixes exhausted")),
                    _ => Err(uio_error!(e, "input/output error")),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::NumberType;
    use crate::NumberTypeError;

    #[test]
    fn test_number_type_from() {
        assert_eq!(NumberType::from("123").unwrap(), NumberType::Bytes(123));
        assert_eq!(NumberType::from("l/123").unwrap(), NumberType::Lines(123));
        assert_eq!(
            NumberType::from("l/123/456").unwrap(),
            NumberType::KthLines(123, 456)
        );
        assert_eq!(
            NumberType::from("r/123").unwrap(),
            NumberType::RoundRobin(123)
        );
        assert_eq!(
            NumberType::from("r/123/456").unwrap(),
            NumberType::KthRoundRobin(123, 456)
        );
    }

    #[test]
    fn test_number_type_from_error() {
        assert_eq!(
            NumberType::from("xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("l/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("l/123/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("l/abc/456").unwrap_err(),
            NumberTypeError::ChunkNumber("abc".to_string())
        );
        // In GNU split, the number of chunks get precedence:
        //
        //     $ split -n l/abc/xyz
        //     split: invalid number of chunks: ‘xyz’
        //
        assert_eq!(
            NumberType::from("l/abc/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("r/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("r/123/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
        assert_eq!(
            NumberType::from("r/abc/456").unwrap_err(),
            NumberTypeError::ChunkNumber("abc".to_string())
        );
        // In GNU split, the number of chunks get precedence:
        //
        //     $ split -n r/abc/xyz
        //     split: invalid number of chunks: ‘xyz’
        //
        assert_eq!(
            NumberType::from("r/abc/xyz").unwrap_err(),
            NumberTypeError::NumberOfChunks("xyz".to_string())
        );
    }

    #[test]
    fn test_number_type_num_chunks() {
        assert_eq!(NumberType::from("123").unwrap().num_chunks(), 123);
        assert_eq!(NumberType::from("l/123").unwrap().num_chunks(), 123);
        assert_eq!(NumberType::from("l/123/456").unwrap().num_chunks(), 456);
        assert_eq!(NumberType::from("r/123").unwrap().num_chunks(), 123);
        assert_eq!(NumberType::from("r/123/456").unwrap().num_chunks(), 456);
    }
}
