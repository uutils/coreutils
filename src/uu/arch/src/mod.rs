// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Command};
use uucore::{help_about, help_section};
static ABOUT: &str = help_about!("arch.md");
static SUMMARY: &str = help_section!("after help", "arch.md");

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(SUMMARY)
        .infer_long_args(true)
}
