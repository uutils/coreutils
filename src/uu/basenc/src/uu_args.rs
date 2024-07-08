// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use uucore::encoding::Format;
use uucore::{help_about, help_usage};

use uucore::base_common;

const ABOUT: &str = help_about!("basenc.md");
const USAGE: &str = help_usage!("basenc.md");

pub const ENCODINGS: &[(&str, Format, &str)] = &[
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
