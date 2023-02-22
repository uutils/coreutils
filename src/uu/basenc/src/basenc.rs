// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

//spell-checker:ignore (args) lsbf msbf

use clap::{Arg, ArgAction, Command};
use uu_base32::base_common::{self, Config, BASE_CMD_PARSE_ERROR};

use uucore::{
    encoding::Format,
    error::{UResult, UUsageError},
};

use std::io::{stdin, Read};
use uucore::error::UClapError;

use uucore::{help_about, help_section, help_usage};

const ABOUT: &str = help_about!("basenc.md");
const USAGE: &str = help_usage!("basenc.md");

const ENCODINGS: &[(&str, Format)] = &[
    ("base64", Format::Base64),
    ("base64url", Format::Base64Url),
    ("base32", Format::Base32),
    ("base32hex", Format::Base32Hex),
    ("base16", Format::Base16),
    ("base2lsbf", Format::Base2Lsbf),
    ("base2msbf", Format::Base2Msbf),
    ("z85", Format::Z85),
];



pub fn uu_app() -> Command {
    let mut command = base_common::base_app(ABOUT, USAGE);
    for encoding in ENCODINGS {
        command = command.arg(
            Arg::new(encoding.0)
                .long(encoding.0)
                .action(ArgAction::SetTrue),
        );
    }
    command
}

fn parse_cmd_args(args: impl uucore::Args) -> UResult<(Config, Format)> {
    let matches = uu_app()
        .try_get_matches_from(args.collect_lossy())
        .with_exit_code(1)?;
    let format = ENCODINGS
        .iter()
        .find(|encoding| matches.get_flag(encoding.0))
        .ok_or_else(|| UUsageError::new(BASE_CMD_PARSE_ERROR, "missing encoding type"))?
        .1;
    let config = Config::from(&matches)?;
    Ok((config, format))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (config, format) = parse_cmd_args(args)?;
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
