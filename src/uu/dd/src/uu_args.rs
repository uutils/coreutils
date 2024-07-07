// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("dd.md");
const AFTER_HELP: &str = help_section!("after help", "dd.md");
const USAGE: &str = help_usage!("dd.md");

pub mod options {
    pub const OPERANDS: &str = "operands";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .arg(Arg::new(options::OPERANDS).num_args(1..))
}
