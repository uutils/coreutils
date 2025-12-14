// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore lsbf msbf

use clap::{Arg, ArgAction, Command};
use uu_base32::base_common::{self, BASE_CMD_PARSE_ERROR, Config};
use uucore::translate;
use uucore::{
    encoding::Format,
    error::{UResult, UUsageError},
};

fn get_encodings() -> Vec<(&'static str, Format, String)> {
    vec![
        ("base64", Format::Base64, translate!("basenc-help-base64")),
        (
            "base64url",
            Format::Base64Url,
            translate!("basenc-help-base64url"),
        ),
        ("base32", Format::Base32, translate!("basenc-help-base32")),
        (
            "base32hex",
            Format::Base32Hex,
            translate!("basenc-help-base32hex"),
        ),
        ("base16", Format::Base16, translate!("basenc-help-base16")),
        (
            "base2lsbf",
            Format::Base2Lsbf,
            translate!("basenc-help-base2lsbf"),
        ),
        (
            "base2msbf",
            Format::Base2Msbf,
            translate!("basenc-help-base2msbf"),
        ),
        ("z85", Format::Z85, translate!("basenc-help-z85")),
        ("base58", Format::Base58, translate!("basenc-help-base58")),
    ]
}

pub fn uu_app() -> Command {
    let encodings = get_encodings();
    let mut command = base_common::base_app(translate!("basenc-about"), translate!("basenc-usage"));

    for encoding in &encodings {
        let raw_arg = Arg::new(encoding.0)
            .long(encoding.0)
            .help(&encoding.2)
            .action(ArgAction::SetTrue);
        let overriding_arg = encodings
            .iter()
            .fold(raw_arg, |arg, enc| arg.overrides_with(enc.0));
        command = command.arg(overriding_arg);
    }
    command
}

fn parse_cmd_args(args: impl uucore::Args) -> UResult<(Config, Format)> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let encodings = get_encodings();
    let format = encodings
        .iter()
        .find(|encoding| matches.get_flag(encoding.0))
        .ok_or_else(|| {
            UUsageError::new(
                BASE_CMD_PARSE_ERROR,
                translate!("basenc-error-missing-encoding-type"),
            )
        })?
        .1;
    let config = Config::from(&matches)?;
    Ok((config, format))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (config, format) = parse_cmd_args(args)?;

    let mut input = base_common::get_input(&config)?;

    base_common::handle_input(&mut input, format, config)
}
