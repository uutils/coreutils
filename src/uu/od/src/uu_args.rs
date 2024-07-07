// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("od.md");
const USAGE: &str = help_usage!("od.md");
const AFTER_HELP: &str = help_section!("after help", "od.md");

pub(crate) mod options {
    pub const HELP: &str = "help";
    pub const ADDRESS_RADIX: &str = "address-radix";
    pub const SKIP_BYTES: &str = "skip-bytes";
    pub const READ_BYTES: &str = "read-bytes";
    pub const ENDIAN: &str = "endian";
    pub const STRINGS: &str = "strings";
    pub const FORMAT: &str = "format";
    pub const OUTPUT_DUPLICATES: &str = "output-duplicates";
    pub const TRADITIONAL: &str = "traditional";
    pub const WIDTH: &str = "width";
    pub const FILENAME: &str = "FILENAME";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .trailing_var_arg(true)
        .dont_delimit_trailing_values(true)
        .infer_long_args(true)
        .args_override_self(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help)
        )
        .arg(
            Arg::new(options::ADDRESS_RADIX)
                .short('A')
                .long(options::ADDRESS_RADIX)
                .help("Select the base in which file offsets are printed.")
                .value_name("RADIX"),
        )
        .arg(
            Arg::new(options::SKIP_BYTES)
                .short('j')
                .long(options::SKIP_BYTES)
                .help("Skip bytes input bytes before formatting and writing.")
                .value_name("BYTES"),
        )
        .arg(
            Arg::new(options::READ_BYTES)
                .short('N')
                .long(options::READ_BYTES)
                .help("limit dump to BYTES input bytes")
                .value_name("BYTES"),
        )
        .arg(
            Arg::new(options::ENDIAN)
                .long(options::ENDIAN)
                .help("byte order to use for multi-byte formats")
                .value_parser(ShortcutValueParser::new(["big", "little"]))
                .value_name("big|little"),
        )
        .arg(
            Arg::new(options::STRINGS)
                .short('S')
                .long(options::STRINGS)
                .help(
                    "NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when \
                     BYTES is not specified.",
                )
                .default_missing_value("3")
                .value_name("BYTES"),
        )
        .arg(
            Arg::new("a")
                .short('a')
                .help("named characters, ignoring high-order bit")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("b")
                .short('b')
                .help("octal bytes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("c")
                .short('c')
                .help("ASCII characters or backslash escapes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("d")
                .short('d')
                .help("unsigned decimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("D")
                .short('D')
                .help("unsigned decimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("o")
                .short('o')
                .help("octal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("I")
                .short('I')
                .help("decimal 8-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("L")
                .short('L')
                .help("decimal 8-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("i")
                .short('i')
                .help("decimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("l")
                .short('l')
                .help("decimal 8-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("x")
                .short('x')
                .help("hexadecimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("h")
                .short('h')
                .help("hexadecimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("O")
                .short('O')
                .help("octal 4-byte units")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("s")
                .short('s')
                .help("decimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("X")
                .short('X')
                .help("hexadecimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("H")
                .short('H')
                .help("hexadecimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("e")
                .short('e')
                .help("floating point double precision (64-bit) units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("f")
                .short('f')
                .help("floating point double precision (32-bit) units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("F")
                .short('F')
                .help("floating point double precision (64-bit) units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORMAT)
                .short('t')
                .long("format")
                .help("select output format or formats")
                .action(ArgAction::Append)
                .num_args(1)
                .value_name("TYPE"),
        )
        .arg(
            Arg::new(options::OUTPUT_DUPLICATES)
                .short('v')
                .long(options::OUTPUT_DUPLICATES)
                .help("do not use * to mark line suppression")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .help(
                    "output BYTES bytes per output line. 32 is implied when BYTES is not \
                     specified.",
                )
                .default_missing_value("32")
                .value_name("BYTES")
                .num_args(..=1),
        )
        .arg(
            Arg::new(options::TRADITIONAL)
                .long(options::TRADITIONAL)
                .help("compatibility mode with one input, offset and label.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILENAME)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}
