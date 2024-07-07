// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("stdbuf.md");
const USAGE: &str = help_usage!("stdbuf.md");
const LONG_HELP: &str = help_section!("after help", "stdbuf.md");

pub mod options {
    pub const INPUT: &str = "input";
    pub const INPUT_SHORT: char = 'i';
    pub const OUTPUT: &str = "output";
    pub const OUTPUT_SHORT: char = 'o';
    pub const ERROR: &str = "error";
    pub const ERROR_SHORT: char = 'e';
    pub const COMMAND: &str = "command";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .override_usage(format_usage(USAGE))
        .trailing_var_arg(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::INPUT)
                .long(options::INPUT)
                .short(options::INPUT_SHORT)
                .help("adjust standard input stream buffering")
                .value_name("MODE")
                .required_unless_present_any([options::OUTPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::OUTPUT)
                .long(options::OUTPUT)
                .short(options::OUTPUT_SHORT)
                .help("adjust standard output stream buffering")
                .value_name("MODE")
                .required_unless_present_any([options::INPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::ERROR)
                .long(options::ERROR)
                .short(options::ERROR_SHORT)
                .help("adjust standard error stream buffering")
                .value_name("MODE")
                .required_unless_present_any([options::INPUT, options::OUTPUT]),
        )
        .arg(
            Arg::new(options::COMMAND)
                .action(ArgAction::Append)
                .hide(true)
                .required(true)
                .value_hint(clap::ValueHint::CommandName),
        )
}
