// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const USAGE: &str = help_usage!("unexpand.md");
const ABOUT: &str = help_about!("unexpand.md");

pub mod options {
    pub const FILE: &str = "file";
    pub const ALL: &str = "all";
    pub const FIRST_ONLY: &str = "first-only";
    pub const TABS: &str = "tabs";
    pub const NO_UTF8: &str = "no-utf8";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("convert all blanks, instead of just initial blanks")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FIRST_ONLY)
                .long(options::FIRST_ONLY)
                .help("convert only leading sequences of blanks (overrides -a)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TABS)
                .short('t')
                .long(options::TABS)
                .help(
                    "use comma separated LIST of tab positions or have tabs N characters \
                apart instead of 8 (enables -a)",
                )
                .action(ArgAction::Append)
                .value_name("N, LIST"),
        )
        .arg(
            Arg::new(options::NO_UTF8)
                .short('U')
                .long(options::NO_UTF8)
                .help("interpret input file as 8-bit ASCII rather than UTF-8")
                .action(ArgAction::SetTrue),
        )
}
