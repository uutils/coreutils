// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command, ValueHint};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("split.md");
const USAGE: &str = help_usage!("split.md");
const AFTER_HELP: &str = help_section!("after help", "split.md");

pub mod options {
    pub static OPT_BYTES: &str = "bytes";
    pub static OPT_LINE_BYTES: &str = "line-bytes";
    pub static OPT_LINES: &str = "lines";
    pub static OPT_ADDITIONAL_SUFFIX: &str = "additional-suffix";
    pub static OPT_FILTER: &str = "filter";
    pub static OPT_NUMBER: &str = "number";
    pub static OPT_NUMERIC_SUFFIXES: &str = "numeric-suffixes";
    pub static OPT_NUMERIC_SUFFIXES_SHORT: &str = "-d";
    pub static OPT_HEX_SUFFIXES: &str = "hex-suffixes";
    pub static OPT_HEX_SUFFIXES_SHORT: &str = "-x";
    pub static OPT_SUFFIX_LENGTH: &str = "suffix-length";
    pub static OPT_VERBOSE: &str = "verbose";
    pub static OPT_SEPARATOR: &str = "separator";
    pub static OPT_ELIDE_EMPTY_FILES: &str = "elide-empty-files";
    pub static OPT_IO_BLKSIZE: &str = "-io-blksize";

    pub static ARG_INPUT: &str = "input";
    pub static ARG_PREFIX: &str = "prefix";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        // strategy (mutually exclusive)
        .arg(
            Arg::new(options::OPT_BYTES)
                .short('b')
                .long(options::OPT_BYTES)
                .allow_hyphen_values(true)
                .value_name("SIZE")
                .help("put SIZE bytes per output file"),
        )
        .arg(
            Arg::new(options::OPT_LINE_BYTES)
                .short('C')
                .long(options::OPT_LINE_BYTES)
                .allow_hyphen_values(true)
                .value_name("SIZE")
                .help("put at most SIZE bytes of lines per output file"),
        )
        .arg(
            Arg::new(options::OPT_LINES)
                .short('l')
                .long(options::OPT_LINES)
                .allow_hyphen_values(true)
                .value_name("NUMBER")
                .default_value("1000")
                .help("put NUMBER lines/records per output file"),
        )
        .arg(
            Arg::new(options::OPT_NUMBER)
                .short('n')
                .long(options::OPT_NUMBER)
                .allow_hyphen_values(true)
                .value_name("CHUNKS")
                .help("generate CHUNKS output files; see explanation below"),
        )
        // rest of the arguments
        .arg(
            Arg::new(options::OPT_ADDITIONAL_SUFFIX)
                .long(options::OPT_ADDITIONAL_SUFFIX)
                .allow_hyphen_values(true)
                .value_name("SUFFIX")
                .default_value("")
                .help("additional SUFFIX to append to output file names"),
        )
        .arg(
            Arg::new(options::OPT_FILTER)
                .long(options::OPT_FILTER)
                .allow_hyphen_values(true)
                .value_name("COMMAND")
                .value_hint(ValueHint::CommandName)
                .help(
                    "write to shell COMMAND; file name is $FILE (Currently not implemented for Windows)",
                ),
        )
        .arg(
            Arg::new(options::OPT_ELIDE_EMPTY_FILES)
                .long(options::OPT_ELIDE_EMPTY_FILES)
                .short('e')
                .help("do not generate empty output files with '-n'")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_NUMERIC_SUFFIXES_SHORT)
                .short('d')
                .action(ArgAction::SetTrue)
                .overrides_with_all([
                    options::OPT_NUMERIC_SUFFIXES,
                    options::OPT_NUMERIC_SUFFIXES_SHORT,
                    options::OPT_HEX_SUFFIXES,
                    options::OPT_HEX_SUFFIXES_SHORT
                ])
                .help("use numeric suffixes starting at 0, not alphabetic"),
        )
        .arg(
            Arg::new(options::OPT_NUMERIC_SUFFIXES)
                .long(options::OPT_NUMERIC_SUFFIXES)
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    options::OPT_NUMERIC_SUFFIXES,
                    options:: OPT_NUMERIC_SUFFIXES_SHORT,
                    options:: OPT_HEX_SUFFIXES,
                    options:: OPT_HEX_SUFFIXES_SHORT
                ])
                .value_name("FROM")
                .help("same as -d, but allow setting the start value"),
        )
        .arg(
            Arg::new(options::OPT_HEX_SUFFIXES_SHORT)
                .short('x')
                .action(ArgAction::SetTrue)
                .overrides_with_all([
                    options::OPT_NUMERIC_SUFFIXES,
                    options::OPT_NUMERIC_SUFFIXES_SHORT,
                    options::OPT_HEX_SUFFIXES,
                    options:: OPT_HEX_SUFFIXES_SHORT
                ])
                .help("use hex suffixes starting at 0, not alphabetic"),
        )
        .arg(
            Arg::new(options::OPT_HEX_SUFFIXES)
                .long(options::OPT_HEX_SUFFIXES)
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    options:: OPT_NUMERIC_SUFFIXES,
                    options::OPT_NUMERIC_SUFFIXES_SHORT,
                    options:: OPT_HEX_SUFFIXES,
                    options::OPT_HEX_SUFFIXES_SHORT
                ])
                .value_name("FROM")
                .help("same as -x, but allow setting the start value"),
        )
        .arg(
            Arg::new(options::OPT_SUFFIX_LENGTH)
                .short('a')
                .long(options::OPT_SUFFIX_LENGTH)
                .allow_hyphen_values(true)
                .value_name("N")
                .help("generate suffixes of length N (default 2)"),
        )
        .arg(
            Arg::new(options::OPT_VERBOSE)
                .long(options::OPT_VERBOSE)
                .help("print a diagnostic just before each output file is opened")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_SEPARATOR)
                .short('t')
                .long(options::OPT_SEPARATOR)
                .allow_hyphen_values(true)
                .value_name("SEP")
                .action(ArgAction::Append)
                .help("use SEP instead of newline as the record separator; '\\0' (zero) specifies the NUL character"),
        )
        .arg(
            Arg::new(options::OPT_IO_BLKSIZE)
                .long("io-blksize")
                .alias(options::OPT_IO_BLKSIZE)
                .hide(true),
        )
        .arg(
            Arg::new(options::ARG_INPUT)
                .default_value("-")
                .value_hint(ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ARG_PREFIX)
                .default_value("x")
        )
}
