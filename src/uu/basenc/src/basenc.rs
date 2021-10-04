// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

//spell-checker:ignore (args) lsbf msbf

use clap::{App, Arg};
use uu_base32::base_common::{self, Config, BASE_CMD_PARSE_ERROR};

use uucore::{
    encoding::Format,
    error::{UResult, UUsageError},
    InvalidEncodingHandling,
};

use std::io::{stdin, Read};

static ABOUT: &str = "
 With no FILE, or when FILE is -, read standard input.

 When decoding, the input may contain newlines in addition to the bytes of
 the formal alphabet. Use --ignore-garbage to attempt to recover
 from any other non-alphabet bytes in the encoded stream.
";

const ENCODINGS: &[(&str, Format)] = &[
    ("base64", Format::Base64),
    ("base64url", Format::Base64Url),
    ("base32", Format::Base32),
    ("base32hex", Format::Base32Hex),
    ("base16", Format::Base16),
    ("base2lsbf", Format::Base2Lsbf),
    ("base2msbf", Format::Base2Msbf),
    ("z85", Format::Z85),
    // common abbreviations. TODO: once we have clap 3.0 we can use `AppSettings::InferLongArgs` to get all abbreviations automatically
    ("base2l", Format::Base2Lsbf),
    ("base2m", Format::Base2Msbf),
];

fn usage() -> String {
    format!("{0} [OPTION]... [FILE]", uucore::execution_phrase())
}

pub fn uu_app() -> App<'static, 'static> {
    let mut app = base_common::base_app(ABOUT);
    for encoding in ENCODINGS {
        app = app.arg(Arg::with_name(encoding.0).long(encoding.0));
    }
    app
}

fn parse_cmd_args(args: impl uucore::Args) -> UResult<(Config, Format)> {
    let usage = usage();
    let matches = uu_app().usage(&usage[..]).get_matches_from(
        args.collect_str(InvalidEncodingHandling::ConvertLossy)
            .accept_any(),
    );
    let format = ENCODINGS
        .iter()
        .find(|encoding| matches.is_present(encoding.0))
        .ok_or_else(|| UUsageError::new(BASE_CMD_PARSE_ERROR, "missing encoding type"))?
        .1;
    let config = Config::from(&matches)?;
    Ok((config, format))
}

#[uucore_procs::gen_uumain]
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
