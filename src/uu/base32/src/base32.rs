// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

use std::io::{stdin, Read};

use clap::Command;
use uucore::{encoding::Format, error::UResult, help_section, help_usage};

pub mod base_common;

const ABOUT: &str = help_section!("about", "base32.md");
const USAGE: &str = help_usage!("base32.md");

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let format = Format::Base32;

    let config: base_common::Config = base_common::parse_base_cmd_args(args, ABOUT, USAGE)?;

    // Create a reference to stdin so we can return a locked stdin from
    // parse_base_cmd_args
    let stdin_raw = stdin();
    let mut input: Box<dyn Read> = base_common::get_input(&config, &stdin_raw)?;

    base_common::handle_input(
        &mut input,
        format,
        config.wrap_cols,
        config.ignore_garbage,
        config.decode,
    )
}

pub fn uu_app() -> Command {
    base_common::base_app(ABOUT, USAGE)
}
