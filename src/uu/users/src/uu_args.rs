// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_version, Arg, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("users.md");
const USAGE: &str = help_usage!("users.md");

pub mod options {
    pub const ARG_FILES: &str = "files";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ARG_FILES)
                .num_args(1)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string()),
        )
}
