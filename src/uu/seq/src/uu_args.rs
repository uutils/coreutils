// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("seq.md");
const USAGE: &str = help_usage!("seq.md");

pub mod options {
    pub const OPT_SEPARATOR: &str = "separator";
    pub const OPT_TERMINATOR: &str = "terminator";
    pub const OPT_EQUAL_WIDTH: &str = "equal-width";
    pub const OPT_FORMAT: &str = "format";
    pub const ARG_NUMBERS: &str = "numbers";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .trailing_var_arg(true)
        .allow_negative_numbers(true)
        .infer_long_args(true)
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new(options::OPT_SEPARATOR)
                .short('s')
                .long("separator")
                .help("Separator character (defaults to \\n)"),
        )
        .arg(
            Arg::new(options::OPT_TERMINATOR)
                .short('t')
                .long("terminator")
                .help("Terminator character (defaults to \\n)"),
        )
        .arg(
            Arg::new(options::OPT_EQUAL_WIDTH)
                .short('w')
                .long("equal-width")
                .help("Equalize widths of all numbers by padding with zeros")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_FORMAT)
                .short('f')
                .long(options::OPT_FORMAT)
                .help("use printf style floating-point FORMAT"),
        )
        .arg(
            Arg::new(options::ARG_NUMBERS)
                .action(ArgAction::Append)
                .num_args(1..=3),
        )
}
