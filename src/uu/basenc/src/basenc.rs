// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//spell-checker:ignore (args) lsbf msbf

use clap::{Arg, ArgAction, Command};
use uu_base32::base_common::{self, Config, BASE_CMD_PARSE_ERROR};

use uucore::{
    encoding::Format,
    error::{UResult, UUsageError},
};

use std::io::{stdin, Read};
use uucore::error::UClapError;

use uucore::{help_about, help_usage};

const ABOUT: &str = help_about!("basenc.md");
const USAGE: &str = help_usage!("basenc.md");

const ENCODINGS: &[(&str, Format, &str)] = &[
    ("base64", Format::Base64, "same as 'base64' program"),
    ("base64url", Format::Base64Url, "file- and url-safe base64"),
    ("base32", Format::Base32, "same as 'base32' program"),
    (
        "base32hex",
        Format::Base32Hex,
        "extended hex alphabet base32",
    ),
    ("base16", Format::Base16, "hex encoding"),
    (
        "base2lsbf",
        Format::Base2Lsbf,
        "bit string with least significant bit (lsb) first",
    ),
    (
        "base2msbf",
        Format::Base2Msbf,
        "bit string with most significant bit (msb) first",
    ),
    (
        "z85",
        Format::Z85,
        "ascii85-like encoding;\n\
        when encoding, input length must be a multiple of 4;\n\
        when decoding, input length must be a multiple of 5",
    ),
];

pub fn uu_app() -> Command {
    let mut command = base_common::base_app(ABOUT, USAGE);
    for encoding in ENCODINGS {
        let raw_arg = Arg::new(encoding.0)
            .long(encoding.0)
            .help(encoding.2)
            .action(ArgAction::SetTrue);
        let overriding_arg = ENCODINGS
            .iter()
            .fold(raw_arg, |arg, enc| arg.overrides_with(enc.0));
        command = command.arg(overriding_arg);
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
