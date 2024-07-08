// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("printenv.md");
const USAGE: &str = help_usage!("printenv.md");

pub mod options {
    pub static OPT_NULL: &str = "null";

    pub static ARG_VARIABLES: &str = "variables";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_NULL)
                .short('0')
                .long(options::OPT_NULL)
                .help("end each output line with 0 byte rather than newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARG_VARIABLES)
                .action(ArgAction::Append)
                .num_args(1..),
        )
}
