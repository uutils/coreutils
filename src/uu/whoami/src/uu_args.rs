// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Command};
use uucore::{format_usage, help_about, help_usage};

pub const ABOUT: &str = help_about!("whoami.md");
pub const USAGE: &str = help_usage!("whoami.md");

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
}
