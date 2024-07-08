// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use uucore::{help_about, help_usage};

use uucore::base_common;

pub const ABOUT: &str = help_about!("base64.md");
pub const USAGE: &str = help_usage!("base64.md");

pub fn uu_app() -> Command {
    base_common::base_app(ABOUT, USAGE)
}
