// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{backup_control, format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("install.md");
const USAGE: &str = help_usage!("install.md");

pub static OPT_COMPARE: &str = "compare";
pub static OPT_DIRECTORY: &str = "directory";
pub static OPT_IGNORED: &str = "ignored";
pub static OPT_CREATE_LEADING: &str = "create-leading";
pub static OPT_GROUP: &str = "group";
pub static OPT_MODE: &str = "mode";
pub static OPT_OWNER: &str = "owner";
pub static OPT_PRESERVE_TIMESTAMPS: &str = "preserve-timestamps";
pub static OPT_STRIP: &str = "strip";
pub static OPT_STRIP_PROGRAM: &str = "strip-program";
pub static OPT_TARGET_DIRECTORY: &str = "target-directory";
pub static OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
pub static OPT_VERBOSE: &str = "verbose";
pub static OPT_PRESERVE_CONTEXT: &str = "preserve-context";
pub static OPT_CONTEXT: &str = "context";

pub static ARG_FILES: &str = "files";

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(backup_control::arguments::backup())
        .arg(backup_control::arguments::backup_no_args())
        .arg(
            Arg::new(OPT_IGNORED)
                .short('c')
                .help("ignored")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_COMPARE)
                .short('C')
                .long(OPT_COMPARE)
                .help(
                    "compare each pair of source and destination files, and in some cases, \
                    do not modify the destination at all",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_DIRECTORY)
                .short('d')
                .long(OPT_DIRECTORY)
                .help(
                    "treat all arguments as directory names. create all components of \
                        the specified directories",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_CREATE_LEADING)
                .short('D')
                .help(
                    "create all leading components of DEST except the last, then copy \
                        SOURCE to DEST",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_GROUP)
                .short('g')
                .long(OPT_GROUP)
                .help("set group ownership, instead of process's current group")
                .value_name("GROUP"),
        )
        .arg(
            Arg::new(OPT_MODE)
                .short('m')
                .long(OPT_MODE)
                .help("set permission mode (as in chmod), instead of rwxr-xr-x")
                .value_name("MODE"),
        )
        .arg(
            Arg::new(OPT_OWNER)
                .short('o')
                .long(OPT_OWNER)
                .help("set ownership (super-user only)")
                .value_name("OWNER")
                .value_hint(clap::ValueHint::Username),
        )
        .arg(
            Arg::new(OPT_PRESERVE_TIMESTAMPS)
                .short('p')
                .long(OPT_PRESERVE_TIMESTAMPS)
                .help(
                    "apply access/modification times of SOURCE files to \
                    corresponding destination files",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_STRIP)
                .short('s')
                .long(OPT_STRIP)
                .help("strip symbol tables (no action Windows)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_STRIP_PROGRAM)
                .long(OPT_STRIP_PROGRAM)
                .help("program used to strip binaries (no action Windows)")
                .value_name("PROGRAM")
                .value_hint(clap::ValueHint::CommandName),
        )
        .arg(backup_control::arguments::suffix())
        .arg(
            // TODO implement flag
            Arg::new(OPT_TARGET_DIRECTORY)
                .short('t')
                .long(OPT_TARGET_DIRECTORY)
                .help("move all SOURCE arguments into DIRECTORY")
                .value_name("DIRECTORY")
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_NO_TARGET_DIRECTORY)
                .short('T')
                .long(OPT_NO_TARGET_DIRECTORY)
                .help("(unimplemented) treat DEST as a normal file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help("explain what is being done")
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_PRESERVE_CONTEXT)
                .short('P')
                .long(OPT_PRESERVE_CONTEXT)
                .help("(unimplemented) preserve security context")
                .action(ArgAction::SetTrue),
        )
        .arg(
            // TODO implement flag
            Arg::new(OPT_CONTEXT)
                .short('Z')
                .long(OPT_CONTEXT)
                .help("(unimplemented) set security context of files and directories")
                .value_name("CONTEXT")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
