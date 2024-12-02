// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod base_common;

use clap::Command;
use uucore::{encoding::Format, error::UResult, help_about, help_usage};

const ABOUT: &str = help_about!("base32.md");
const USAGE: &str = help_usage!("base32.md");

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let format = Format::Base32;

    let config = base_common::parse_base_cmd_args(args, ABOUT, USAGE)?;

    let mut input = base_common::get_input(&config)?;

    base_common::handle_input(&mut input, format, config)
}

pub fn uu_app() -> Command {
    base_common::base_app(ABOUT, USAGE)
}
