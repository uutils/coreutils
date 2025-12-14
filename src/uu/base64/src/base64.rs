// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use uu_base32::base_common;
use uucore::translate;
use uucore::{encoding::Format, error::UResult};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let config = base_common::parse_base_cmd_args(args, uu_app())?;
    let mut input = base_common::get_input(&config)?;
    base_common::handle_input(&mut input, Format::Base64, config)
}

pub fn uu_app() -> Command {
    base_common::base_app(translate!("base64-about"), translate!("base64-usage"))
}
