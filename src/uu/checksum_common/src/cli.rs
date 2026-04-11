// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use uucore::{checksum::SUPPORTED_ALGORITHMS, translate};

/// List of all options that can be encountered in checksum utils
pub mod options {
    // cksum-specific
    pub const ALGORITHM: &str = "algorithm";
    pub const DEBUG: &str = "debug";

    // positional arg
    pub const FILE: &str = "file";

    pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    pub const RAW: &str = "raw";
    pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
    pub const ZERO: &str = "zero";

    // check-specific
    pub const STRICT: &str = "strict";
    pub const STATUS: &str = "status";
    pub const WARN: &str = "warn";
    pub const IGNORE_MISSING: &str = "ignore-missing";
    pub const QUIET: &str = "quiet";
}

/// `ChecksumCommand` is a convenience trait to more easily declare checksum
/// CLI interfaces with
pub trait ChecksumCommand {
    fn with_algo(self) -> Self;

    fn with_length(self) -> Self;

    fn with_check_and_opts(self) -> Self;

    fn with_binary(self) -> Self;

    fn with_text(self, is_default: bool) -> Self;

    fn with_tag(self, is_default: bool) -> Self;

    fn with_untagged(self) -> Self;

    fn with_raw(self) -> Self;

    fn with_base64(self) -> Self;

    fn with_zero(self) -> Self;

    fn with_debug(self) -> Self;
}

impl ChecksumCommand for Command {
    fn with_algo(self) -> Self {
        self.arg(
            Arg::new(options::ALGORITHM)
                .long(options::ALGORITHM)
                .short('a')
                .help(translate!("ck-common-help-algorithm"))
                .value_name("ALGORITHM")
                .value_parser(SUPPORTED_ALGORITHMS),
        )
    }

    fn with_length(self) -> Self {
        self.arg(
            Arg::new(options::LENGTH)
                .long(options::LENGTH)
                .short('l')
                .help(translate!("ck-common-help-length"))
                .action(ArgAction::Set),
        )
    }

    fn with_check_and_opts(self) -> Self {
        self.arg(
            Arg::new(options::CHECK)
                .short('c')
                .long(options::CHECK)
                .help(translate!("ck-common-help-check"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WARN)
                .short('w')
                .long("warn")
                .help(translate!("ck-common-help-warn"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::STATUS, options::QUIET]),
        )
        .arg(
            Arg::new(options::STATUS)
                .long("status")
                .help(translate!("ck-common-help-status"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::WARN, options::QUIET]),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .help(translate!("ck-common-help-quiet"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::STATUS, options::WARN]),
        )
        .arg(
            Arg::new(options::IGNORE_MISSING)
                .long(options::IGNORE_MISSING)
                .help(translate!("ck-common-help-ignore-missing"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRICT)
                .long(options::STRICT)
                .help(translate!("ck-common-help-strict"))
                .action(ArgAction::SetTrue),
        )
    }

    fn with_binary(self) -> Self {
        self.arg(
            Arg::new(options::BINARY)
                .long(options::BINARY)
                .short('b')
                .hide(true)
                .overrides_with(options::TEXT)
                .action(ArgAction::SetTrue),
        )
    }

    fn with_text(self, is_default: bool) -> Self {
        let mut arg = Arg::new(options::TEXT)
            .long(options::TEXT)
            .short('t')
            .action(ArgAction::SetTrue);

        arg = if is_default {
            arg.help(translate!("ck-common-help-text"))
        } else {
            arg.hide(true)
        };

        self.arg(arg)
    }

    fn with_tag(self, default: bool) -> Self {
        let mut arg = Arg::new(options::TAG)
            .long(options::TAG)
            .action(ArgAction::SetTrue);

        arg = if default {
            arg.help(translate!("ck-common-help-tag-default"))
        } else {
            arg.help(translate!("ck-common-help-tag"))
        };

        self.arg(arg)
    }

    fn with_untagged(self) -> Self {
        self.arg(
            Arg::new(options::UNTAGGED)
                .long(options::UNTAGGED)
                .help(translate!("ck-common-help-untagged"))
                .overrides_with(options::TAG)
                .action(ArgAction::SetTrue),
        )
    }

    fn with_raw(self) -> Self {
        self.arg(
            Arg::new(options::RAW)
                .long(options::RAW)
                .help(translate!("ck-common-help-raw"))
                .action(ArgAction::SetTrue),
        )
    }

    fn with_base64(self) -> Self {
        self.arg(
            Arg::new(options::BASE64)
                .long(options::BASE64)
                .help(translate!("ck-common-help-base64"))
                .action(ArgAction::SetTrue)
                // Even though this could easily just override an earlier '--raw',
                // GNU cksum does not permit these flags to be combined:
                .conflicts_with(options::RAW),
        )
    }

    fn with_zero(self) -> Self {
        self.arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help(translate!("ck-common-help-zero"))
                .action(ArgAction::SetTrue),
        )
    }

    fn with_debug(self) -> Self {
        self.arg(
            Arg::new(options::DEBUG)
                .long(options::DEBUG)
                .help(translate!("ck-common-help-debug"))
                .action(ArgAction::SetTrue),
        )
    }
}
