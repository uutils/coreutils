// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod base_common;

use clap::Command;
use uucore::{encoding::Format, error::UResult, translate};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let format = Format::Base32;
    let (about, usage) = get_info();
    let config = base_common::parse_base_cmd_args(args, about, usage)?;
    let mut input = base_common::get_input(&config)?;
    base_common::handle_input(&mut input, format, config)
}

pub fn uu_app() -> Command {
    let (about, usage) = get_info();
    base_common::base_app(about, usage)
}

fn get_info() -> (&'static str, &'static str) {
    let about: &'static str = Box::leak(translate!("base32-about").into_boxed_str());
    let usage: &'static str = Box::leak(translate!("base32-usage").into_boxed_str());
    (about, usage)
}
