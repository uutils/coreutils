use clap::{crate_version, App, Arg};
use uucore::backup_control;

pub const OPT_ARCHIVE: &str = "archive";
pub const OPT_ATTRIBUTES_ONLY: &str = "attributes-only";
pub const OPT_BACKUP: &str = "backup";
pub const OPT_BACKUP_NO_ARG: &str = "b";
pub const OPT_CLI_SYMBOLIC_LINKS: &str = "cli-symbolic-links";
pub const OPT_CONTEXT: &str = "context";
pub const OPT_COPY_CONTENTS: &str = "copy-contents";
pub const OPT_DEREFERENCE: &str = "dereference";
pub const OPT_FORCE: &str = "force";
pub const OPT_INTERACTIVE: &str = "interactive";
pub const OPT_LINK: &str = "link";
pub const OPT_NO_CLOBBER: &str = "no-clobber";
pub const OPT_NO_DEREFERENCE: &str = "no-dereference";
pub const OPT_NO_DEREFERENCE_PRESERVE_LINKS: &str = "no-dereference-preserve-links";
pub const OPT_NO_PRESERVE: &str = "no-preserve";
pub const OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
pub const OPT_ONE_FILE_SYSTEM: &str = "one-file-system";
pub const OPT_PARENT: &str = "parent";
pub const OPT_PARENTS: &str = "parents";
pub const OPT_PATHS: &str = "paths";
pub const OPT_PRESERVE: &str = "preserve";
pub const OPT_PRESERVE_DEFAULT_ATTRIBUTES: &str = "preserve-default-attributes";
pub const OPT_RECURSIVE: &str = "recursive";
pub const OPT_RECURSIVE_ALIAS: &str = "recursive_alias";
pub const OPT_REFLINK: &str = "reflink";
pub const OPT_REMOVE_DESTINATION: &str = "remove-destination";
pub const OPT_SPARSE: &str = "sparse";
pub const OPT_STRIP_TRAILING_SLASHES: &str = "strip-trailing-slashes";
pub const OPT_SUFFIX: &str = "suffix";
pub const OPT_SYMBOLIC_LINK: &str = "symbolic-link";
pub const OPT_TARGET_DIRECTORY: &str = "target-directory";
pub const OPT_UPDATE: &str = "update";
pub const OPT_VERBOSE: &str = "verbose";

#[derive(Clone, Eq, PartialEq)]
pub enum Attribute {
    #[cfg(unix)]
    Mode,
    Ownership,
    Timestamps,
    Context,
    Links,
    Xattr,
}

#[cfg(unix)]
pub const PRESERVABLE_ATTRIBUTES: &[&str] = &[
    "mode",
    "ownership",
    "timestamps",
    "context",
    "links",
    "xattr",
    "all",
];

#[cfg(not(unix))]
pub const PRESERVABLE_ATTRIBUTES: &[&str] = &[
    "ownership",
    "timestamps",
    "context",
    "links",
    "xattr",
    "all",
];

