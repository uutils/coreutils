// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const USAGE: &str = help_usage!("fold.md");
const ABOUT: &str = help_about!("fold.md");

pub mod options {
    pub const BYTES: &str = "bytes";
    pub const SPACES: &str = "spaces";
    pub const WIDTH: &str = "width";
    pub const FILE: &str = "file";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .long(options::BYTES)
                .short('b')
                .help(
                    "count using bytes rather than columns (meaning control characters \
                     such as newline are not treated specially)",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SPACES)
                .long(options::SPACES)
                .short('s')
                .help("break lines at word boundaries rather than a hard cut-off")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .long(options::WIDTH)
                .short('w')
                .help("set WIDTH as the maximum line width rather than 80")
                .value_name("WIDTH")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}
