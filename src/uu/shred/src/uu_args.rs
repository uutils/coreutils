// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage};

pub const ABOUT: &str = help_about!("shred.md");
pub const USAGE: &str = help_usage!("shred.md");
pub const AFTER_HELP: &str = help_section!("after help", "shred.md");

pub mod options {
    pub const FORCE: &str = "force";
    pub const FILE: &str = "file";
    pub const ITERATIONS: &str = "iterations";
    pub const SIZE: &str = "size";
    pub const WIPESYNC: &str = "u";
    pub const REMOVE: &str = "remove";
    pub const VERBOSE: &str = "verbose";
    pub const EXACT: &str = "exact";
    pub const ZERO: &str = "zero";

    pub mod remove {
        pub const UNLINK: &str = "unlink";
        pub const WIPE: &str = "wipe";
        pub const WIPESYNC: &str = "wipesync";
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FORCE)
                .long(options::FORCE)
                .short('f')
                .help("change permissions to allow writing if necessary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ITERATIONS)
                .long(options::ITERATIONS)
                .short('n')
                .help("overwrite N times instead of the default (3)")
                .value_name("NUMBER")
                .default_value("3"),
        )
        .arg(
            Arg::new(options::SIZE)
                .long(options::SIZE)
                .short('s')
                .value_name("N")
                .help("shred this many bytes (suffixes like K, M, G accepted)"),
        )
        .arg(
            Arg::new(options::WIPESYNC)
                .short('u')
                .help("deallocate and remove file after overwriting")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REMOVE)
                .long(options::REMOVE)
                .value_name("HOW")
                .value_parser(ShortcutValueParser::new([
                    options::remove::UNLINK,
                    options::remove::WIPE,
                    options::remove::WIPESYNC,
                ]))
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value(options::remove::WIPESYNC)
                .help("like -u but give control on HOW to delete;  See below")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long(options::VERBOSE)
                .short('v')
                .help("show progress")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::EXACT)
                .long(options::EXACT)
                .short('x')
                .help(
                    "do not round file sizes up to the next full block;\n\
                     this is the default for non-regular files",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help("add a final overwrite with zeros to hide shredding")
                .action(ArgAction::SetTrue),
        )
        // Positional arguments
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}
