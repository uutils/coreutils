// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("join.md");
const USAGE: &str = help_usage!("join.md");

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new("a")
                .short('a')
                .action(ArgAction::Append)
                .num_args(1)
                .value_parser(["1", "2"])
                .value_name("FILENUM")
                .help(
                    "also print unpairable lines from file FILENUM, where
FILENUM is 1 or 2, corresponding to FILE1 or FILE2",
                ),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .action(ArgAction::Append)
                .num_args(1)
                .value_parser(["1", "2"])
                .value_name("FILENUM")
                .help("like -a FILENUM, but suppress joined output lines"),
        )
        .arg(
            Arg::new("e")
                .short('e')
                .value_name("EMPTY")
                .help("replace missing input fields with EMPTY"),
        )
        .arg(
            Arg::new("i")
                .short('i')
                .long("ignore-case")
                .help("ignore differences in case when comparing fields")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("j")
                .short('j')
                .value_name("FIELD")
                .help("equivalent to '-1 FIELD -2 FIELD'"),
        )
        .arg(
            Arg::new("o")
                .short('o')
                .value_name("FORMAT")
                .help("obey FORMAT while constructing output line"),
        )
        .arg(
            Arg::new("t")
                .short('t')
                .value_name("CHAR")
                .value_parser(ValueParser::os_string())
                .help("use CHAR as input and output field separator"),
        )
        .arg(
            Arg::new("1")
                .short('1')
                .value_name("FIELD")
                .help("join on this FIELD of file 1"),
        )
        .arg(
            Arg::new("2")
                .short('2')
                .value_name("FIELD")
                .help("join on this FIELD of file 2"),
        )
        .arg(
            Arg::new("check-order")
                .long("check-order")
                .help(
                    "check that the input is correctly sorted, \
             even if all input lines are pairable",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("nocheck-order")
                .long("nocheck-order")
                .help("do not check that the input is correctly sorted")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("header")
                .long("header")
                .help(
                    "treat the first line in each file as field headers, \
             print them without trying to pair them",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("z")
                .short('z')
                .long("zero-terminated")
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("file1")
                .required(true)
                .value_name("FILE1")
                .value_hint(clap::ValueHint::FilePath)
                .hide(true),
        )
        .arg(
            Arg::new("file2")
                .required(true)
                .value_name("FILE2")
                .value_hint(clap::ValueHint::FilePath)
                .hide(true),
        )
}
