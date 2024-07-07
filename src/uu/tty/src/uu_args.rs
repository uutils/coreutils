// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("tty.md");
const USAGE: &str = help_usage!("tty.md");

pub mod options {
    pub const SILENT: &str = "silent";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SILENT)
                .long(options::SILENT)
                .visible_alias("quiet")
                .short('s')
                .help("print nothing, only return an exit status")
                .action(ArgAction::SetTrue),
        )
}