const ABOUT: &str = "Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
    .version(crate_version!())
    .about(ABOUT)
    .arg(Arg::with_name(OPT_TARGET_DIRECTORY)
         .short("t")
         .conflicts_with(OPT_NO_TARGET_DIRECTORY)
         .long(OPT_TARGET_DIRECTORY)
         .value_name(OPT_TARGET_DIRECTORY)
         .takes_value(true)
         .help("copy all SOURCE arguments into target-directory"))
    .arg(Arg::with_name(OPT_NO_TARGET_DIRECTORY)
         .short("T")
         .long(OPT_NO_TARGET_DIRECTORY)
         .conflicts_with(OPT_TARGET_DIRECTORY)
         .help("Treat DEST as a regular file and not a directory"))
    .arg(Arg::with_name(OPT_INTERACTIVE)
         .short("i")
         .long(OPT_INTERACTIVE)
         .conflicts_with(OPT_NO_CLOBBER)
         .help("ask before overwriting files"))
    .arg(Arg::with_name(OPT_LINK)
         .short("l")
         .long(OPT_LINK)
         .overrides_with(OPT_REFLINK)
         .help("hard-link files instead of copying"))
    .arg(Arg::with_name(OPT_NO_CLOBBER)
         .short("n")
         .long(OPT_NO_CLOBBER)
         .conflicts_with(OPT_INTERACTIVE)
         .help("don't overwrite a file that already exists"))
    .arg(Arg::with_name(OPT_RECURSIVE)
         .short("r")
         .long(OPT_RECURSIVE)
         // --archive sets this option
        .help("copy directories recursively"))
    .arg(Arg::with_name(OPT_RECURSIVE_ALIAS)
         .short("R")
         .help("same as -r"))
    .arg(Arg::with_name(OPT_STRIP_TRAILING_SLASHES)
         .long(OPT_STRIP_TRAILING_SLASHES)
         .help("remove any trailing slashes from each SOURCE argument"))
    .arg(Arg::with_name(OPT_VERBOSE)
         .short("v")
         .long(OPT_VERBOSE)
         .help("explicitly state what is being done"))
    .arg(Arg::with_name(OPT_SYMBOLIC_LINK)
         .short("s")
         .long(OPT_SYMBOLIC_LINK)
         .conflicts_with(OPT_LINK)
         .overrides_with(OPT_REFLINK)
         .help("make symbolic links instead of copying"))
    .arg(Arg::with_name(OPT_FORCE)
         .short("f")
         .long(OPT_FORCE)
         .help("if an existing destination file cannot be opened, remove it and \
                try again (this option is ignored when the -n option is also used). \
                Currently not implemented for Windows."))
    .arg(Arg::with_name(OPT_REMOVE_DESTINATION)
         .long(OPT_REMOVE_DESTINATION)
         .conflicts_with(OPT_FORCE)
         .help("remove each existing destination file before attempting to open it \
                (contrast with --force). On Windows, current only works for writeable files."))
    .arg(Arg::with_name(OPT_BACKUP)
         .long(OPT_BACKUP)
         .help("make a backup of each existing destination file")
         .takes_value(true)
         .require_equals(true)
         .min_values(0)
         .possible_values(backup_control::BACKUP_CONTROL_VALUES)
         .value_name("CONTROL")
    )
    .arg(Arg::with_name(OPT_BACKUP_NO_ARG)
         .short(OPT_BACKUP_NO_ARG)
         .help("like --backup but does not accept an argument")
    )
    .arg(Arg::with_name(OPT_SUFFIX)
         .short("S")
         .long(OPT_SUFFIX)
         .takes_value(true)
         .value_name("SUFFIX")
         .help("override the usual backup suffix"))
    .arg(Arg::with_name(OPT_UPDATE)
         .short("u")
         .long(OPT_UPDATE)
         .help("copy only when the SOURCE file is newer than the destination file\
                or when the destination file is missing"))
    .arg(Arg::with_name(OPT_REFLINK)
         .long(OPT_REFLINK)
         .takes_value(true)
         .value_name("WHEN")
         .help("control clone/CoW copies. See below"))
    .arg(Arg::with_name(OPT_ATTRIBUTES_ONLY)
         .long(OPT_ATTRIBUTES_ONLY)
         .conflicts_with(OPT_COPY_CONTENTS)
         .overrides_with(OPT_REFLINK)
         .help("Don't copy the file data, just the attributes"))
    .arg(Arg::with_name(OPT_PRESERVE)
         .long(OPT_PRESERVE)
         .takes_value(true)
         .multiple(true)
         .use_delimiter(true)
         .possible_values(PRESERVABLE_ATTRIBUTES)
         .value_name("ATTR_LIST")
         .conflicts_with_all(&[OPT_PRESERVE_DEFAULT_ATTRIBUTES, OPT_NO_PRESERVE])
         // -d sets this option
         // --archive sets this option
         .help("Preserve the specified attributes (default: mode(unix only),ownership,timestamps),\
                if possible additional attributes: context, links, xattr, all"))
    .arg(Arg::with_name(OPT_PRESERVE_DEFAULT_ATTRIBUTES)
         .short("-p")
         .long(OPT_PRESERVE_DEFAULT_ATTRIBUTES)
         .conflicts_with_all(&[OPT_PRESERVE, OPT_NO_PRESERVE, OPT_ARCHIVE])
         .help("same as --preserve=mode(unix only),ownership,timestamps"))
    .arg(Arg::with_name(OPT_NO_PRESERVE)
         .long(OPT_NO_PRESERVE)
         .takes_value(true)
         .value_name("ATTR_LIST")
         .conflicts_with_all(&[OPT_PRESERVE_DEFAULT_ATTRIBUTES, OPT_PRESERVE, OPT_ARCHIVE])
         .help("don't preserve the specified attributes"))
    .arg(Arg::with_name(OPT_PARENTS)
        .long(OPT_PARENTS)
        .alias(OPT_PARENT)
        .help("use full source file name under DIRECTORY"))
    .arg(Arg::with_name(OPT_NO_DEREFERENCE)
         .short("-P")
         .long(OPT_NO_DEREFERENCE)
         .conflicts_with(OPT_DEREFERENCE)
         // -d sets this option
         .help("never follow symbolic links in SOURCE"))
    .arg(Arg::with_name(OPT_DEREFERENCE)
         .short("L")
         .long(OPT_DEREFERENCE)
         .conflicts_with(OPT_NO_DEREFERENCE)
         .help("always follow symbolic links in SOURCE"))
    .arg(Arg::with_name(OPT_ARCHIVE)
         .short("a")
         .long(OPT_ARCHIVE)
         .conflicts_with_all(&[OPT_PRESERVE_DEFAULT_ATTRIBUTES, OPT_PRESERVE, OPT_NO_PRESERVE])
         .help("Same as -dR --preserve=all"))
    .arg(Arg::with_name(OPT_NO_DEREFERENCE_PRESERVE_LINKS)
         .short("d")
         .help("same as --no-dereference --preserve=links"))
    .arg(Arg::with_name(OPT_ONE_FILE_SYSTEM)
         .short("x")
         .long(OPT_ONE_FILE_SYSTEM)
         .help("stay on this file system"))

    // TODO: implement the following args
    .arg(Arg::with_name(OPT_COPY_CONTENTS)
         .long(OPT_COPY_CONTENTS)
         .conflicts_with(OPT_ATTRIBUTES_ONLY)
         .help("NotImplemented: copy contents of special files when recursive"))
    .arg(Arg::with_name(OPT_SPARSE)
         .long(OPT_SPARSE)
         .takes_value(true)
         .value_name("WHEN")
         .help("NotImplemented: control creation of sparse files. See below"))
    .arg(Arg::with_name(OPT_CONTEXT)
         .long(OPT_CONTEXT)
         .takes_value(true)
         .value_name("CTX")
         .help("NotImplemented: set SELinux security context of destination file to default type"))
    .arg(Arg::with_name(OPT_CLI_SYMBOLIC_LINKS)
         .short("H")
         .help("NotImplemented: follow command-line symbolic links in SOURCE"))
    // END TODO

    .arg(Arg::with_name(OPT_PATHS)
         .multiple(true))
}
