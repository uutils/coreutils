//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Akira Hayakawa <ruby.wktk@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) PREFIXaa

mod filenames;
mod number;
mod platform;

use crate::filenames::FilenameIterator;
use clap::{crate_version, App, AppSettings, Arg, ArgMatches};
use std::convert::TryFrom;
use std::env;
use std::fmt;
use std::fs::{metadata, remove_file, File};
use std::io::{stdin, BufRead, BufReader, BufWriter, Read, Write};
use std::num::ParseIntError;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::parse_size::{parse_size, ParseSizeError};

static OPT_BYTES: &str = "bytes";
static OPT_LINE_BYTES: &str = "line-bytes";
static OPT_LINES: &str = "lines";
static OPT_ADDITIONAL_SUFFIX: &str = "additional-suffix";
static OPT_FILTER: &str = "filter";
static OPT_NUMBER: &str = "number";
static OPT_NUMERIC_SUFFIXES: &str = "numeric-suffixes";
static OPT_SUFFIX_LENGTH: &str = "suffix-length";
static OPT_DEFAULT_SUFFIX_LENGTH: &str = "0";
static OPT_VERBOSE: &str = "verbose";

static ARG_INPUT: &str = "input";
static ARG_PREFIX: &str = "prefix";

fn usage() -> String {
    format!(
        "{0} [OPTION]... [INPUT [PREFIX]]",
        uucore::execution_phrase()
    )
}
fn get_long_usage() -> String {
    format!(
        "Usage:
  {0}

Output fixed-size pieces of INPUT to PREFIXaa, PREFIX ab, ...; default
size is 1000, and default PREFIX is 'x'. With no INPUT, or when INPUT is
-, read standard input.",
        usage()
    )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();
    let long_usage = get_long_usage();
    let matches = uu_app()
        .override_usage(&usage[..])
        .after_help(&long_usage[..])
        .get_matches_from(args);
    match Settings::from(&matches) {
        Ok(settings) => split(&settings),
        Err(e) if e.requires_usage() => Err(UUsageError::new(1, format!("{}", e))),
        Err(e) => Err(USimpleError::new(1, format!("{}", e))),
    }
}

pub fn uu_app<'a>() -> App<'a> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about("Create output files containing consecutive or interleaved sections of input")
        .setting(AppSettings::InferLongArgs)
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
            Arg::new(OPT_VERBOSE)
                .long(OPT_VERBOSE)
                .help("print a diagnostic just before each output file is opened"),
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

/// The strategy for breaking up the input file into chunks.
enum Strategy {
    /// Each chunk has the specified number of lines.
    Lines(usize),

    /// Each chunk has the specified number of bytes.
    Bytes(usize),

    /// Each chunk has as many lines as possible without exceeding the
    /// specified number of bytes.
    LineBytes(usize),

    /// Split the file into this many chunks.
    Number(usize),
}

/// An error when parsing a chunking strategy from command-line arguments.
enum StrategyError {
    /// Invalid number of lines.
    Lines(ParseSizeError),

    /// Invalid number of bytes.
    Bytes(ParseSizeError),

    /// Invalid number of chunks.
    NumberOfChunks(ParseIntError),

    /// Multiple chunking strategies were specified (but only one should be).
    MultipleWays,
}

impl fmt::Display for StrategyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Lines(e) => write!(f, "invalid number of lines: {}", e),
            Self::Bytes(e) => write!(f, "invalid number of bytes: {}", e),
            Self::NumberOfChunks(e) => write!(f, "invalid number of chunks: {}", e),
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
                let n = s.parse::<usize>().map_err(StrategyError::NumberOfChunks)?;
                Ok(Self::Number(n))
            }
            _ => Err(StrategyError::MultipleWays),
        }
    }
}

/// Parameters that control how a file gets split.
///
/// You can convert an [`ArgMatches`] instance into a [`Settings`]
/// instance by calling [`Settings::from`].
struct Settings {
    prefix: String,
    numeric_suffix: bool,
    suffix_length: usize,
    additional_suffix: String,
    input: String,
    /// When supplied, a shell command to output to instead of xaa, xab …
    filter: Option<String>,
    strategy: Strategy,
    verbose: bool,
}

