// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, value_parser, Arg, ArgAction, Command};

use uucore::checksum::SUPPORTED_ALGORITHMS;
use uucore::{format_usage, help_about, help_section, help_usage};

const USAGE: &str = help_usage!("cksum.md");
const ABOUT: &str = help_about!("cksum.md");
const AFTER_HELP: &str = help_section!("after help", "cksum.md");

pub mod options {
    pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    pub const RAW: &str = "raw";
    pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    pub const STRICT: &str = "strict";
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
    pub const STATUS: &str = "status";
    pub const WARN: &str = "warn";
    pub const IGNORE_MISSING: &str = "ignore-missing";
    pub const QUIET: &str = "quiet";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ALGORITHM)
                .long(options::ALGORITHM)
                .short('a')
                .help("select the digest type to use. See DIGEST below")
                .value_name("ALGORITHM")
                .value_parser(SUPPORTED_ALGORITHMS),
        )
        .arg(
            Arg::new(options::UNTAGGED)
                .long(options::UNTAGGED)
                .help("create a reversed style checksum, without digest type")
                .action(ArgAction::SetTrue)
                .overrides_with(options::TAG),
        )
        .arg(
            Arg::new(options::TAG)
                .long(options::TAG)
                .help("create a BSD style checksum, undo --untagged (default)")
                .action(ArgAction::SetTrue)
                .overrides_with(options::UNTAGGED),
        )
        .arg(
            Arg::new(options::LENGTH)
                .long(options::LENGTH)
                .value_parser(value_parser!(usize))
                .short('l')
                .help(
                    "digest length in bits; must not exceed the max for the blake2 algorithm \
                    and must be a multiple of 8",
                )
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::RAW)
                .long(options::RAW)
                .help("emit a raw binary digest, not hexadecimal")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRICT)
                .long(options::STRICT)
                .help("exit non-zero for improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long(options::CHECK)
                .help("read hashsums from the FILEs and check them")
                .action(ArgAction::SetTrue)
                .conflicts_with("tag"),
        )
        .arg(
            Arg::new(options::BASE64)
                .long(options::BASE64)
                .help("emit a base64 digest, not hexadecimal")
                .action(ArgAction::SetTrue)
                // Even though this could easily just override an earlier '--raw',
                // GNU cksum does not permit these flags to be combined:
                .conflicts_with(options::RAW),
        )
        .arg(
            Arg::new(options::TEXT)
                .long(options::TEXT)
                .short('t')
                .hide(true)
                .overrides_with(options::BINARY)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BINARY)
                .long(options::BINARY)
                .short('b')
                .hide(true)
                .overrides_with(options::TEXT)
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
            Arg::new(options::STATUS)
                .long("status")
                .help("don't output anything, status code shows success")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .help("don't print OK for each successfully verified file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_MISSING)
                .long(options::IGNORE_MISSING)
                .help("don't fail or report status for missing files")
                .action(ArgAction::SetTrue),
        )
        .after_help(AFTER_HELP)
}
