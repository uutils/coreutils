use clap::{crate_version, App, Arg};

const ABOUT: &str = "Copy SOURCE to DEST or multiple SOURCE(s) to the existing
 DIRECTORY, while setting permission modes and owner/group";

pub const OPT_COMPARE: &str = "compare";
pub const OPT_BACKUP: &str = "backup";
pub const OPT_BACKUP_2: &str = "backup2";
pub const OPT_DIRECTORY: &str = "directory";
pub const OPT_IGNORED: &str = "ignored";
pub const OPT_CREATE_LEADING: &str = "create-leading";
pub const OPT_GROUP: &str = "group";
pub const OPT_MODE: &str = "mode";
pub const OPT_OWNER: &str = "owner";
pub const OPT_PRESERVE_TIMESTAMPS: &str = "preserve-timestamps";
pub const OPT_STRIP: &str = "strip";
pub const OPT_STRIP_PROGRAM: &str = "strip-program";
pub const OPT_SUFFIX: &str = "suffix";
pub const OPT_TARGET_DIRECTORY: &str = "target-directory";
pub const OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
pub const OPT_VERBOSE: &str = "verbose";
pub const OPT_PRESERVE_CONTEXT: &str = "preserve-context";
pub const OPT_CONTEXT: &str = "context";

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
    .version(crate_version!())
    .about(ABOUT)
    .arg(
            Arg::with_name(OPT_BACKUP)
            .long(OPT_BACKUP)
            .help("(unimplemented) make a backup of each existing destination file")
            .value_name("CONTROL")
    )
    .arg(
        // TODO implement flag
        Arg::with_name(OPT_BACKUP_2)
        .short("b")
        .help("(unimplemented) like --backup but does not accept an argument")
    )
    .arg(
        Arg::with_name(OPT_IGNORED)
        .short("c")
        .help("ignored")
    )
    .arg(
        Arg::with_name(OPT_COMPARE)
        .short("C")
        .long(OPT_COMPARE)
        .help("compare each pair of source and destination files, and in some cases, do not modify the destination at all")
    )
    .arg(
        Arg::with_name(OPT_DIRECTORY)
            .short("d")
            .long(OPT_DIRECTORY)
            .help("treat all arguments as directory names. create all components of the specified directories")
    )

    .arg(
        // TODO implement flag
        Arg::with_name(OPT_CREATE_LEADING)
            .short("D")
            .help("create all leading components of DEST except the last, then copy SOURCE to DEST")
    )
    .arg(
        Arg::with_name(OPT_GROUP)
            .short("g")
            .long(OPT_GROUP)
            .help("set group ownership, instead of process's current group")
            .value_name("GROUP")
            .takes_value(true)
    )
    .arg(
        Arg::with_name(OPT_MODE)
            .short("m")
            .long(OPT_MODE)
            .help("set permission mode (as in chmod), instead of rwxr-xr-x")
            .value_name("MODE")
            .takes_value(true)
    )
    .arg(
        Arg::with_name(OPT_OWNER)
            .short("o")
            .long(OPT_OWNER)
            .help("set ownership (super-user only)")
            .value_name("OWNER")
            .takes_value(true)
    )
    .arg(
        Arg::with_name(OPT_PRESERVE_TIMESTAMPS)
            .short("p")
            .long(OPT_PRESERVE_TIMESTAMPS)
            .help("apply access/modification times of SOURCE files to corresponding destination files")
    )
    .arg(
        Arg::with_name(OPT_STRIP)
        .short("s")
        .long(OPT_STRIP)
        .help("strip symbol tables (no action Windows)")
    )
    .arg(
        Arg::with_name(OPT_STRIP_PROGRAM)
            .long(OPT_STRIP_PROGRAM)
            .help("program used to strip binaries (no action Windows)")
            .value_name("PROGRAM")
    )
    .arg(
        // TODO implement flag
        Arg::with_name(OPT_SUFFIX)
            .short("S")
            .long(OPT_SUFFIX)
            .help("(unimplemented) override the usual backup suffix")
            .value_name("SUFFIX")
            .takes_value(true)
            .min_values(1)
    )
    .arg(
        // TODO implement flag
        Arg::with_name(OPT_TARGET_DIRECTORY)
            .short("t")
            .long(OPT_TARGET_DIRECTORY)
            .help("(unimplemented) move all SOURCE arguments into DIRECTORY")
            .value_name("DIRECTORY")
    )
    .arg(
        // TODO implement flag
        Arg::with_name(OPT_NO_TARGET_DIRECTORY)
            .short("T")
            .long(OPT_NO_TARGET_DIRECTORY)
            .help("(unimplemented) treat DEST as a normal file")

    )
    .arg(
        Arg::with_name(OPT_VERBOSE)
        .short("v")
        .long(OPT_VERBOSE)
        .help("explain what is being done")
    )
    .arg(
        // TODO implement flag
        Arg::with_name(OPT_PRESERVE_CONTEXT)
            .short("P")
            .long(OPT_PRESERVE_CONTEXT)
            .help("(unimplemented) preserve security context")
    )
    .arg(
        // TODO implement flag
        Arg::with_name(OPT_CONTEXT)
            .short("Z")
            .long(OPT_CONTEXT)
            .help("(unimplemented) set security context of files and directories")
            .value_name("CONTEXT")
    )
    .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true).min_values(1))
}
