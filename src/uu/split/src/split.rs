//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Akira Hayakawa <ruby.wktk@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) PREFIXaa

#[macro_use]
extern crate uucore;

mod platform;

use clap::{App, Arg};
use std::env;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::{char, fs::remove_file};

static NAME: &str = "split";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_BYTES: &str = "bytes";
static OPT_LINE_BYTES: &str = "line-bytes";
static OPT_LINES: &str = "lines";
static OPT_ADDITIONAL_SUFFIX: &str = "additional-suffix";
static OPT_FILTER: &str = "filter";
static OPT_NUMERIC_SUFFIXES: &str = "numeric-suffixes";
static OPT_SUFFIX_LENGTH: &str = "suffix-length";
static OPT_DEFAULT_SUFFIX_LENGTH: usize = 2;
static OPT_VERBOSE: &str = "verbose";

static ARG_INPUT: &str = "input";
static ARG_PREFIX: &str = "prefix";

fn get_usage() -> String {
    format!("{0} [OPTION]... [INPUT [PREFIX]]", NAME)
}
fn get_long_usage() -> String {
    format!(
        "Usage:
  {0}

Output fixed-size pieces of INPUT to PREFIXaa, PREFIX ab, ...; default
size is 1000, and default PREFIX is 'x'. With no INPUT, or when INPUT is
-, read standard input.",
        get_usage()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let long_usage = get_long_usage();
    let default_suffix_length_str = OPT_DEFAULT_SUFFIX_LENGTH.to_string();

    let matches = App::new(executable!())
        .version(VERSION)
        .about("Create output files containing consecutive or interleaved sections of input")
        .usage(&usage[..])
        .after_help(&long_usage[..])
        // strategy (mutually exclusive)
        .arg(
            Arg::with_name(OPT_BYTES)
                .short("b")
                .long(OPT_BYTES)
                .takes_value(true)
                .default_value("2")
                .help("use suffixes of length N (default 2)"),
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
                .help("write to shell COMMAND file name is $FILE (Currently not implemented for Windows)"),
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
                .default_value(default_suffix_length_str.as_str())
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
        .get_matches_from(args);

    let mut settings = Settings {
        prefix: "".to_owned(),
        numeric_suffix: false,
        suffix_length: 0,
        additional_suffix: "".to_owned(),
        input: "".to_owned(),
        filter: None,
        strategy: "".to_owned(),
        strategy_param: "".to_owned(),
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
    // check that the user is not specifying more than one strategy
    // note: right now, this exact behaviour cannot be handled by ArgGroup since ArgGroup
    // considers a default value Arg as "defined"
    let explicit_strategies =
        vec![OPT_LINE_BYTES, OPT_LINES, OPT_BYTES]
            .into_iter()
            .fold(0, |count, strat| {
                if matches.occurrences_of(strat) > 0 {
                    count + 1
                } else {
                    count
                }
            });
    if explicit_strategies > 1 {
        crash!(1, "cannot split in more than one way");
    }

    // default strategy (if no strategy is passed, use this one)
    settings.strategy = String::from(OPT_LINES);
    settings.strategy_param = matches.value_of(OPT_LINES).unwrap().to_owned();
    // take any (other) defined strategy
    for strat in vec![OPT_LINE_BYTES, OPT_BYTES].into_iter() {
        if matches.occurrences_of(strat) > 0 {
            settings.strategy = String::from(strat);
            settings.strategy_param = matches.value_of(strat).unwrap().to_owned();
        }
    }

    settings.input = matches.value_of(ARG_INPUT).unwrap().to_owned();
    settings.prefix = matches.value_of(ARG_PREFIX).unwrap().to_owned();

    if matches.occurrences_of(OPT_FILTER) > 0 {
        if cfg!(windows) {
            // see https://github.com/rust-lang/rust/issues/29494
            show_error!("{} is currently not supported in this platform", OPT_FILTER);
            exit!(-1);
        } else {
            settings.filter = Some(matches.value_of(OPT_FILTER).unwrap().to_owned());
        }
    }

    split(&settings)
}

struct Settings {
    prefix: String,
    numeric_suffix: bool,
    suffix_length: usize,
    additional_suffix: String,
    input: String,
    /// When supplied, a shell command to output to instead of xaa, xab …
    filter: Option<String>,
    strategy: String,
    strategy_param: String,
    verbose: bool,
}

trait Splitter {
    // Consume as much as possible from `reader` so as to saturate `writer`.
    // Equivalent to finishing one of the part files. Returns the number of
    // bytes that have been moved.
    fn consume(
        &mut self,
        reader: &mut BufReader<Box<dyn Read>>,
        writer: &mut BufWriter<Box<dyn Write>>,
    ) -> u128;
}

struct LineSplitter {
    lines_per_split: usize,
}

impl LineSplitter {
    fn new(settings: &Settings) -> LineSplitter {
        LineSplitter {
            lines_per_split: settings
                .strategy_param
                .parse()
                .unwrap_or_else(|e| crash!(1, "invalid number of lines: {}", e)),
        }
    }
}

impl Splitter for LineSplitter {
    fn consume(
        &mut self,
        reader: &mut BufReader<Box<dyn Read>>,
        writer: &mut BufWriter<Box<dyn Write>>,
    ) -> u128 {
        let mut bytes_consumed = 0u128;
        let mut buffer = String::with_capacity(1024);
        for _ in 0..self.lines_per_split {
            let bytes_read = reader
                .read_line(&mut buffer)
                .unwrap_or_else(|_| crash!(1, "error reading bytes from input file"));
            // If we ever read 0 bytes then we know we've hit EOF.
            if bytes_read == 0 {
                return bytes_consumed;
            }

            writer
                .write_all(buffer.as_bytes())
                .unwrap_or_else(|_| crash!(1, "error writing bytes to output file"));
            // Empty out the String buffer since `read_line` appends instead of
            // replaces.
            buffer.clear();

            bytes_consumed += bytes_read as u128;
        }

        bytes_consumed
    }
}

struct ByteSplitter {
    bytes_per_split: u128,
}

impl ByteSplitter {
    fn new(settings: &Settings) -> ByteSplitter {
        // These multipliers are the same as supported by GNU coreutils.
        let modifiers: Vec<(&str, u128)> = vec![
            ("K", 1024u128),
            ("M", 1024 * 1024),
            ("G", 1024 * 1024 * 1024),
            ("T", 1024 * 1024 * 1024 * 1024),
            ("P", 1024 * 1024 * 1024 * 1024 * 1024),
            ("E", 1024 * 1024 * 1024 * 1024 * 1024 * 1024),
            ("Z", 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024),
            ("Y", 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024),
            ("KB", 1000),
            ("MB", 1000 * 1000),
            ("GB", 1000 * 1000 * 1000),
            ("TB", 1000 * 1000 * 1000 * 1000),
            ("PB", 1000 * 1000 * 1000 * 1000 * 1000),
            ("EB", 1000 * 1000 * 1000 * 1000 * 1000 * 1000),
            ("ZB", 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000),
            ("YB", 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000),
        ];

        // This sequential find is acceptable since none of the modifiers are
        // suffixes of any other modifiers, a la Huffman codes.
        let (suffix, multiplier) = modifiers
            .iter()
            .find(|(suffix, _)| settings.strategy_param.ends_with(suffix))
            .unwrap_or(&("", 1));

        // Try to parse the actual numeral.
        let n = &settings.strategy_param[0..(settings.strategy_param.len() - suffix.len())]
            .parse::<u128>()
            .unwrap_or_else(|e| crash!(1, "invalid number of bytes: {}", e));

        ByteSplitter {
            bytes_per_split: n * multiplier,
        }
    }
}

impl Splitter for ByteSplitter {
    fn consume(
        &mut self,
        reader: &mut BufReader<Box<dyn Read>>,
        writer: &mut BufWriter<Box<dyn Write>>,
    ) -> u128 {
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
            let bytes_read = reader
                .read(&mut buffer[0..bytes_desired])
                .unwrap_or_else(|_| crash!(1, "error reading bytes from input file"));
            // If we ever read 0 bytes then we know we've hit EOF.
            if bytes_read == 0 {
                return bytes_consumed;
            }

            writer
                .write_all(&buffer[0..bytes_read])
                .unwrap_or_else(|_| crash!(1, "error writing bytes to output file"));

            bytes_consumed += bytes_read as u128;
        }

        bytes_consumed
    }
}

// (1, 3) -> "aab"
#[allow(clippy::many_single_char_names)]
fn str_prefix(i: usize, width: usize) -> String {
    let mut c = "".to_owned();
    let mut n = i;
    let mut w = width;
    while w > 0 {
        w -= 1;
        let div = 26usize.pow(w as u32);
        let r = n / div;
        n -= r * div;
        c.push(char::from_u32((r as u32) + 97).unwrap());
    }
    c
}

// (1, 3) -> "001"
#[allow(clippy::many_single_char_names)]
fn num_prefix(i: usize, width: usize) -> String {
    let mut c = "".to_owned();
    let mut n = i;
    let mut w = width;
    while w > 0 {
        w -= 1;
        let div = 10usize.pow(w as u32);
        let r = n / div;
        n -= r * div;
        c.push(char::from_digit(r as u32, 10).unwrap());
    }
    c
}

fn split(settings: &Settings) -> i32 {
    let mut reader = BufReader::new(if settings.input == "-" {
        Box::new(stdin()) as Box<dyn Read>
    } else {
        let r = File::open(Path::new(&settings.input)).unwrap_or_else(|_| {
            crash!(
                1,
                "cannot open '{}' for reading: No such file or directory",
                settings.input
            )
        });
        Box::new(r) as Box<dyn Read>
    });

    let mut splitter: Box<dyn Splitter> = match settings.strategy.as_str() {
        s if s == OPT_LINES => Box::new(LineSplitter::new(settings)),
        s if (s == OPT_BYTES || s == OPT_LINE_BYTES) => Box::new(ByteSplitter::new(settings)),
        a => crash!(1, "strategy {} not supported", a),
    };

    let mut fileno = 0;
    loop {
        // Get a new part file set up, and construct `writer` for it.
        let mut filename = settings.prefix.clone();
        filename.push_str(
            if settings.numeric_suffix {
                num_prefix(fileno, settings.suffix_length)
            } else {
                str_prefix(fileno, settings.suffix_length)
            }
            .as_ref(),
        );
        filename.push_str(settings.additional_suffix.as_ref());
        let mut writer = platform::instantiate_current_writer(&settings.filter, filename.as_str());

        let bytes_consumed = splitter.consume(&mut reader, &mut writer);
        writer
            .flush()
            .unwrap_or_else(|e| crash!(1, "error flushing to output file: {}", e));

        // If we didn't write anything we should clean up the empty file, and
        // break from the loop.
        if bytes_consumed == 0 {
            // The output file is only ever created if --filter isn't used.
            // Complicated, I know...
            if settings.filter.is_none() {
                remove_file(filename)
                    .unwrap_or_else(|e| crash!(1, "error removing empty file: {}", e));
            }
            break;
        }

        fileno += 1;
    }
    0
}
