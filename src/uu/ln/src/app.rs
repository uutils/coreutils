use clap::{crate_version, App, Arg};

const ABOUT: &str = "change file owner and group";

const LONG_USAGE: &str = "In the 1st form, create a link to TARGET with the name LINK_NAME.
In the 2nd form, create a link to TARGET in the current directory.
In the 3rd and 4th forms, create links to each TARGET in DIRECTORY.
Create hard links by default, symbolic links with --symbolic.
By default, each destination (name of new link) should not already exist.
When creating hard links, each TARGET must exist.  Symbolic links
can hold arbitrary text; if later resolved, a relative link is
interpreted in relation to its parent directory.
";

pub mod options {
    pub const B: &str = "b";
    pub const BACKUP: &str = "backup";
    pub const FORCE: &str = "force";
    pub const INTERACTIVE: &str = "interactive";
    pub const NO_DEREFERENCE: &str = "no-dereference";
    pub const SYMBOLIC: &str = "symbolic";
    pub const SUFFIX: &str = "suffix";
    pub const TARGET_DIRECTORY: &str = "target-directory";
    pub const NO_TARGET_DIRECTORY: &str = "no-target-directory";
    pub const RELATIVE: &str = "relative";
    pub const VERBOSE: &str = "verbose";
}

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_USAGE)
        .arg(Arg::with_name(options::B).short(options::B).help(
            "make a backup of each file that would otherwise be overwritten or \
             removed",
        ))
        .arg(
            Arg::with_name(options::BACKUP)
                .long(options::BACKUP)
                .help(
                    "make a backup of each file that would otherwise be overwritten \
                     or removed",
                )
                .takes_value(true)
                .possible_values(&[
                    "simple", "never", "numbered", "t", "existing", "nil", "none", "off",
                ])
                .value_name("METHOD"),
        )
        // TODO: opts.arg(
        //    Arg::with_name(("d", "directory", "allow users with appropriate privileges to attempt \
        //                                       to make hard links to directories");
        .arg(
            Arg::with_name(options::FORCE)
                .short("f")
                .long(options::FORCE)
                .help("remove existing destination files"),
        )
        .arg(
            Arg::with_name(options::INTERACTIVE)
                .short("i")
                .long(options::INTERACTIVE)
                .help("prompt whether to remove existing destination files"),
        )
        .arg(
            Arg::with_name(options::NO_DEREFERENCE)
                .short("n")
                .long(options::NO_DEREFERENCE)
                .help(
                    "treat LINK_NAME as a normal file if it is a \
                     symbolic link to a directory",
                ),
        )
        // TODO: opts.arg(
        //    Arg::with_name(("L", "logical", "dereference TARGETs that are symbolic links");
        //
        // TODO: opts.arg(
        //    Arg::with_name(("P", "physical", "make hard links directly to symbolic links");
        .arg(
            Arg::with_name(options::SYMBOLIC)
                .short("s")
                .long("symbolic")
                .help("make symbolic links instead of hard links")
                // override added for https://github.com/uutils/coreutils/issues/2359
                .overrides_with(options::SYMBOLIC),
        )
        .arg(
            Arg::with_name(options::SUFFIX)
                .short("S")
                .long(options::SUFFIX)
                .help("override the usual backup suffix")
                .value_name("SUFFIX")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::TARGET_DIRECTORY)
                .short("t")
                .long(options::TARGET_DIRECTORY)
                .help("specify the DIRECTORY in which to create the links")
                .value_name("DIRECTORY")
                .conflicts_with(options::NO_TARGET_DIRECTORY),
        )
        .arg(
            Arg::with_name(options::NO_TARGET_DIRECTORY)
                .short("T")
                .long(options::NO_TARGET_DIRECTORY)
                .help("treat LINK_NAME as a normal file always"),
        )
        .arg(
            Arg::with_name(options::RELATIVE)
                .short("r")
                .long(options::RELATIVE)
                .help("create symbolic links relative to link location")
                .requires(options::SYMBOLIC),
        )
        .arg(
            Arg::with_name(options::VERBOSE)
                .short("v")
                .long(options::VERBOSE)
                .help("print name of each linked file"),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .required(true)
                .min_values(1),
        )
}
