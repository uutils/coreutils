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
use std::char;
use std::env;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

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

struct SplitControl {
    current_line: String,   // Don't touch
    request_new_file: bool, // Splitter implementation requests new file
}

trait Splitter {
    // Consume the current_line and return the consumed string
    fn consume(&mut self, _: &mut SplitControl) -> String;
}

struct LineSplitter {
    saved_lines_to_write: usize,
    lines_to_write: usize,
}

impl LineSplitter {
    fn new(settings: &Settings) -> LineSplitter {
        let n = match settings.strategy_param.parse() {
            Ok(a) => a,
            Err(e) => crash!(1, "invalid number of lines: {}", e),
        };
        LineSplitter {
            saved_lines_to_write: n,
            lines_to_write: n,
        }
    }
}

impl Splitter for LineSplitter {
    fn consume(&mut self, control: &mut SplitControl) -> String {
        self.lines_to_write -= 1;
        if self.lines_to_write == 0 {
            self.lines_to_write = self.saved_lines_to_write;
            control.request_new_file = true;
        }
        control.current_line.clone()
    }
}

struct ByteSplitter {
    saved_bytes_to_write: usize,
    bytes_to_write: usize,
    break_on_line_end: bool,
    require_whole_line: bool,
}

impl ByteSplitter {
    fn new(settings: &Settings) -> ByteSplitter {
        let mut strategy_param: Vec<char> = settings.strategy_param.chars().collect();
        let suffix = strategy_param.pop().unwrap();
        let multiplier = match suffix {
            '0'..='9' => 1usize,
            'b' => 512usize,
            'k' => 1024usize,
            'm' => 1024usize * 1024usize,
            _ => crash!(1, "invalid number of bytes"),
        };
        let n = if suffix.is_alphabetic() {
            match strategy_param
                .iter()
                .cloned()
                .collect::<String>()
                .parse::<usize>()
            {
                Ok(a) => a,
                Err(e) => crash!(1, "invalid number of bytes: {}", e),
            }
        } else {
            match settings.strategy_param.parse::<usize>() {
                Ok(a) => a,
                Err(e) => crash!(1, "invalid number of bytes: {}", e),
            }
        };
        ByteSplitter {
            saved_bytes_to_write: n * multiplier,
            bytes_to_write: n * multiplier,
            break_on_line_end: settings.strategy == "b",
            require_whole_line: false,
        }
    }
}

impl Splitter for ByteSplitter {
    fn consume(&mut self, control: &mut SplitControl) -> String {
        let line = control.current_line.clone();
        let n = std::cmp::min(line.chars().count(), self.bytes_to_write);
        if self.require_whole_line && n < line.chars().count() {
            self.bytes_to_write = self.saved_bytes_to_write;
            control.request_new_file = true;
            self.require_whole_line = false;
            return "".to_owned();
        }
        self.bytes_to_write -= n;
        if n == 0 {
            self.bytes_to_write = self.saved_bytes_to_write;
            control.request_new_file = true;
        }
        if self.break_on_line_end && n == line.chars().count() {
            self.require_whole_line = self.break_on_line_end;
        }
        line[..n].to_owned()
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
        let r = match File::open(Path::new(&settings.input)) {
            Ok(a) => a,
            Err(_) => crash!(
                1,
                "cannot open '{}' for reading: No such file or directory",
                settings.input
            ),
        };
        Box::new(r) as Box<dyn Read>
    });

    let mut splitter: Box<dyn Splitter> = match settings.strategy.as_str() {
        s if s == OPT_LINES => Box::new(LineSplitter::new(settings)),
        s if (s == OPT_BYTES || s == OPT_LINE_BYTES) => Box::new(ByteSplitter::new(settings)),
        a => crash!(1, "strategy {} not supported", a),
    };

    let mut control = SplitControl {
        current_line: "".to_owned(), // Request new line
        request_new_file: true,      // Request new file
    };

    let mut writer = BufWriter::new(Box::new(stdout()) as Box<dyn Write>);
    let mut fileno = 0;
    loop {
        if control.current_line.chars().count() == 0 {
            match reader.read_line(&mut control.current_line) {
                Ok(0) | Err(_) => break,
                _ => {}
            }
        }
        if control.request_new_file {
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

            crash_if_err!(1, writer.flush());
            fileno += 1;
            writer = platform::instantiate_current_writer(&settings.filter, filename.as_str());
            control.request_new_file = false;
            if settings.verbose {
                println!("creating file '{}'", filename);
            }
        }

        let consumed = splitter.consume(&mut control);
        crash_if_err!(1, writer.write_all(consumed.as_bytes()));

        let advance = consumed.chars().count();
        let clone = control.current_line.clone();
        let sl = clone;
        control.current_line = sl[advance..sl.chars().count()].to_owned();
    }
    0
}
