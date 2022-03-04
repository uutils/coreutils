// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

use uu_base32::base_common;
pub use uu_base32::uu_app;

use uucore::{encoding::Format, error::UResult};

use std::io::{stdin, Read};

static ABOUT: &str = "\
With no FILE, or when FILE is -, read standard input.

The data are encoded as described for the base64 alphabet in RFC
3548. When decoding, the input may contain newlines in addition
to the bytes of the formal base64 alphabet. Use --ignore-garbage
to attempt to recover from any other non-alphabet bytes in the
encoded stream.
";

const USAGE: &str = "{0} [OPTION]... [FILE]";

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
