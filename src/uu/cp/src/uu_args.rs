// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) advcpmv

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};
use uucore::{
    backup_control, format_usage, help_about, help_section, help_usage,
    shortcut_value_parser::ShortcutValueParser, update_control,
};
const ABOUT: &str = help_about!("cp.md");
const USAGE: &str = help_usage!("cp.md");
const AFTER_HELP: &str = help_section!("after help", "cp.md");

const PRESERVE_DEFAULT_VALUES: &str = if cfg!(unix) {
    "mode,ownership,timestamp"
} else {
    "mode,timestamp"
};

#[cfg(unix)]
pub static PRESERVABLE_ATTRIBUTES: &[&str] = &[
    "mode",
    "ownership",
    "timestamps",
    "context",
    "links",
    "xattr",
    "all",
];

#[cfg(not(unix))]
static PRESERVABLE_ATTRIBUTES: &[&str] =
    &["mode", "timestamps", "context", "links", "xattr", "all"];

// Argument constants
pub mod options {
    pub const ARCHIVE: &str = "archive";
    pub const ATTRIBUTES_ONLY: &str = "attributes-only";
    pub const CLI_SYMBOLIC_LINKS: &str = "cli-symbolic-links";
    pub const CONTEXT: &str = "context";
    pub const COPY_CONTENTS: &str = "copy-contents";
    pub const DEREFERENCE: &str = "dereference";
    pub const FORCE: &str = "force";
    pub const INTERACTIVE: &str = "interactive";
    pub const LINK: &str = "link";
    pub const NO_CLOBBER: &str = "no-clobber";
    pub const NO_DEREFERENCE: &str = "no-dereference";
    pub const NO_DEREFERENCE_PRESERVE_LINKS: &str = "no-dereference-preserve-links";
    pub const NO_PRESERVE: &str = "no-preserve";
    pub const NO_TARGET_DIRECTORY: &str = "no-target-directory";
    pub const ONE_FILE_SYSTEM: &str = "one-file-system";
    pub const PARENT: &str = "parent";
    pub const PARENTS: &str = "parents";
    pub const PATHS: &str = "paths";
    pub const PROGRESS_BAR: &str = "progress";
    pub const PRESERVE: &str = "preserve";
    pub const PRESERVE_DEFAULT_ATTRIBUTES: &str = "preserve-default-attributes";
    pub const RECURSIVE: &str = "recursive";
    pub const REFLINK: &str = "reflink";
    pub const REMOVE_DESTINATION: &str = "remove-destination";
    pub const SPARSE: &str = "sparse";
    pub const STRIP_TRAILING_SLASHES: &str = "strip-trailing-slashes";
    pub const SYMBOLIC_LINK: &str = "symbolic-link";
    pub const TARGET_DIRECTORY: &str = "target-directory";
    pub const DEBUG: &str = "debug";
    pub const VERBOSE: &str = "verbose";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    const MODE_ARGS: &[&str] = &[
        options::LINK,
        options::REFLINK,
        options::SYMBOLIC_LINK,
        options::ATTRIBUTES_ONLY,
        options::COPY_CONTENTS,
    ];
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(format!(
            "{AFTER_HELP}\n\n{}",
            backup_control::BACKUP_CONTROL_LONG_HELP
        ))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::TARGET_DIRECTORY)
                .short('t')
                .conflicts_with(options::NO_TARGET_DIRECTORY)
                .long(options::TARGET_DIRECTORY)
                .value_name(options::TARGET_DIRECTORY)
                .value_hint(clap::ValueHint::DirPath)
                .value_parser(ValueParser::path_buf())
                .help("copy all SOURCE arguments into target-directory"),
        )
        .arg(
            Arg::new(options::NO_TARGET_DIRECTORY)
                .short('T')
                .long(options::NO_TARGET_DIRECTORY)
                .conflicts_with(options::TARGET_DIRECTORY)
                .help("Treat DEST as a regular file and not a directory")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INTERACTIVE)
                .short('i')
                .long(options::INTERACTIVE)
                .overrides_with(options::NO_CLOBBER)
                .help("ask before overwriting files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LINK)
                .short('l')
                .long(options::LINK)
                .overrides_with_all(MODE_ARGS)
                .help("hard-link files instead of copying")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CLOBBER)
                .short('n')
                .long(options::NO_CLOBBER)
                .overrides_with(options::INTERACTIVE)
                .help("don't overwrite a file that already exists")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .visible_short_alias('r')
                .long(options::RECURSIVE)
                // --archive sets this option
                .help("copy directories recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRIP_TRAILING_SLASHES)
                .long(options::STRIP_TRAILING_SLASHES)
                .help("remove any trailing slashes from each SOURCE argument")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEBUG)
                .long(options::DEBUG)
                .help("explain how a file is copied. Implies -v")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help("explicitly state what is being done")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SYMBOLIC_LINK)
                .short('s')
                .long(options::SYMBOLIC_LINK)
                .overrides_with_all(MODE_ARGS)
                .help("make symbolic links instead of copying")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORCE)
                .short('f')
                .long(options::FORCE)
                .help(
                    "if an existing destination file cannot be opened, remove it and \
                    try again (this option is ignored when the -n option is also used). \
                    Currently not implemented for Windows.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REMOVE_DESTINATION)
                .long(options::REMOVE_DESTINATION)
                .overrides_with(options::FORCE)
                .help(
                    "remove each existing destination file before attempting to open it \
                    (contrast with --force). On Windows, currently only works for \
                    writeable files.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(backup_control::arguments::backup())
        .arg(backup_control::arguments::backup_no_args())
        .arg(backup_control::arguments::suffix())
        .arg(update_control::arguments::update())
        .arg(update_control::arguments::update_no_args())
        .arg(
            Arg::new(options::REFLINK)
                .long(options::REFLINK)
                .value_name("WHEN")
                .overrides_with_all(MODE_ARGS)
                .require_equals(true)
                .default_missing_value("always")
                .value_parser(ShortcutValueParser::new(["auto", "always", "never"]))
                .num_args(0..=1)
                .help("control clone/CoW copies. See below"),
        )
        .arg(
            Arg::new(options::ATTRIBUTES_ONLY)
                .long(options::ATTRIBUTES_ONLY)
                .overrides_with_all(MODE_ARGS)
                .help("Don't copy the file data, just the attributes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRESERVE)
                .long(options::PRESERVE)
                .action(ArgAction::Append)
                .use_value_delimiter(true)
                .value_parser(ShortcutValueParser::new(PRESERVABLE_ATTRIBUTES))
                .num_args(0..)
                .require_equals(true)
                .value_name("ATTR_LIST")
                .default_missing_value(PRESERVE_DEFAULT_VALUES)
                // -d sets this option
                // --archive sets this option
                .help(
                    "Preserve the specified attributes (default: mode, ownership (unix only), \
                     timestamps), if possible additional attributes: context, links, xattr, all",
                ),
        )
        .arg(
            Arg::new(options::PRESERVE_DEFAULT_ATTRIBUTES)
                .short('p')
                .long(options::PRESERVE_DEFAULT_ATTRIBUTES)
                .help("same as --preserve=mode,ownership(unix only),timestamps")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_PRESERVE)
                .long(options::NO_PRESERVE)
                .action(ArgAction::Append)
                .use_value_delimiter(true)
                .value_parser(ShortcutValueParser::new(PRESERVABLE_ATTRIBUTES))
                .num_args(0..)
                .require_equals(true)
                .value_name("ATTR_LIST")
                .help("don't preserve the specified attributes"),
        )
        .arg(
            Arg::new(options::PARENTS)
                .long(options::PARENTS)
                .alias(options::PARENT)
                .help("use full source file name under DIRECTORY")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE)
                .short('P')
                .long(options::NO_DEREFERENCE)
                .overrides_with(options::DEREFERENCE)
                // -d sets this option
                .help("never follow symbolic links in SOURCE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .overrides_with(options::NO_DEREFERENCE)
                .help("always follow symbolic links in SOURCE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CLI_SYMBOLIC_LINKS)
                .short('H')
                .help("follow command-line symbolic links in SOURCE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARCHIVE)
                .short('a')
                .long(options::ARCHIVE)
                .help("Same as -dR --preserve=all")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE_PRESERVE_LINKS)
                .short('d')
                .help("same as --no-dereference --preserve=links")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONE_FILE_SYSTEM)
                .short('x')
                .long(options::ONE_FILE_SYSTEM)
                .help("stay on this file system")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SPARSE)
                .long(options::SPARSE)
                .value_name("WHEN")
                .value_parser(ShortcutValueParser::new(["never", "auto", "always"]))
                .help("control creation of sparse files. See below"),
        )
        // TODO: implement the following args
        .arg(
            Arg::new(options::COPY_CONTENTS)
                .long(options::COPY_CONTENTS)
                .overrides_with(options::ATTRIBUTES_ONLY)
                .help("NotImplemented: copy contents of special files when recursive")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .help(
                    "NotImplemented: set SELinux security context of destination file to \
                    default type",
                ),
        )
        // END TODO
        .arg(
            // The 'g' short flag is modeled after advcpmv
            // See this repo: https://github.com/jarun/advcpmv
            Arg::new(options::PROGRESS_BAR)
                .long(options::PROGRESS_BAR)
                .short('g')
                .action(clap::ArgAction::SetTrue)
                .help(
                    "Display a progress bar. \n\
                Note: this feature is not supported by GNU coreutils.",
                ),
        )
        .arg(
            Arg::new(options::PATHS)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(ValueParser::path_buf()),
        )
}
