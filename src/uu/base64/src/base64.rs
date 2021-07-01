// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

#[macro_use]
extern crate uucore;

use uu_base32::base_common;
pub use uu_base32::uu_app;

use uucore::encoding::Format;

use std::io::{stdin, Read};

static ABOUT: &str = "
 With no FILE, or when FILE is -, read standard input.

 The data are encoded as described for the base64 alphabet in RFC
 3548. When decoding, the input may contain newlines in addition
 to the bytes of the formal base64 alphabet. Use --ignore-garbage
 to attempt to recover from any other non-alphabet bytes in the
 encoded stream.
";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static BASE_CMD_PARSE_ERROR: i32 = 1;

fn usage() -> String {
    format!("{0} [OPTION]... [FILE]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let format = Format::Base64;
    let usage = usage();
    let name = executable!();
    let config_result: Result<base_common::Config, String> =
        base_common::parse_base_cmd_args(args, name, VERSION, ABOUT, &usage);
    let config = config_result.unwrap_or_else(|s| crash!(BASE_CMD_PARSE_ERROR, "{}", s));

    // Create a reference to stdin so we can return a locked stdin from
    // parse_base_cmd_args
    let stdin_raw = stdin();
    let mut input: Box<dyn Read> = base_common::get_input(&config, &stdin_raw);

    base_common::handle_input(
        &mut input,
        format,
        config.wrap_cols,
        config.ignore_garbage,
        config.decode,
        name,
    );

    0
}
