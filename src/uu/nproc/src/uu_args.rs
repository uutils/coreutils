// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("nproc.md");
const USAGE: &str = help_usage!("nproc.md");

pub mod options {
    pub static OPT_ALL: &str = "all";
    pub static OPT_IGNORE: &str = "ignore";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_ALL)
                .long(options::OPT_ALL)
                .help("print the number of cores available to the system")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_IGNORE)
                .long(options::OPT_IGNORE)
                .value_name("N")
                .help("ignore up to N cores"),
        )
}
