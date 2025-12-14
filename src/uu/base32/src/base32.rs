// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod base_common;

use clap::Command;
use uucore::{encoding::Format, error::UResult, translate};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let config = base_common::parse_base_cmd_args(args, uu_app())?;
    let mut input = base_common::get_input(&config)?;
    base_common::handle_input(&mut input, Format::Base32, config)
}

pub fn uu_app() -> Command {
    base_common::base_app(translate!("base32-about"), translate!("base32-usage"))
}