/// An error when parsing settings from command-line arguments.
enum SettingsError {
    /// Invalid chunking strategy.
    Strategy(StrategyError),

    /// Invalid suffix length parameter.
    SuffixLength(String),

    /// The `--filter` option is not supported on Windows.
    #[cfg(windows)]
    NotSupported,
}

impl SettingsError {
    /// Whether the error demands a usage message.
    fn requires_usage(&self) -> bool {
        matches!(self, Self::Strategy(StrategyError::MultipleWays))
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Strategy(e) => e.fmt(f),
            Self::SuffixLength(s) => write!(f, "invalid suffix length: {}", s.quote()),
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
        let suffix_length_str = matches.value_of(OPT_SUFFIX_LENGTH).unwrap();
        let result = Self {
            suffix_length: suffix_length_str
                .parse()
                .map_err(|_| SettingsError::SuffixLength(suffix_length_str.to_string()))?,
            numeric_suffix: matches.occurrences_of(OPT_NUMERIC_SUFFIXES) > 0,
            additional_suffix: matches.value_of(OPT_ADDITIONAL_SUFFIX).unwrap().to_owned(),
            verbose: matches.occurrences_of("verbose") > 0,
            strategy: Strategy::from(matches).map_err(SettingsError::Strategy)?,
            input: matches.value_of(ARG_INPUT).unwrap().to_owned(),
            prefix: matches.value_of(ARG_PREFIX).unwrap().to_owned(),
            filter: matches.value_of(OPT_FILTER).map(|s| s.to_owned()),
        };
        #[cfg(windows)]
        if result.filter.is_some() {
            // see https://github.com/rust-lang/rust/issues/29494
            return Err(SettingsError::NotSupported);
        }

        Ok(result)
    }
}

trait Splitter {
    // Consume as much as possible from `reader` so as to saturate `writer`.
    // Equivalent to finishing one of the part files. Returns the number of
    // bytes that have been moved.
    fn consume(
        &mut self,
        reader: &mut BufReader<Box<dyn Read>>,
        writer: &mut BufWriter<Box<dyn Write>>,
    ) -> std::io::Result<u128>;
}

struct LineSplitter {
    lines_per_split: usize,
}

impl LineSplitter {
    fn new(chunk_size: usize) -> Self {
        Self {
            lines_per_split: chunk_size,
        }
    }
}

impl Splitter for LineSplitter {
    fn consume(
        &mut self,
        reader: &mut BufReader<Box<dyn Read>>,
        writer: &mut BufWriter<Box<dyn Write>>,
    ) -> std::io::Result<u128> {
        let mut bytes_consumed = 0u128;
        let mut buffer = String::with_capacity(1024);
        for _ in 0..self.lines_per_split {
            let bytes_read = reader.read_line(&mut buffer)?;
            // If we ever read 0 bytes then we know we've hit EOF.
            if bytes_read == 0 {
                return Ok(bytes_consumed);
            }

            writer.write_all(buffer.as_bytes())?;
            // Empty out the String buffer since `read_line` appends instead of
            // replaces.
            buffer.clear();

            bytes_consumed += bytes_read as u128;
        }

        Ok(bytes_consumed)
    }
}

struct ByteSplitter {
    bytes_per_split: u128,
}

impl ByteSplitter {
    fn new(chunk_size: usize) -> Self {
        Self {
            bytes_per_split: u128::try_from(chunk_size).unwrap(),
        }
    }
}

