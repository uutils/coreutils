// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("expand.md");
const USAGE: &str = help_usage!("expand.md");
static LONG_HELP: &str = "";

pub mod options {
    pub static TABS: &str = "tabs";
    pub static INITIAL: &str = "initial";
    pub static NO_UTF8: &str = "no-utf8";
    pub static FILES: &str = "FILES";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::INITIAL)
                .long(options::INITIAL)
                .short('i')
                .help("do not convert tabs after non blanks")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TABS)
                .long(options::TABS)
                .short('t')
                .value_name("N, LIST")
                .action(ArgAction::Append)
                .help(
                    "have tabs N characters apart, not 8 or use comma separated list \
                    of explicit tab positions",
                ),
        )
        .arg(
            Arg::new(options::NO_UTF8)
                .long(options::NO_UTF8)
                .short('U')
                .help("interpret input file as 8-bit ASCII rather than UTF-8")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath),
        )
}
