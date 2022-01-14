//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Akira Hayakawa <ruby.wktk@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) PREFIXaa

mod filenames;
mod platform;

use crate::filenames::FilenameFactory;
use clap::{crate_version, App, Arg, ArgMatches};
use std::convert::TryFrom;
use std::env;
use std::fs::remove_file;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::parse_size::parse_size;

static OPT_BYTES: &str = "bytes";
static OPT_LINE_BYTES: &str = "line-bytes";
static OPT_LINES: &str = "lines";
static OPT_ADDITIONAL_SUFFIX: &str = "additional-suffix";
static OPT_FILTER: &str = "filter";
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

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();
    let long_usage = get_long_usage();

    let matches = uu_app()
        .usage(&usage[..])
        .after_help(&long_usage[..])
        .get_matches_from(args);

    let mut settings = Settings {
        prefix: "".to_owned(),
        numeric_suffix: false,
        suffix_length: 0,
        additional_suffix: "".to_owned(),
        input: "".to_owned(),
        filter: None,
        strategy: Strategy::Lines(1000),
        verbose: false,
    };

    settings.suffix_length = matches
        .value_of(OPT_SUFFIX_LENGTH)
        .unwrap()
        .parse()
        .unwrap_or_else(|_| panic!("Invalid number for {}", OPT_SUFFIX_LENGTH));

    settings.numeric_suffix = matches.occurrences_of(OPT_NUMERIC_SUFFIXES) > 0;
    settings.additional_suffix = matches.value_of(OPT_ADDITIONAL_SUFFIX).unwrap().to_owned();

    settings.verbose = matches.occurrences_of("verbose") > 0;
    settings.strategy = Strategy::from(&matches)?;
    settings.input = matches.value_of(ARG_INPUT).unwrap().to_owned();
    settings.prefix = matches.value_of(ARG_PREFIX).unwrap().to_owned();

    if matches.occurrences_of(OPT_FILTER) > 0 {
        if cfg!(windows) {
            // see https://github.com/rust-lang/rust/issues/29494
            return Err(USimpleError::new(
                -1,
                format!("{} is currently not supported in this platform", OPT_FILTER),
            ));
        } else {
            settings.filter = Some(matches.value_of(OPT_FILTER).unwrap().to_owned());
        }
    }

    split(settings)
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about("Create output files containing consecutive or interleaved sections of input")
        // strategy (mutually exclusive)
        .arg(
            Arg::with_name(OPT_BYTES)
                .short("b")
                .long(OPT_BYTES)
                .takes_value(true)
                .help("put SIZE bytes per output file"),
        )
        .arg(
            Arg::with_name(OPT_LINE_BYTES)
                .short("C")
                .long(OPT_LINE_BYTES)
                .takes_value(true)
                .default_value("2")
                .help("put at most SIZE bytes of lines per output file"),
        )
        .arg(
            Arg::with_name(OPT_LINES)
                .short("l")
                .long(OPT_LINES)
                .takes_value(true)
                .default_value("1000")
                .help("put NUMBER lines/records per output file"),
        )
        // rest of the arguments
        .arg(
            Arg::with_name(OPT_ADDITIONAL_SUFFIX)
                .long(OPT_ADDITIONAL_SUFFIX)
                .takes_value(true)
                .default_value("")
                .help("additional suffix to append to output file names"),
        )
        .arg(
            Arg::with_name(OPT_FILTER)
                .long(OPT_FILTER)
                .takes_value(true)
                .help("write to shell COMMAND file name is $FILE (Currently not implemented for Windows)"),
        )
        .arg(
            Arg::with_name(OPT_NUMERIC_SUFFIXES)
                .short("d")
                .long(OPT_NUMERIC_SUFFIXES)
                .takes_value(true)
                .default_value("0")
                .help("use numeric suffixes instead of alphabetic"),
        )
        .arg(
            Arg::with_name(OPT_SUFFIX_LENGTH)
                .short("a")
                .long(OPT_SUFFIX_LENGTH)
                .takes_value(true)
                .default_value(OPT_DEFAULT_SUFFIX_LENGTH)
                .help("use suffixes of length N (default 2)"),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .long(OPT_VERBOSE)
                .help("print a diagnostic just before each output file is opened"),
        )
        .arg(
            Arg::with_name(ARG_INPUT)
            .takes_value(true)
            .default_value("-")
            .index(1)
        )
        .arg(
            Arg::with_name(ARG_PREFIX)
            .takes_value(true)
            .default_value("x")
            .index(2)
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
}

impl Strategy {
    /// Parse a strategy from the command-line arguments.
    fn from(matches: &ArgMatches) -> UResult<Self> {
        // Check that the user is not specifying more than one strategy.
        //
        // Note: right now, this exact behavior cannot be handled by
        // `ArgGroup` since `ArgGroup` considers a default value `Arg`
        // as "defined".
        match (
            matches.occurrences_of(OPT_LINES),
            matches.occurrences_of(OPT_BYTES),
            matches.occurrences_of(OPT_LINE_BYTES),
        ) {
            (0, 0, 0) => Ok(Strategy::Lines(1000)),
            (1, 0, 0) => {
                let s = matches.value_of(OPT_LINES).unwrap();
                let n = parse_size(s)
                    .map_err(|e| USimpleError::new(1, format!("invalid number of lines: {}", e)))?;
                Ok(Strategy::Lines(n))
            }
            (0, 1, 0) => {
                let s = matches.value_of(OPT_BYTES).unwrap();
                let n = parse_size(s)
                    .map_err(|e| USimpleError::new(1, format!("invalid number of bytes: {}", e)))?;
                Ok(Strategy::Bytes(n))
            }
            (0, 0, 1) => {
                let s = matches.value_of(OPT_LINE_BYTES).unwrap();
                let n = parse_size(s)
                    .map_err(|e| USimpleError::new(1, format!("invalid number of bytes: {}", e)))?;
                Ok(Strategy::LineBytes(n))
            }
            _ => Err(UUsageError::new(1, "cannot split in more than one way")),
        }
    }
}

#[allow(dead_code)]
struct Settings {
    prefix: String,
    numeric_suffix: bool,
    suffix_length: usize,
    additional_suffix: String,
    input: String,
    /// When supplied, a shell command to output to instead of xaa, xab …
    filter: Option<String>,
    strategy: Strategy,
    verbose: bool, // TODO: warning: field is never read: `verbose`
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
    fn new(chunk_size: usize) -> LineSplitter {
        LineSplitter {
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
    fn new(chunk_size: usize) -> ByteSplitter {
        ByteSplitter {
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

fn split(settings: Settings) -> UResult<()> {
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

    let mut splitter: Box<dyn Splitter> = match settings.strategy {
        Strategy::Lines(chunk_size) => Box::new(LineSplitter::new(chunk_size)),
        Strategy::Bytes(chunk_size) | Strategy::LineBytes(chunk_size) => {
            Box::new(ByteSplitter::new(chunk_size))
        }
    };

    // This object is responsible for creating the filename for each chunk.
    let filename_factory = FilenameFactory::new(
        settings.prefix,
        settings.additional_suffix,
        settings.suffix_length,
        settings.numeric_suffix,
    );
    let mut fileno = 0;
    loop {
        // Get a new part file set up, and construct `writer` for it.
        let filename = filename_factory
            .make(fileno)
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

        fileno += 1;
    }
    Ok(())
}
