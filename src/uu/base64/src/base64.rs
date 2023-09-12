// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uu_base32::base_common;
pub use uu_base32::uu_app;

use uucore::{encoding::Format, error::UResult, help_about, help_usage};

use std::io::{stdin, Read};

const ABOUT: &str = help_about!("base64.md");
const USAGE: &str = help_usage!("base64.md");

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let format = Format::Base64;

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
