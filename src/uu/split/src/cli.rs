// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command, ValueHint};
use std::env;
use std::ffi::OsString;
pub use uucore::{format_usage, translate};

pub const ARG_INPUT: &str = "input";
pub const ARG_PREFIX: &str = "prefix";

pub mod options {
    pub const BYTES: &str = "bytes";
    pub const LINE_BYTES: &str = "line-bytes";
    pub const LINES: &str = "lines";
    pub const ADDITIONAL_SUFFIX: &str = "additional-suffix";
    pub const FILTER: &str = "filter";
    pub const NUMBER: &str = "number";
    pub const NUMERIC_SUFFIXES: &str = "numeric-suffixes";
    pub const NUMERIC_SUFFIXES_SHORT: &str = "-d";
    pub const HEX_SUFFIXES: &str = "hex-suffixes";
    pub const HEX_SUFFIXES_SHORT: &str = "-x";
    pub const SUFFIX_LENGTH: &str = "suffix-length";
    pub const VERBOSE: &str = "verbose";
    pub const SEPARATOR: &str = "separator";
    pub const ELIDE_EMPTY_FILES: &str = "elide-empty-files";
    pub const IO_BLKSIZE: &str = "-io-blksize";
}

pub fn uu_app() -> Command {
    Command::new("split")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("split-about"))
        .after_help(translate!("split-after-help"))
        .override_usage(format_usage(&translate!("split-usage")))
        .infer_long_args(true)
        // strategy (mutually exclusive)
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long(options::BYTES)
                .allow_hyphen_values(true)
                .value_name("SIZE")
                .help(translate!("split-help-bytes")),
        )
        .arg(
            Arg::new(options::LINE_BYTES)
                .short('C')
                .long(options::LINE_BYTES)
                .allow_hyphen_values(true)
                .value_name("SIZE")
                .help(translate!("split-help-line-bytes")),
        )
        .arg(
            Arg::new(options::LINES)
                .short('l')
                .long(options::LINES)
                .allow_hyphen_values(true)
                .value_name("NUMBER")
                .default_value("1000")
                .help(translate!("split-help-lines")),
        )
        .arg(
            Arg::new(options::NUMBER)
                .short('n')
                .long(options::NUMBER)
                .allow_hyphen_values(true)
                .value_name("CHUNKS")
                .help(translate!("split-help-number")),
        )
        // rest of the arguments
        .arg(
            Arg::new(options::ADDITIONAL_SUFFIX)
                .long(options::ADDITIONAL_SUFFIX)
                .allow_hyphen_values(true)
                .value_name("SUFFIX")
                .default_value("")
                .value_parser(clap::value_parser!(OsString))
                .help(translate!("split-help-additional-suffix")),
        )
        .arg(
            Arg::new(options::FILTER)
                .long(options::FILTER)
                .allow_hyphen_values(true)
                .value_name("COMMAND")
                .value_hint(ValueHint::CommandName)
                .help(translate!("split-help-filter")),
        )
        .arg(
            Arg::new(options::ELIDE_EMPTY_FILES)
                .long(options::ELIDE_EMPTY_FILES)
                .short('e')
                .help(translate!("split-help-elide-empty-files"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMERIC_SUFFIXES_SHORT)
                .short('d')
                .action(ArgAction::SetTrue)
                .overrides_with_all([
                    options::NUMERIC_SUFFIXES,
                    options::NUMERIC_SUFFIXES_SHORT,
                    options::HEX_SUFFIXES,
                    options::HEX_SUFFIXES_SHORT,
                ])
                .help(translate!("split-help-numeric-suffixes-short")),
        )
        .arg(
            Arg::new(options::NUMERIC_SUFFIXES)
                .long(options::NUMERIC_SUFFIXES)
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    options::NUMERIC_SUFFIXES,
                    options::NUMERIC_SUFFIXES_SHORT,
                    options::HEX_SUFFIXES,
                    options::HEX_SUFFIXES_SHORT,
                ])
                .value_name("FROM")
                .help(translate!("split-help-numeric-suffixes")),
        )
        .arg(
            Arg::new(options::HEX_SUFFIXES_SHORT)
                .short('x')
                .action(ArgAction::SetTrue)
                .overrides_with_all([
                    options::NUMERIC_SUFFIXES,
                    options::NUMERIC_SUFFIXES_SHORT,
                    options::HEX_SUFFIXES,
                    options::HEX_SUFFIXES_SHORT,
                ])
                .help(translate!("split-help-hex-suffixes-short")),
        )
        .arg(
            Arg::new(options::HEX_SUFFIXES)
                .long(options::HEX_SUFFIXES)
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    options::NUMERIC_SUFFIXES,
                    options::NUMERIC_SUFFIXES_SHORT,
                    options::HEX_SUFFIXES,
                    options::HEX_SUFFIXES_SHORT,
                ])
                .value_name("FROM")
                .help(translate!("split-help-hex-suffixes")),
        )
        .arg(
            Arg::new(options::SUFFIX_LENGTH)
                .short('a')
                .long(options::SUFFIX_LENGTH)
                .allow_hyphen_values(true)
                .value_name("N")
                .help(translate!("split-help-suffix-length")),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long(options::VERBOSE)
                .help(translate!("split-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SEPARATOR)
                .short('t')
                .long(options::SEPARATOR)
                .allow_hyphen_values(true)
                .value_name("SEP")
                .action(ArgAction::Append)
                .help(translate!("split-help-separator")),
        )
        .arg(
            Arg::new(options::IO_BLKSIZE)
                .long("io-blksize")
                .alias(options::IO_BLKSIZE)
                .hide(true),
        )
        .arg(
            Arg::new(ARG_INPUT)
                .default_value("-")
                .value_hint(ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(ARG_PREFIX)
                .default_value("x")
                .value_parser(clap::value_parser!(OsString)),
        )
}
