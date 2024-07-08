// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, value_parser, Arg, Command};
use uucore::display::Quotable;
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("mknod.md");
const USAGE: &str = help_usage!("mknod.md");
const AFTER_HELP: &str = help_section!("after help", "mknod.md");

#[derive(Clone, PartialEq)]
pub enum FileType {
    Block,
    Character,
    Fifo,
}

/// # Errors
/// Returns an error if the device type is invalid.
fn parse_type(tpe: &str) -> Result<FileType, String> {
    // Only check the first character, to allow mnemonic usage like
    // 'mknod /dev/rst0 character 18 0'.
    tpe.chars()
        .next()
        .ok_or_else(|| "missing device type".to_string())
        .and_then(|first_char| match first_char {
            'b' => Ok(FileType::Block),
            'c' | 'u' => Ok(FileType::Character),
            'p' => Ok(FileType::Fifo),
            _ => Err(format!("invalid device type {}", tpe.quote())),
        })
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help("set file permission bits to MODE, not a=rw - umask"),
        )
        .arg(
            Arg::new("name")
                .value_name("NAME")
                .help("name of the new file")
                .required(true)
                .value_hint(clap::ValueHint::AnyPath),
        )
        .arg(
            Arg::new("type")
                .value_name("TYPE")
                .help("type of the new file (b, c, u or p)")
                .required(true)
                .value_parser(parse_type),
        )
        .arg(
            Arg::new("major")
                .value_name("MAJOR")
                .help("major file type")
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new("minor")
                .value_name("MINOR")
                .help("minor file type")
                .value_parser(value_parser!(u64)),
        )
}
