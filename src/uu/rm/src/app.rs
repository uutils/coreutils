use clap::{crate_version, App, Arg};

const ABOUT: &str = "Remove (unlink) the FILE(s)";

pub const OPT_DIR: &str = "dir";
pub const OPT_INTERACTIVE: &str = "interactive";
pub const OPT_FORCE: &str = "force";
pub const OPT_NO_PRESERVE_ROOT: &str = "no-preserve-root";
pub const OPT_ONE_FILE_SYSTEM: &str = "one-file-system";
pub const OPT_PRESERVE_ROOT: &str = "preserve-root";
pub const OPT_PROMPT: &str = "prompt";
pub const OPT_PROMPT_MORE: &str = "prompt-more";
pub const OPT_RECURSIVE: &str = "recursive";
pub const OPT_RECURSIVE_R: &str = "recursive_R";
pub const OPT_VERBOSE: &str = "verbose";

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)

        .arg(
            Arg::with_name(OPT_FORCE)
            .short("f")
            .long(OPT_FORCE)
            .multiple(true)
            .help("ignore nonexistent files and arguments, never prompt")
        )
        .arg(
            Arg::with_name(OPT_PROMPT)
            .short("i")
            .long("prompt before every removal")
        )
        .arg(
            Arg::with_name(OPT_PROMPT_MORE)
            .short("I")
            .help("prompt once before removing more than three files, or when removing recursively. Less intrusive than -i, while still giving some protection against most mistakes")
        )
        .arg(
            Arg::with_name(OPT_INTERACTIVE)
            .long(OPT_INTERACTIVE)
            .help("prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, prompts always")
            .value_name("WHEN")
            .takes_value(true)
        )
        .arg(
            Arg::with_name(OPT_ONE_FILE_SYSTEM)
            .long(OPT_ONE_FILE_SYSTEM)
            .help("when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)")
        )
        .arg(
            Arg::with_name(OPT_NO_PRESERVE_ROOT)
            .long(OPT_NO_PRESERVE_ROOT)
            .help("do not treat '/' specially")
        )
        .arg(
            Arg::with_name(OPT_PRESERVE_ROOT)
            .long(OPT_PRESERVE_ROOT)
            .help("do not remove '/' (default)")
        )
        .arg(
            Arg::with_name(OPT_RECURSIVE).short("r")
            .long(OPT_RECURSIVE)
            .help("remove directories and their contents recursively")
        )
        .arg(
            // To mimic GNU's behavior we also want the '-R' flag. However, using clap's
            // alias method 'visible_alias("R")' would result in a long '--R' flag.
            Arg::with_name(OPT_RECURSIVE_R).short("R")
            .help("Equivalent to -r")
        )
        .arg(
            Arg::with_name(OPT_DIR)
            .short("d")
            .long(OPT_DIR)
            .help("remove empty directories")
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
            .short("v")
            .long(OPT_VERBOSE)
            .help("explain what is being done")
        )
        .arg(
            Arg::with_name(ARG_FILES)
            .multiple(true)
            .takes_value(true)
            .min_values(1)
        )
}
