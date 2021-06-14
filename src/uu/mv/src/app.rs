use clap::{crate_version, App, Arg};
use uucore::backup_control;

const ABOUT: &str = "Move SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.";

pub const OPT_BACKUP: &str = "backup";
pub const OPT_BACKUP_NO_ARG: &str = "b";
pub const OPT_FORCE: &str = "force";
pub const OPT_INTERACTIVE: &str = "interactive";
pub const OPT_NO_CLOBBER: &str = "no-clobber";
pub const OPT_STRIP_TRAILING_SLASHES: &str = "strip-trailing-slashes";
pub const OPT_SUFFIX: &str = "suffix";
pub const OPT_TARGET_DIRECTORY: &str = "target-directory";
pub const OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
pub const OPT_UPDATE: &str = "update";
pub const OPT_VERBOSE: &str = "verbose";

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
    .version(crate_version!())
    .about(ABOUT)
.arg(
        Arg::with_name(OPT_BACKUP)
        .long(OPT_BACKUP)
        .help("make a backup of each existing destination file")
        .takes_value(true)
        .require_equals(true)
        .min_values(0)
        .possible_values(backup_control::BACKUP_CONTROL_VALUES)
        .value_name("CONTROL")
)
.arg(
        Arg::with_name(OPT_BACKUP_NO_ARG)
        .short(OPT_BACKUP_NO_ARG)
        .help("like --backup but does not accept an argument")
)
.arg(
        Arg::with_name(OPT_FORCE)
        .short("f")
        .long(OPT_FORCE)
        .help("do not prompt before overwriting")
)
.arg(
        Arg::with_name(OPT_INTERACTIVE)
        .short("i")
        .long(OPT_INTERACTIVE)
        .help("prompt before override")
)
.arg(
        Arg::with_name(OPT_NO_CLOBBER).short("n")
        .long(OPT_NO_CLOBBER)
        .help("do not overwrite an existing file")
)
.arg(
        Arg::with_name(OPT_STRIP_TRAILING_SLASHES)
        .long(OPT_STRIP_TRAILING_SLASHES)
        .help("remove any trailing slashes from each SOURCE argument")
)
.arg(
        Arg::with_name(OPT_SUFFIX)
        .short("S")
        .long(OPT_SUFFIX)
        .help("override the usual backup suffix")
        .takes_value(true)
        .value_name("SUFFIX")
)
.arg(
    Arg::with_name(OPT_TARGET_DIRECTORY)
    .short("t")
    .long(OPT_TARGET_DIRECTORY)
    .help("move all SOURCE arguments into DIRECTORY")
    .takes_value(true)
    .value_name("DIRECTORY")
    .conflicts_with(OPT_NO_TARGET_DIRECTORY)
)
.arg(
        Arg::with_name(OPT_NO_TARGET_DIRECTORY)
        .short("T")
        .long(OPT_NO_TARGET_DIRECTORY).
        help("treat DEST as a normal file")
)
.arg(
        Arg::with_name(OPT_UPDATE)
        .short("u")
        .long(OPT_UPDATE)
        .help("move only when the SOURCE file is newer than the destination file or when the destination file is missing")
)
.arg(
        Arg::with_name(OPT_VERBOSE)
        .short("v")
        .long(OPT_VERBOSE).help("explain what is being done")
)
.arg(
    Arg::with_name(ARG_FILES)
        .multiple(true)
        .takes_value(true)
        .min_values(2)
        .required(true)
    )
}