impl Splitter for ByteSplitter {
    fn consume(
        &mut self,
        reader: &mut BufReader<Box<dyn Read>>,
        writer: &mut BufWriter<Box<dyn Write>>,
    ) -> std::io::Result<u128> {
        // We buffer reads and writes. We proceed until `bytes_consumed` is
        // equal to `self.bytes_per_split` or we reach EOF.
        let mut bytes_consumed = 0u128;
        const BUFFER_SIZE: usize = 1024;
        let mut buffer = [0u8; BUFFER_SIZE];
        while bytes_consumed < self.bytes_per_split {
            // Don't overshoot `self.bytes_per_split`! Note: Using std::cmp::min
            // doesn't really work since we have to get types to match which
            // can't be done in a way that keeps all conversions safe.
            let bytes_desired = if (BUFFER_SIZE as u128) <= self.bytes_per_split - bytes_consumed {
                BUFFER_SIZE
            } else {
                // This is a safe conversion since the difference must be less
                // than BUFFER_SIZE in this branch.
                (self.bytes_per_split - bytes_consumed) as usize
            };
            let bytes_read = reader.read(&mut buffer[0..bytes_desired])?;
            // If we ever read 0 bytes then we know we've hit EOF.
            if bytes_read == 0 {
                return Ok(bytes_consumed);
            }

            writer.write_all(&buffer[0..bytes_read])?;

            bytes_consumed += bytes_read as u128;
        }

        Ok(bytes_consumed)
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
    num_chunks: usize,
) -> UResult<()>
where
    R: Read,
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
        settings.numeric_suffix,
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
        // Re-use the buffer to avoid re-allocating a `Vec` on each
        // iteration. The contents will be completely overwritten each
        // time we call `read_exact()`.
        //
        // The last writer gets all remaining bytes so that if the number
        // of bytes in the input file was not evenly divisible by
        // `num_chunks`, we don't leave any bytes behind.
        let mut buf = vec![0u8; chunk_size];
        for writer in writers.iter_mut().take(num_chunks - 1) {
            reader.read_exact(&mut buf)?;
            writer.write_all(&buf)?;
        }

        // Write all the remaining bytes to the last chunk.
        //
        // To do this, we resize our buffer to have the necessary number
        // of bytes.
        let i = num_chunks - 1;
        let last_chunk_size = num_bytes as usize - (chunk_size * (num_chunks - 1));
        buf.resize(last_chunk_size, 0);

        reader.read_exact(&mut buf)?;
        writers[i].write_all(&buf)?;

        Ok(())
    }
    .map_err_context(|| "I/O error".to_string())
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

    if let Strategy::Number(num_chunks) = settings.strategy {
        return split_into_n_chunks_by_byte(settings, &mut reader, num_chunks);
    }

    let mut splitter: Box<dyn Splitter> = match settings.strategy {
        Strategy::Lines(chunk_size) => Box::new(LineSplitter::new(chunk_size)),
        Strategy::Bytes(chunk_size) | Strategy::LineBytes(chunk_size) => {
            Box::new(ByteSplitter::new(chunk_size))
        }
        _ => unreachable!(),
    };

    // This object is responsible for creating the filename for each chunk.
    let mut filename_iterator = FilenameIterator::new(
        &settings.prefix,
        &settings.additional_suffix,
        settings.suffix_length,
        settings.numeric_suffix,
    );
    loop {
        // Get a new part file set up, and construct `writer` for it.
        let filename = filename_iterator
            .next()
            .ok_or_else(|| USimpleError::new(1, "output file suffixes exhausted"))?;
        let mut writer = platform::instantiate_current_writer(&settings.filter, filename.as_str());

        let bytes_consumed = splitter
            .consume(&mut reader, &mut writer)
            .map_err_context(|| "input/output error".to_string())?;
        writer
            .flush()
            .map_err_context(|| "error flushing to output file".to_string())?;

        // If we didn't write anything we should clean up the empty file, and
        // break from the loop.
        if bytes_consumed == 0 {
            // The output file is only ever created if --filter isn't used.
            // Complicated, I know...
            if settings.filter.is_none() {
                remove_file(filename)
                    .map_err_context(|| "error removing empty file".to_string())?;
            }
            break;
        }

        // TODO It is silly to have the "creating file" message here
        // after the file has been already created. However, because
        // of the way the main loop has been written, an extra file
        // gets created and then deleted in the last iteration of the
        // loop. So we need to make sure we are not in that case when
        // printing this message.
        //
        // This is only here temporarily while we make some
        // improvements to the architecture of the main loop in this
        // function. In the future, it will move to a more appropriate
        // place---at the point where the file is actually created.
        if settings.verbose {
            println!("creating file {}", filename.quote());
        }
    }
    Ok(())
}
