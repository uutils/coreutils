// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::num::ParseIntError;

use clap::{builder::ValueParser, crate_version, value_parser, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("hashsum.md");
const USAGE: &str = help_usage!("hashsum.md");

pub mod options {
    //pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    //pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    //pub const RAW: &str = "raw";
    //pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    pub const STRICT: &str = "strict";
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
    pub const STATUS: &str = "status";
    pub const WARN: &str = "warn";
    pub const QUIET: &str = "quiet";
}

// TODO: return custom error type
fn parse_bit_num(arg: &str) -> Result<usize, ParseIntError> {
    arg.parse()
}

pub fn uu_app_common() -> Command {
    #[cfg(windows)]
    const BINARY_HELP: &str = "read in binary mode (default)";
    #[cfg(not(windows))]
    const BINARY_HELP: &str = "read in binary mode";
    #[cfg(windows)]
    const TEXT_HELP: &str = "read in text mode";
    #[cfg(not(windows))]
    const TEXT_HELP: &str = "read in text mode (default)";
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::BINARY)
                .short('b')
                .long("binary")
                .help(BINARY_HELP)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long("check")
                .help("read hashsums from the FILEs and check them")
                .action(ArgAction::SetTrue)
                .conflicts_with("tag"),
        )
        .arg(
            Arg::new(options::TAG)
                .long("tag")
                .help("create a BSD-style checksum")
                .action(ArgAction::SetTrue)
                .conflicts_with("text"),
        )
        .arg(
            Arg::new(options::TEXT)
                .short('t')
                .long("text")
                .help(TEXT_HELP)
                .conflicts_with("binary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('q')
                .long(options::QUIET)
                .help("don't print OK for each successfully verified file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STATUS)
                .short('s')
                .long("status")
                .help("don't output anything, status code shows success")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRICT)
                .long("strict")
                .help("exit non-zero for improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("ignore-missing")
                .long("ignore-missing")
                .help("don't fail or report status for missing files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WARN)
                .short('w')
                .long("warn")
                .help("warn about improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("zero")
                .short('z')
                .long("zero")
                .help("end each output line with NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .index(1)
                .action(ArgAction::Append)
                .value_name(options::FILE)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string()),
        )
}

pub fn uu_app_length() -> Command {
    uu_app_opt_length(uu_app_common())
}

fn uu_app_opt_length(command: Command) -> Command {
    command.arg(
        Arg::new(options::LENGTH)
            .long(options::LENGTH)
            .value_parser(value_parser!(usize))
            .short('l')
            .help(
                "digest length in bits; must not exceed the max for the blake2 algorithm \
                    and must be a multiple of 8",
            )
            .overrides_with(options::LENGTH)
            .action(ArgAction::Set),
    )
}

pub fn uu_app_b3sum() -> Command {
    uu_app_b3sum_opts(uu_app_common())
}

fn uu_app_b3sum_opts(command: Command) -> Command {
    command.arg(
        Arg::new("no-names")
            .long("no-names")
            .help("Omits filenames in the output (option not present in GNU/Coreutils)")
            .action(ArgAction::SetTrue),
    )
}

pub fn uu_app_bits() -> Command {
    uu_app_opt_bits(uu_app_common())
}

fn uu_app_opt_bits(command: Command) -> Command {
    // Needed for variable-length output sums (e.g. SHAKE)
    command.arg(
        Arg::new("bits")
            .long("bits")
            .help("set the size of the output (only for SHAKE)")
            .value_name("BITS")
            // XXX: should we actually use validators?  they're not particularly efficient
            .value_parser(parse_bit_num),
    )
}

pub fn uu_app_custom() -> Command {
    let mut command = uu_app_b3sum_opts(uu_app_opt_bits(uu_app_common()));
    let algorithms = &[
        ("md5", "work with MD5"),
        ("sha1", "work with SHA1"),
        ("sha224", "work with SHA224"),
        ("sha256", "work with SHA256"),
        ("sha384", "work with SHA384"),
        ("sha512", "work with SHA512"),
        ("sha3", "work with SHA3"),
        ("sha3-224", "work with SHA3-224"),
        ("sha3-256", "work with SHA3-256"),
        ("sha3-384", "work with SHA3-384"),
        ("sha3-512", "work with SHA3-512"),
        (
            "shake128",
            "work with SHAKE128 using BITS for the output size",
        ),
        (
            "shake256",
            "work with SHAKE256 using BITS for the output size",
        ),
        ("b2sum", "work with BLAKE2"),
        ("b3sum", "work with BLAKE3"),
    ];

    for (name, desc) in algorithms {
        command = command.arg(
            Arg::new(*name)
                .long(name)
                .help(*desc)
                .action(ArgAction::SetTrue),
        );
    }
    command
}

// hashsum is handled differently in build.rs, therefore this is not the same
// as in other utilities.
// Second return value is is_hashsum_bin
#[allow(dead_code)]
pub fn uu_app(binary_name: &str) -> (Command, bool) {
    match binary_name {
        // These all support the same options.
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum" => {
            (uu_app_common(), false)
        }
        // b2sum supports the md5sum options plus -l/--length.
        "b2sum" => (uu_app_length(), false),
        // These have never been part of GNU Coreutils, but can function with the same
        // options as md5sum.
        "sha3-224sum" | "sha3-256sum" | "sha3-384sum" | "sha3-512sum" => (uu_app_common(), false),
        // These have never been part of GNU Coreutils, and require an additional --bits
        // option to specify their output size.
        "sha3sum" | "shake128sum" | "shake256sum" => (uu_app_bits(), false),
        // b3sum has never been part of GNU Coreutils, and has a --no-names option in
        // addition to the b2sum options.
        "b3sum" => (uu_app_b3sum(), false),
        // We're probably just being called as `hashsum`, so give them everything.
        _ => (uu_app_custom(), true),
    }
}
