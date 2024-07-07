// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

static USAGE: &str = help_usage!("tac.md");
static ABOUT: &str = help_about!("tac.md");

pub mod options {
    pub static BEFORE: &str = "before";
    pub static REGEX: &str = "regex";
    pub static SEPARATOR: &str = "separator";
    pub static FILE: &str = "file";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::BEFORE)
                .short('b')
                .long(options::BEFORE)
                .help("attach the separator before instead of after")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REGEX)
                .short('r')
                .long(options::REGEX)
                .help("interpret the sequence as a regular expression")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SEPARATOR)
                .short('s')
                .long(options::SEPARATOR)
                .help("use STRING as the separator instead of newline")
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}
