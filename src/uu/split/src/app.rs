use clap::{crate_version, App, Arg};

pub const OPT_BYTES: &str = "bytes";
pub const OPT_LINE_BYTES: &str = "line-bytes";
pub const OPT_LINES: &str = "lines";
pub const OPT_ADDITIONAL_SUFFIX: &str = "additional-suffix";
pub const OPT_FILTER: &str = "filter";
pub const OPT_NUMERIC_SUFFIXES: &str = "numeric-suffixes";
pub const OPT_SUFFIX_LENGTH: &str = "suffix-length";
pub const OPT_DEFAULT_SUFFIX_LENGTH: &str = "2";
pub const OPT_VERBOSE: &str = "verbose";

pub const ARG_INPUT: &str = "input";
pub const ARG_PREFIX: &str = "prefix";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about("Create output files containing consecutive or interleaved sections of input")
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
