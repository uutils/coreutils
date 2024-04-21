// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) copydir ficlone fiemap ftruncate linkgs lstat nlink nlinks pathbuf pwrite reflink strs xattrs symlinked deduplicated advcpmv nushell IRWXG IRWXO IRWXU IRWXUGO IRWXU IRWXG IRWXO IRWXUGO

use quick_error::quick_error;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
#[cfg(not(windows))]
use std::ffi::CString;
use std::fs::{self, File, Metadata, OpenOptions, Permissions};
use std::io;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf, StripPrefixError};

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, ArgMatches, Command};
use filetime::FileTime;
use indicatif::{ProgressBar, ProgressStyle};
#[cfg(unix)]
use libc::mkfifo;
use quick_error::ResultExt;

use platform::copy_on_write;
use uucore::display::Quotable;
use uucore::error::{set_exit_code, UClapError, UError, UResult, UUsageError};
use uucore::fs::{
    are_hardlinks_to_same_file, canonicalize, get_filename, is_symlink_loop,
    path_ends_with_terminator, paths_refer_to_same_file, FileInformation, MissingHandling,
    ResolveMode,
};
use uucore::{backup_control, update_control};
// These are exposed for projects (e.g. nushell) that want to create an `Options` value, which
// requires these enum.
pub use uucore::{backup_control::BackupMode, update_control::UpdateMode};
use uucore::{
    format_usage, help_about, help_section, help_usage, prompt_yes,
    shortcut_value_parser::ShortcutValueParser, show_error, show_warning, util_name,
};

use crate::copydir::copy_directory;

mod copydir;
mod platform;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        /// Simple io::Error wrapper
        IoErr(err: io::Error) { from() source(err) display("{}", err)}

        /// Wrapper for io::Error with path context
        IoErrContext(err: io::Error, path: String) {
            display("{}: {}", path, err)
            context(path: &'a str, err: io::Error) -> (err, path.to_owned())
            context(context: String, err: io::Error) -> (err, context)
            source(err)
        }

        /// General copy error
        Error(err: String) {
            display("{}", err)
            from(err: String) -> (err)
            from(err: &'static str) -> (err.to_string())
        }

        /// Represents the state when a non-fatal error has occurred
        /// and not all files were copied.
        NotAllFilesCopied {}

        /// Simple walkdir::Error wrapper
        WalkDirErr(err: walkdir::Error) { from() display("{}", err) source(err) }

        /// Simple std::path::StripPrefixError wrapper
        StripPrefixError(err: StripPrefixError) { from() }

        /// Result of a skipped file
        /// Currently happens when "no" is selected in interactive mode
        Skipped { }

        /// Result of a skipped file
        InvalidArgument(description: String) { display("{}", description) }

        /// All standard options are included as an an implementation
        /// path, but those that are not implemented yet should return
        /// a NotImplemented error.
        NotImplemented(opt: String) { display("Option '{}' not yet implemented.", opt) }

        /// Invalid arguments to backup
        Backup(description: String) { display("{}\nTry '{} --help' for more information.", description, uucore::execution_phrase()) }

        NotADirectory(path: PathBuf) { display("'{}' is not a directory", path.display()) }
    }
}

impl UError for Error {
    fn code(&self) -> i32 {
        EXIT_ERR
    }
}

pub type CopyResult<T> = Result<T, Error>;

/// Specifies how to overwrite files.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ClobberMode {
    Force,
    RemoveDestination,
    Standard,
}

/// Specifies whether files should be overwritten.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum OverwriteMode {
    /// [Default] Always overwrite existing files
    Clobber(ClobberMode),
    /// Prompt before overwriting a file
    Interactive(ClobberMode),
    /// Never overwrite a file
    NoClobber,
}

/// Possible arguments for `--reflink`.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ReflinkMode {
    Always,
    Auto,
    Never,
}

/// Possible arguments for `--sparse`.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SparseMode {
    Always,
    Auto,
    Never,
}

/// The expected file type of copy target
#[derive(Copy, Clone)]
pub enum TargetType {
    Directory,
    File,
}

/// Copy action to perform
#[derive(PartialEq)]
pub enum CopyMode {
    Link,
    SymLink,
    Copy,
    Update,
    AttrOnly,
}

/// Preservation settings for various attributes
///
/// It should be derived from options as follows:
///
///  - if there is a list of attributes to preserve (i.e. `--preserve=ATTR_LIST`) parse that list with [`Attributes::parse_iter`],
///  - if `-p` or `--preserve` is given without arguments, use [`Attributes::DEFAULT`],
///  - if `-a`/`--archive` is passed, use [`Attributes::ALL`],
///  - if `-d` is passed use [`Attributes::LINKS`],
///  - otherwise, use [`Attributes::NONE`].
///
/// For full compatibility with GNU, these options should also combine. We
/// currently only do a best effort imitation of that behavior, because it is
/// difficult to achieve in clap, especially with `--no-preserve`.
#[derive(Debug)]
pub struct Attributes {
    #[cfg(unix)]
    pub ownership: Preserve,
    pub mode: Preserve,
    pub timestamps: Preserve,
    pub context: Preserve,
    pub links: Preserve,
    pub xattr: Preserve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preserve {
    // explicit means whether the --no-preserve flag is used or not to distinguish out the default value.
    // e.g. --no-preserve=mode means mode = No { explicit = true }
    No { explicit: bool },
    Yes { required: bool },
}

impl PartialOrd for Preserve {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Preserve {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::No { .. }, Self::No { .. }) => Ordering::Equal,
            (Self::Yes { .. }, Self::No { .. }) => Ordering::Greater,
            (Self::No { .. }, Self::Yes { .. }) => Ordering::Less,
            (
                Self::Yes { required: req_self },
                Self::Yes {
                    required: req_other,
                },
            ) => req_self.cmp(req_other),
        }
    }
}

/// Options for the `cp` command
///
/// All options are public so that the options can be programmatically
/// constructed by other crates, such as nushell. That means that this struct
/// is part of our public API. It should therefore not be changed without good
/// reason.
///
/// The fields are documented with the arguments that determine their value.
#[allow(dead_code)]
pub struct Options {
    /// `--attributes-only`
    pub attributes_only: bool,
    /// `--backup[=CONTROL]`, `-b`
    pub backup: BackupMode,
    /// `--copy-contents`
    pub copy_contents: bool,
    /// `-H`
    pub cli_dereference: bool,
    /// Determines the type of copying that should be done
    ///
    /// Set by the following arguments:
    ///  - `-l`, `--link`: [`CopyMode::Link`]
    ///  - `-s`, `--symbolic-link`: [`CopyMode::SymLink`]
    ///  - `-u`, `--update[=WHEN]`: [`CopyMode::Update`]
    ///  - `--attributes-only`: [`CopyMode::AttrOnly`]
    ///  - otherwise: [`CopyMode::Copy`]
    pub copy_mode: CopyMode,
    /// `-L`, `--dereference`
    pub dereference: bool,
    /// `-T`, `--no-target-dir`
    pub no_target_dir: bool,
    /// `-x`, `--one-file-system`
    pub one_file_system: bool,
    /// Specifies what to do with an existing destination
    ///
    /// Set by the following arguments:
    ///  - `-i`, `--interactive`: [`OverwriteMode::Interactive`]
    ///  - `-n`, `--no-clobber`: [`OverwriteMode::NoClobber`]
    ///  - otherwise: [`OverwriteMode::Clobber`]
    ///
    /// The `Interactive` and `Clobber` variants have a [`ClobberMode`] argument,
    /// set by the following arguments:
    ///  - `-f`, `--force`: [`ClobberMode::Force`]
    ///  - `--remove-destination`: [`ClobberMode::RemoveDestination`]
    ///  - otherwise: [`ClobberMode::Standard`]
    pub overwrite: OverwriteMode,
    /// `--parents`
    pub parents: bool,
    /// `--sparse[=WHEN]`
    pub sparse_mode: SparseMode,
    /// `--strip-trailing-slashes`
    pub strip_trailing_slashes: bool,
    /// `--reflink[=WHEN]`
    pub reflink_mode: ReflinkMode,
    /// `--preserve=[=ATTRIBUTE_LIST]` and `--no-preserve=ATTRIBUTE_LIST`
    pub attributes: Attributes,
    /// `-R`, `-r`, `--recursive`
    pub recursive: bool,
    /// `-S`, `--suffix`
    pub backup_suffix: String,
    /// `-t`, `--target-directory`
    pub target_dir: Option<PathBuf>,
    /// `--update[=UPDATE]`
    pub update: UpdateMode,
    /// `--debug`
    pub debug: bool,
    /// `-v`, `--verbose`
    pub verbose: bool,
    /// `-g`, `--progress`
    pub progress_bar: bool,
}

/// Enum representing various debug states of the offload and reflink actions.
#[derive(Debug)]
#[allow(dead_code)] // All of them are used on Linux
enum OffloadReflinkDebug {
    Unknown,
    No,
    Yes,
    Avoided,
    Unsupported,
}

/// Enum representing various debug states of the sparse detection.
#[derive(Debug)]
#[allow(dead_code)] // silent for now until we use them
enum SparseDebug {
    Unknown,
    No,
    Zeros,
    SeekHole,
    SeekHoleZeros,
    Unsupported,
}

/// Struct that contains the debug state for each action in a file copy operation.
#[derive(Debug)]
struct CopyDebug {
    offload: OffloadReflinkDebug,
    reflink: OffloadReflinkDebug,
    sparse_detection: SparseDebug,
}

impl OffloadReflinkDebug {
    fn to_string(&self) -> &'static str {
        match self {
            Self::No => "no",
            Self::Yes => "yes",
            Self::Avoided => "avoided",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

impl SparseDebug {
    fn to_string(&self) -> &'static str {
        match self {
            Self::No => "no",
            Self::Zeros => "zeros",
            Self::SeekHole => "SEEK_HOLE",
            Self::SeekHoleZeros => "SEEK_HOLE + zeros",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

/// This function prints the debug information of a file copy operation if
/// no hard link or symbolic link is required, and data copy is required.
/// It prints the debug information of the offload, reflink, and sparse detection actions.
fn show_debug(copy_debug: &CopyDebug) {
    println!(
        "copy offload: {}, reflink: {}, sparse detection: {}",
        copy_debug.offload.to_string(),
        copy_debug.reflink.to_string(),
        copy_debug.sparse_detection.to_string(),
    );
}

const ABOUT: &str = help_about!("cp.md");
const USAGE: &str = help_usage!("cp.md");
const AFTER_HELP: &str = help_section!("after help", "cp.md");

static EXIT_ERR: i32 = 1;

// Argument constants
mod options {
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

#[cfg(unix)]
static PRESERVABLE_ATTRIBUTES: &[&str] = &[
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
                .overrides_with_all([
                    options::ARCHIVE,
                    options::PRESERVE_DEFAULT_ATTRIBUTES,
                    options::NO_PRESERVE,
                ])
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
                .overrides_with_all([options::PRESERVE, options::NO_PRESERVE, options::ARCHIVE])
                .help("same as --preserve=mode,ownership(unix only),timestamps")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_PRESERVE)
                .long(options::NO_PRESERVE)
                .value_name("ATTR_LIST")
                .overrides_with_all([
                    options::PRESERVE_DEFAULT_ATTRIBUTES,
                    options::PRESERVE,
                    options::ARCHIVE,
                ])
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
                .overrides_with_all([
                    options::PRESERVE_DEFAULT_ATTRIBUTES,
                    options::PRESERVE,
                    options::NO_PRESERVE,
                ])
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args);

    // The error is parsed here because we do not want version or help being printed to stderr.
    if let Err(e) = matches {
        let mut app = uu_app();

        match e.kind() {
            clap::error::ErrorKind::DisplayHelp => {
                app.print_help()?;
            }
            clap::error::ErrorKind::DisplayVersion => print!("{}", app.render_version()),
            _ => return Err(Box::new(e.with_exit_code(1))),
        };
    } else if let Ok(mut matches) = matches {
        let options = Options::from_matches(&matches)?;

        if options.overwrite == OverwriteMode::NoClobber && options.backup != BackupMode::NoBackup {
            return Err(UUsageError::new(
                EXIT_ERR,
                "options --backup and --no-clobber are mutually exclusive",
            ));
        }

        let paths: Vec<PathBuf> = matches
            .remove_many::<PathBuf>(options::PATHS)
            .map(|v| v.collect())
            .unwrap_or_default();

        let (sources, target) = parse_path_args(paths, &options)?;

        if let Err(error) = copy(&sources, &target, &options) {
            match error {
                // Error::NotAllFilesCopied is non-fatal, but the error
                // code should still be EXIT_ERR as does GNU cp
                Error::NotAllFilesCopied => {}
                // Else we caught a fatal bubbled-up error, log it to stderr
                _ => show_error!("{}", error),
            };
            set_exit_code(EXIT_ERR);
        }
    }

    Ok(())
}

impl ClobberMode {
    fn from_matches(matches: &ArgMatches) -> Self {
        if matches.get_flag(options::FORCE) {
            Self::Force
        } else if matches.get_flag(options::REMOVE_DESTINATION) {
            Self::RemoveDestination
        } else {
            Self::Standard
        }
    }
}

impl OverwriteMode {
    fn from_matches(matches: &ArgMatches) -> Self {
        if matches.get_flag(options::INTERACTIVE) {
            Self::Interactive(ClobberMode::from_matches(matches))
        } else if matches.get_flag(options::NO_CLOBBER) {
            Self::NoClobber
        } else {
            Self::Clobber(ClobberMode::from_matches(matches))
        }
    }
}

impl CopyMode {
    fn from_matches(matches: &ArgMatches) -> Self {
        if matches.get_flag(options::LINK) {
            Self::Link
        } else if matches.get_flag(options::SYMBOLIC_LINK) {
            Self::SymLink
        } else if matches
            .get_one::<String>(update_control::arguments::OPT_UPDATE)
            .is_some()
            || matches.get_flag(update_control::arguments::OPT_UPDATE_NO_ARG)
        {
            Self::Update
        } else if matches.get_flag(options::ATTRIBUTES_ONLY) {
            if matches.get_flag(options::REMOVE_DESTINATION) {
                Self::Copy
            } else {
                Self::AttrOnly
            }
        } else {
            Self::Copy
        }
    }
}

impl Attributes {
    pub const ALL: Self = Self {
        #[cfg(unix)]
        ownership: Preserve::Yes { required: true },
        mode: Preserve::Yes { required: true },
        timestamps: Preserve::Yes { required: true },
        context: {
            #[cfg(feature = "feat_selinux")]
            {
                Preserve::Yes { required: false }
            }
            #[cfg(not(feature = "feat_selinux"))]
            {
                Preserve::No { explicit: false }
            }
        },
        links: Preserve::Yes { required: true },
        xattr: Preserve::Yes { required: false },
    };

    pub const NONE: Self = Self {
        #[cfg(unix)]
        ownership: Preserve::No { explicit: false },
        mode: Preserve::No { explicit: false },
        timestamps: Preserve::No { explicit: false },
        context: Preserve::No { explicit: false },
        links: Preserve::No { explicit: false },
        xattr: Preserve::No { explicit: false },
    };

    // TODO: ownership is required if the user is root, for non-root users it's not required.
    pub const DEFAULT: Self = Self {
        #[cfg(unix)]
        ownership: Preserve::Yes { required: true },
        mode: Preserve::Yes { required: true },
        timestamps: Preserve::Yes { required: true },
        xattr: Preserve::Yes { required: true },
        ..Self::NONE
    };

    pub const LINKS: Self = Self {
        links: Preserve::Yes { required: true },
        ..Self::NONE
    };

    pub fn union(self, other: &Self) -> Self {
        Self {
            #[cfg(unix)]
            ownership: self.ownership.max(other.ownership),
            context: self.context.max(other.context),
            timestamps: self.timestamps.max(other.timestamps),
            mode: self.mode.max(other.mode),
            links: self.links.max(other.links),
            xattr: self.xattr.max(other.xattr),
        }
    }

    pub fn parse_iter<T>(values: impl Iterator<Item = T>) -> Result<Self, Error>
    where
        T: AsRef<str>,
    {
        let mut new = Self::NONE;
        for value in values {
            new = new.union(&Self::parse_single_string(value.as_ref())?);
        }
        Ok(new)
    }

    /// Tries to match string containing a parameter to preserve with the corresponding entry in the
    /// Attributes struct.
    fn parse_single_string(value: &str) -> Result<Self, Error> {
        let value = value.to_lowercase();

        if value == "all" {
            return Ok(Self::ALL);
        }

        let mut new = Self::NONE;
        let attribute = match value.as_ref() {
            "mode" => &mut new.mode,
            #[cfg(unix)]
            "ownership" => &mut new.ownership,
            "timestamps" => &mut new.timestamps,
            "context" => &mut new.context,
            "link" | "links" => &mut new.links,
            "xattr" => &mut new.xattr,
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "invalid attribute {}",
                    value.quote()
                )));
            }
        };

        *attribute = Preserve::Yes { required: true };

        Ok(new)
    }
}

impl Options {
    #[allow(clippy::cognitive_complexity)]
    fn from_matches(matches: &ArgMatches) -> CopyResult<Self> {
        let not_implemented_opts = vec![
            #[cfg(not(any(windows, unix)))]
            options::ONE_FILE_SYSTEM,
            options::CONTEXT,
            #[cfg(windows)]
            options::FORCE,
        ];

        for not_implemented_opt in not_implemented_opts {
            if matches.contains_id(not_implemented_opt)
                && matches.value_source(not_implemented_opt)
                    == Some(clap::parser::ValueSource::CommandLine)
            {
                return Err(Error::NotImplemented(not_implemented_opt.to_string()));
            }
        }

        let recursive = matches.get_flag(options::RECURSIVE) || matches.get_flag(options::ARCHIVE);

        let backup_mode = match backup_control::determine_backup_mode(matches) {
            Err(e) => return Err(Error::Backup(format!("{e}"))),
            Ok(mode) => mode,
        };
        let update_mode = update_control::determine_update_mode(matches);

        let backup_suffix = backup_control::determine_backup_suffix(matches);

        let overwrite = OverwriteMode::from_matches(matches);

        // Parse target directory options
        let no_target_dir = matches.get_flag(options::NO_TARGET_DIRECTORY);
        let target_dir = matches
            .get_one::<PathBuf>(options::TARGET_DIRECTORY)
            .cloned();

        if let Some(dir) = &target_dir {
            if !dir.is_dir() {
                return Err(Error::NotADirectory(dir.clone()));
            }
        };

        // Parse attributes to preserve
        let mut attributes =
            if let Some(attribute_strs) = matches.get_many::<String>(options::PRESERVE) {
                if attribute_strs.len() == 0 {
                    Attributes::DEFAULT
                } else {
                    Attributes::parse_iter(attribute_strs)?
                }
            } else if matches.get_flag(options::ARCHIVE) {
                // --archive is used. Same as --preserve=all
                Attributes::ALL
            } else if matches.get_flag(options::NO_DEREFERENCE_PRESERVE_LINKS) {
                Attributes::LINKS
            } else if matches.get_flag(options::PRESERVE_DEFAULT_ATTRIBUTES) {
                Attributes::DEFAULT
            } else {
                Attributes::NONE
            };

        // handling no-preserve options and adjusting the attributes
        if let Some(attribute_strs) = matches.get_many::<String>(options::NO_PRESERVE) {
            if attribute_strs.len() > 0 {
                let no_preserve_attributes = Attributes::parse_iter(attribute_strs)?;
                if matches!(no_preserve_attributes.links, Preserve::Yes { .. }) {
                    attributes.links = Preserve::No { explicit: true };
                } else if matches!(no_preserve_attributes.mode, Preserve::Yes { .. }) {
                    attributes.mode = Preserve::No { explicit: true };
                }
            }
        }

        #[cfg(not(feature = "feat_selinux"))]
        if let Preserve::Yes { required } = attributes.context {
            let selinux_disabled_error =
                Error::Error("SELinux was not enabled during the compile time!".to_string());
            if required {
                return Err(selinux_disabled_error);
            } else {
                show_error_if_needed(&selinux_disabled_error);
            }
        }

        let options = Self {
            attributes_only: matches.get_flag(options::ATTRIBUTES_ONLY),
            copy_contents: matches.get_flag(options::COPY_CONTENTS),
            cli_dereference: matches.get_flag(options::CLI_SYMBOLIC_LINKS),
            copy_mode: CopyMode::from_matches(matches),
            // No dereference is set with -p, -d and --archive
            dereference: !(matches.get_flag(options::NO_DEREFERENCE)
                || matches.get_flag(options::NO_DEREFERENCE_PRESERVE_LINKS)
                || matches.get_flag(options::ARCHIVE)
                || recursive)
                || matches.get_flag(options::DEREFERENCE),
            one_file_system: matches.get_flag(options::ONE_FILE_SYSTEM),
            parents: matches.get_flag(options::PARENTS),
            update: update_mode,
            debug: matches.get_flag(options::DEBUG),
            verbose: matches.get_flag(options::VERBOSE) || matches.get_flag(options::DEBUG),
            strip_trailing_slashes: matches.get_flag(options::STRIP_TRAILING_SLASHES),
            reflink_mode: {
                if let Some(reflink) = matches.get_one::<String>(options::REFLINK) {
                    match reflink.as_str() {
                        "always" => ReflinkMode::Always,
                        "auto" => ReflinkMode::Auto,
                        "never" => ReflinkMode::Never,
                        value => {
                            return Err(Error::InvalidArgument(format!(
                                "invalid argument {} for \'reflink\'",
                                value.quote()
                            )));
                        }
                    }
                } else {
                    #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
                    {
                        ReflinkMode::Auto
                    }
                    #[cfg(not(any(
                        target_os = "linux",
                        target_os = "android",
                        target_os = "macos"
                    )))]
                    {
                        ReflinkMode::Never
                    }
                }
            },
            sparse_mode: {
                if let Some(val) = matches.get_one::<String>(options::SPARSE) {
                    match val.as_str() {
                        "always" => SparseMode::Always,
                        "auto" => SparseMode::Auto,
                        "never" => SparseMode::Never,
                        _ => {
                            return Err(Error::InvalidArgument(format!(
                                "invalid argument {val} for \'sparse\'"
                            )));
                        }
                    }
                } else {
                    SparseMode::Auto
                }
            },
            backup: backup_mode,
            backup_suffix,
            overwrite,
            no_target_dir,
            attributes,
            recursive,
            target_dir,
            progress_bar: matches.get_flag(options::PROGRESS_BAR),
        };

        Ok(options)
    }

    fn dereference(&self, in_command_line: bool) -> bool {
        self.dereference || (in_command_line && self.cli_dereference)
    }

    fn preserve_hard_links(&self) -> bool {
        match self.attributes.links {
            Preserve::No { .. } => false,
            Preserve::Yes { .. } => true,
        }
    }

    #[cfg(unix)]
    fn preserve_mode(&self) -> (bool, bool) {
        match self.attributes.mode {
            Preserve::No { explicit } => {
                if explicit {
                    (false, true)
                } else {
                    (false, false)
                }
            }
            Preserve::Yes { .. } => (true, false),
        }
    }

    /// Whether to force overwriting the destination file.
    fn force(&self) -> bool {
        matches!(self.overwrite, OverwriteMode::Clobber(ClobberMode::Force))
    }
}

impl TargetType {
    /// Return TargetType required for `target`.
    ///
    /// Treat target as a dir if we have multiple sources or the target
    /// exists and already is a directory
    fn determine(sources: &[PathBuf], target: &Path) -> Self {
        if sources.len() > 1 || target.is_dir() {
            Self::Directory
        } else {
            Self::File
        }
    }
}

/// Returns tuple of (Source paths, Target)
fn parse_path_args(
    mut paths: Vec<PathBuf>,
    options: &Options,
) -> CopyResult<(Vec<PathBuf>, PathBuf)> {
    if paths.is_empty() {
        // No files specified
        return Err("missing file operand".into());
    } else if paths.len() == 1 && options.target_dir.is_none() {
        // Only one file specified
        return Err(format!("missing destination file operand after {:?}", paths[0]).into());
    }

    // Return an error if the user requested to copy more than one
    // file source to a file target
    if options.no_target_dir && options.target_dir.is_none() && paths.len() > 2 {
        return Err(format!("extra operand {:?}", paths[2]).into());
    }

    let target = match options.target_dir {
        Some(ref target) => {
            // All path args are sources, and the target dir was
            // specified separately
            target.clone()
        }
        None => {
            // If there was no explicit target-dir, then use the last
            // path_arg
            paths.pop().unwrap()
        }
    };

    if options.strip_trailing_slashes {
        for source in &mut paths {
            *source = source.components().as_path().to_owned();
        }
    }

    Ok((paths, target))
}

/// When handling errors, we don't always want to show them to the user. This function handles that.
fn show_error_if_needed(error: &Error) {
    match error {
        // When using --no-clobber, we don't want to show
        // an error message
        Error::NotAllFilesCopied => {
            // Need to return an error code
        }
        Error::Skipped => {
            // touch a b && echo "n"|cp -i a b && echo $?
            // should return an error from GNU 9.2
        }
        _ => {
            show_error!("{}", error);
        }
    }
}

/// Copy all `sources` to `target`.
///
/// Returns an `Err(Error::NotAllFilesCopied)` if at least one non-fatal error
/// was encountered.
///
/// Behavior is determined by the `options` parameter, see [`Options`] for details.
pub fn copy(sources: &[PathBuf], target: &Path, options: &Options) -> CopyResult<()> {
    let target_type = TargetType::determine(sources, target);
    verify_target_type(target, &target_type)?;

    let mut non_fatal_errors = false;
    let mut seen_sources = HashSet::with_capacity(sources.len());
    let mut symlinked_files = HashSet::new();

    // to remember the copied files for further usage.
    // the FileInformation implemented the Hash trait by using
    // 1. inode number
    // 2. device number
    // the combination of a file's inode number and device number is unique throughout all the file systems.
    //
    // key is the source file's information and the value is the destination filepath.
    let mut copied_files: HashMap<FileInformation, PathBuf> = HashMap::with_capacity(sources.len());
    // remember the copied destinations for further usage.
    // we can't use copied_files as it is because the key is the source file's information.
    let mut copied_destinations: HashSet<PathBuf> = HashSet::with_capacity(sources.len());

    let progress_bar = if options.progress_bar {
        let pb = ProgressBar::new(disk_usage(sources, options.recursive)?)
            .with_style(
                ProgressStyle::with_template(
                    "{msg}: [{elapsed_precise}] {wide_bar} {bytes:>7}/{total_bytes:7}",
                )
                .unwrap(),
            )
            .with_message(uucore::util_name());
        pb.tick();
        Some(pb)
    } else {
        None
    };

    for source in sources {
        if seen_sources.contains(source) {
            // FIXME: compare sources by the actual file they point to, not their path. (e.g. dir/file == dir/../dir/file in most cases)
            show_warning!("source file {} specified more than once", source.quote());
        } else {
            let dest = construct_dest_path(source, target, target_type, options)
                .unwrap_or_else(|_| target.to_path_buf());

            if fs::metadata(&dest).is_ok() && !fs::symlink_metadata(&dest)?.file_type().is_symlink()
            {
                // There is already a file and it isn't a symlink (managed in a different place)
                if copied_destinations.contains(&dest)
                    && options.backup != BackupMode::NumberedBackup
                {
                    // If the target file was already created in this cp call, do not overwrite
                    return Err(Error::Error(format!(
                        "will not overwrite just-created '{}' with '{}'",
                        dest.display(),
                        source.display()
                    )));
                }
            }

            if let Err(error) = copy_source(
                &progress_bar,
                source,
                target,
                target_type,
                options,
                &mut symlinked_files,
                &mut copied_files,
            ) {
                show_error_if_needed(&error);
                non_fatal_errors = true;
            }
            copied_destinations.insert(dest.clone());
        }
        seen_sources.insert(source);
    }

    if let Some(pb) = progress_bar {
        pb.finish();
    }

    if non_fatal_errors {
        Err(Error::NotAllFilesCopied)
    } else {
        Ok(())
    }
}

fn construct_dest_path(
    source_path: &Path,
    target: &Path,
    target_type: TargetType,
    options: &Options,
) -> CopyResult<PathBuf> {
    if options.no_target_dir && target.is_dir() {
        return Err(format!(
            "cannot overwrite directory {} with non-directory",
            target.quote()
        )
        .into());
    }

    if options.parents && !target.is_dir() {
        return Err("with --parents, the destination must be a directory".into());
    }

    Ok(match target_type {
        TargetType::Directory => {
            let root = if options.parents {
                Path::new("")
            } else {
                source_path.parent().unwrap_or(source_path)
            };
            localize_to_target(root, source_path, target)?
        }
        TargetType::File => target.to_path_buf(),
    })
}

fn copy_source(
    progress_bar: &Option<ProgressBar>,
    source: &Path,
    target: &Path,
    target_type: TargetType,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    copied_files: &mut HashMap<FileInformation, PathBuf>,
) -> CopyResult<()> {
    let source_path = Path::new(&source);
    if source_path.is_dir() {
        // Copy as directory
        copy_directory(
            progress_bar,
            source,
            target,
            options,
            symlinked_files,
            copied_files,
            true,
        )
    } else {
        // Copy as file
        let dest = construct_dest_path(source_path, target, target_type, options)?;
        let res = copy_file(
            progress_bar,
            source_path,
            dest.as_path(),
            options,
            symlinked_files,
            copied_files,
            true,
        );
        if options.parents {
            for (x, y) in aligned_ancestors(source, dest.as_path()) {
                copy_attributes(x, y, &options.attributes)?;
            }
        }
        res
    }
}

impl OverwriteMode {
    fn verify(&self, path: &Path) -> CopyResult<()> {
        match *self {
            Self::NoClobber => {
                eprintln!("{}: not replacing {}", util_name(), path.quote());
                Err(Error::NotAllFilesCopied)
            }
            Self::Interactive(_) => {
                if prompt_yes!("overwrite {}?", path.quote()) {
                    Ok(())
                } else {
                    Err(Error::Skipped)
                }
            }
            Self::Clobber(_) => Ok(()),
        }
    }
}

/// Handles errors for attributes preservation. If the attribute is not required, and
/// errored, tries to show error (see `show_error_if_needed` for additional behavior details).
/// If it's required, then the error is thrown.
fn handle_preserve<F: Fn() -> CopyResult<()>>(p: &Preserve, f: F) -> CopyResult<()> {
    match p {
        Preserve::No { .. } => {}
        Preserve::Yes { required } => {
            let result = f();
            if *required {
                result?;
            } else if let Err(error) = result {
                show_error_if_needed(&error);
            }
        }
    };
    Ok(())
}

/// Copy the specified attributes from one path to another.
pub(crate) fn copy_attributes(
    source: &Path,
    dest: &Path,
    attributes: &Attributes,
) -> CopyResult<()> {
    let context = &*format!("{} -> {}", source.quote(), dest.quote());
    let source_metadata = fs::symlink_metadata(source).context(context)?;

    // Ownership must be changed first to avoid interfering with mode change.
    #[cfg(unix)]
    handle_preserve(&attributes.ownership, || -> CopyResult<()> {
        use std::os::unix::prelude::MetadataExt;
        use uucore::perms::wrap_chown;
        use uucore::perms::Verbosity;
        use uucore::perms::VerbosityLevel;

        let dest_uid = source_metadata.uid();
        let dest_gid = source_metadata.gid();

        wrap_chown(
            dest,
            &dest.symlink_metadata().context(context)?,
            Some(dest_uid),
            Some(dest_gid),
            false,
            Verbosity {
                groups_only: false,
                level: VerbosityLevel::Normal,
            },
        )
        .map_err(Error::Error)?;

        Ok(())
    })?;

    handle_preserve(&attributes.mode, || -> CopyResult<()> {
        // The `chmod()` system call that underlies the
        // `fs::set_permissions()` call is unable to change the
        // permissions of a symbolic link. In that case, we just
        // do nothing, since every symbolic link has the same
        // permissions.
        if !dest.is_symlink() {
            fs::set_permissions(dest, source_metadata.permissions()).context(context)?;
            // FIXME: Implement this for windows as well
            #[cfg(feature = "feat_acl")]
            exacl::getfacl(source, None)
                .and_then(|acl| exacl::setfacl(&[dest], &acl, None))
                .map_err(|err| Error::Error(err.to_string()))?;
        }

        Ok(())
    })?;

    handle_preserve(&attributes.timestamps, || -> CopyResult<()> {
        let atime = FileTime::from_last_access_time(&source_metadata);
        let mtime = FileTime::from_last_modification_time(&source_metadata);
        if dest.is_symlink() {
            filetime::set_symlink_file_times(dest, atime, mtime)?;
        } else {
            filetime::set_file_times(dest, atime, mtime)?;
        }

        Ok(())
    })?;

    #[cfg(feature = "feat_selinux")]
    handle_preserve(&attributes.context, || -> CopyResult<()> {
        let context = selinux::SecurityContext::of_path(source, false, false).map_err(|e| {
            format!(
                "failed to get security context of {}: {}",
                source.display(),
                e
            )
        })?;
        if let Some(context) = context {
            context.set_for_path(dest, false, false).map_err(|e| {
                format!(
                    "failed to set security context for {}: {}",
                    dest.display(),
                    e
                )
            })?;
        }

        Ok(())
    })?;

    handle_preserve(&attributes.xattr, || -> CopyResult<()> {
        #[cfg(all(unix, not(target_os = "android")))]
        {
            let xattrs = xattr::list(source)?;
            for attr in xattrs {
                if let Some(attr_value) = xattr::get(source, attr.clone())? {
                    xattr::set(dest, attr, &attr_value[..])?;
                }
            }
        }
        #[cfg(not(all(unix, not(target_os = "android"))))]
        {
            // The documentation for GNU cp states:
            //
            // > Try to preserve SELinux security context and
            // > extended attributes (xattr), but ignore any failure
            // > to do that and print no corresponding diagnostic.
            //
            // so we simply do nothing here.
            //
            // TODO Silently ignore failures in the `#[cfg(unix)]`
            // block instead of terminating immediately on errors.
        }

        Ok(())
    })?;

    Ok(())
}

fn symlink_file(
    source: &Path,
    dest: &Path,
    symlinked_files: &mut HashSet<FileInformation>,
) -> CopyResult<()> {
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(source, dest).context(format!(
            "cannot create symlink {} to {}",
            get_filename(dest).unwrap_or("invalid file name").quote(),
            get_filename(source).unwrap_or("invalid file name").quote()
        ))?;
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(source, dest).context(format!(
            "cannot create symlink {} to {}",
            get_filename(dest).unwrap_or("invalid file name").quote(),
            get_filename(source).unwrap_or("invalid file name").quote()
        ))?;
    }
    if let Ok(file_info) = FileInformation::from_path(dest, false) {
        symlinked_files.insert(file_info);
    }
    Ok(())
}

fn context_for(src: &Path, dest: &Path) -> String {
    format!("{} -> {}", src.quote(), dest.quote())
}

/// Implements a simple backup copy for the destination file .
/// if is_dest_symlink flag is set to true dest will be renamed to backup_path
/// TODO: for the backup, should this function be replaced by `copy_file(...)`?
fn backup_dest(dest: &Path, backup_path: &Path, is_dest_symlink: bool) -> CopyResult<PathBuf> {
    if is_dest_symlink {
        fs::rename(dest, backup_path)?;
    } else {
        fs::copy(dest, backup_path)?;
    }
    Ok(backup_path.into())
}

/// Decide whether source and destination files are the same and
/// copying is forbidden.
///
/// Copying to the same file is only allowed if both `--backup` and
/// `--force` are specified and the file is a regular file.
fn is_forbidden_to_copy_to_same_file(
    source: &Path,
    dest: &Path,
    options: &Options,
    source_in_command_line: bool,
) -> bool {
    // TODO To match the behavior of GNU cp, we also need to check
    // that the file is a regular file.
    let source_is_symlink = source.is_symlink();
    let dest_is_symlink = dest.is_symlink();
    // only disable dereference if both source and dest is symlink and dereference flag is disabled
    let dereference_to_compare =
        options.dereference(source_in_command_line) || (!source_is_symlink || !dest_is_symlink);
    if !paths_refer_to_same_file(source, dest, dereference_to_compare) {
        return false;
    }
    if options.backup != BackupMode::NoBackup {
        if options.force() && !source_is_symlink {
            return false;
        }
        if source_is_symlink && !options.dereference {
            return false;
        }
        if dest_is_symlink {
            return false;
        }
        if !dest_is_symlink && !source_is_symlink && dest != source {
            return false;
        }
    }
    if options.copy_mode == CopyMode::Link {
        return false;
    }
    if options.copy_mode == CopyMode::SymLink && dest_is_symlink {
        return false;
    }
    if dest_is_symlink && source_is_symlink && !options.dereference {
        return false;
    }
    true
}

/// Back up, remove, or leave intact the destination file, depending on the options.
fn handle_existing_dest(
    source: &Path,
    dest: &Path,
    options: &Options,
    source_in_command_line: bool,
    copied_files: &mut HashMap<FileInformation, PathBuf>,
) -> CopyResult<()> {
    // Disallow copying a file to itself, unless `--force` and
    // `--backup` are both specified.
    if is_forbidden_to_copy_to_same_file(source, dest, options, source_in_command_line) {
        return Err(format!("{} and {} are the same file", source.quote(), dest.quote()).into());
    }

    if options.update != UpdateMode::ReplaceIfOlder {
        options.overwrite.verify(dest)?;
    }

    let mut is_dest_removed = false;
    let backup_path = backup_control::get_backup_path(options.backup, dest, &options.backup_suffix);
    if let Some(backup_path) = backup_path {
        if paths_refer_to_same_file(source, &backup_path, true) {
            return Err(format!(
                "backing up {} might destroy source;  {} not copied",
                dest.quote(),
                source.quote()
            )
            .into());
        } else {
            is_dest_removed = dest.is_symlink();
            backup_dest(dest, &backup_path, is_dest_removed)?;
        }
    }
    match options.overwrite {
        // FIXME: print that the file was removed if --verbose is enabled
        OverwriteMode::Clobber(ClobberMode::Force) => {
            if !is_dest_removed
                && (is_symlink_loop(dest) || fs::metadata(dest)?.permissions().readonly())
            {
                fs::remove_file(dest)?;
            }
        }
        OverwriteMode::Clobber(ClobberMode::RemoveDestination) => {
            fs::remove_file(dest)?;
        }
        OverwriteMode::Clobber(ClobberMode::Standard) => {
            // Consider the following files:
            //
            // * `src/f` - a regular file
            // * `src/link` - a hard link to `src/f`
            // * `dest/src/f` - a different regular file
            //
            // In this scenario, if we do `cp -a src/ dest/`, it is
            // possible that the order of traversal causes `src/link`
            // to get copied first (to `dest/src/link`). In that case,
            // in order to make sure `dest/src/link` is a hard link to
            // `dest/src/f` and `dest/src/f` has the contents of
            // `src/f`, we delete the existing file to allow the hard
            // linking.

            if options.preserve_hard_links()
            // only try to remove dest file only if the current source 
            // is hardlink to a file that is already copied  
                && copied_files.contains_key(
                    &FileInformation::from_path(
                        source,
                        options.dereference(source_in_command_line),
                    )
                    .context(format!("cannot stat {}", source.quote()))?,
                )
                && !is_dest_removed
            {
                fs::remove_file(dest)?;
            }
        }
        _ => (),
    };

    Ok(())
}

/// Decide whether the given path exists.
fn file_or_link_exists(path: &Path) -> bool {
    // Using `Path.exists()` or `Path.try_exists()` is not sufficient,
    // because if `path` is a symbolic link and there are too many
    // levels of symbolic link, then those methods will return false
    // or an OS error.
    path.symlink_metadata().is_ok()
}

/// Zip the ancestors of a source path and destination path.
///
/// # Examples
///
/// ```rust,ignore
/// let actual = aligned_ancestors(&Path::new("a/b/c"), &Path::new("d/a/b/c"));
/// let expected = vec![
///     (Path::new("a"), Path::new("d/a")),
///     (Path::new("a/b"), Path::new("d/a/b")),
/// ];
/// assert_eq!(actual, expected);
/// ```
fn aligned_ancestors<'a>(source: &'a Path, dest: &'a Path) -> Vec<(&'a Path, &'a Path)> {
    // Collect the ancestors of each. For example, if `source` is
    // "a/b/c", then the ancestors are "a/b/c", "a/b", "a/", and "".
    let source_ancestors: Vec<&Path> = source.ancestors().collect();
    let dest_ancestors: Vec<&Path> = dest.ancestors().collect();

    // For this particular application, we don't care about the null
    // path "" and we don't care about the full path (e.g. "a/b/c"),
    // so we exclude those.
    let n = source_ancestors.len();
    let source_ancestors = &source_ancestors[1..n - 1];

    // Get the matching number of elements from the ancestors of the
    // destination path (for example, get "d/a" and "d/a/b").
    let k = source_ancestors.len();
    let dest_ancestors = &dest_ancestors[1..=k];

    // Now we have two slices of the same length, so we zip them.
    let mut result = vec![];
    for (x, y) in source_ancestors
        .iter()
        .rev()
        .zip(dest_ancestors.iter().rev())
    {
        result.push((*x, *y));
    }
    result
}

fn print_verbose_output(
    parents: bool,
    progress_bar: &Option<ProgressBar>,
    source: &Path,
    dest: &Path,
) {
    if let Some(pb) = progress_bar {
        // Suspend (hide) the progress bar so the println won't overlap with the progress bar.
        pb.suspend(|| {
            print_paths(parents, source, dest);
        });
    } else {
        print_paths(parents, source, dest);
    }
}

fn print_paths(parents: bool, source: &Path, dest: &Path) {
    if parents {
        // For example, if copying file `a/b/c` and its parents
        // to directory `d/`, then print
        //
        //     a -> d/a
        //     a/b -> d/a/b
        //
        for (x, y) in aligned_ancestors(source, dest) {
            println!("{} -> {}", x.display(), y.display());
        }
    }

    println!("{}", context_for(source, dest));
}

/// Handles the copy mode for a file copy operation.
///
/// This function determines how to copy a file based on the provided options.
/// It supports different copy modes, including hard linking, copying, symbolic linking, updating, and attribute-only copying.
/// It also handles file backups, overwriting, and dereferencing based on the provided options.
///
/// # Returns
///
/// * `Ok(())` - The file was copied successfully.
/// * `Err(CopyError)` - An error occurred while copying the file.
fn handle_copy_mode(
    source: &Path,
    dest: &Path,
    options: &Options,
    context: &str,
    source_metadata: Metadata,
    symlinked_files: &mut HashSet<FileInformation>,
    source_in_command_line: bool,
) -> CopyResult<()> {
    let source_file_type = source_metadata.file_type();

    let source_is_symlink = source_file_type.is_symlink();

    #[cfg(unix)]
    let source_is_fifo = source_file_type.is_fifo();
    #[cfg(not(unix))]
    let source_is_fifo = false;

    match options.copy_mode {
        CopyMode::Link => {
            if dest.exists() {
                let backup_path =
                    backup_control::get_backup_path(options.backup, dest, &options.backup_suffix);
                if let Some(backup_path) = backup_path {
                    backup_dest(dest, &backup_path, dest.is_symlink())?;
                    fs::remove_file(dest)?;
                }
                if options.overwrite == OverwriteMode::Clobber(ClobberMode::Force) {
                    fs::remove_file(dest)?;
                }
            }
            if options.dereference(source_in_command_line) && source.is_symlink() {
                let resolved =
                    canonicalize(source, MissingHandling::Missing, ResolveMode::Physical).unwrap();
                fs::hard_link(resolved, dest)
            } else {
                fs::hard_link(source, dest)
            }
            .context(format!(
                "cannot create hard link {} to {}",
                get_filename(dest).unwrap_or("invalid file name").quote(),
                get_filename(source).unwrap_or("invalid file name").quote()
            ))?;
        }
        CopyMode::Copy => {
            copy_helper(
                source,
                dest,
                options,
                context,
                source_is_symlink,
                source_is_fifo,
                symlinked_files,
            )?;
        }
        CopyMode::SymLink => {
            if dest.exists() && options.overwrite == OverwriteMode::Clobber(ClobberMode::Force) {
                fs::remove_file(dest)?;
            }
            symlink_file(source, dest, symlinked_files)?;
        }
        CopyMode::Update => {
            if dest.exists() {
                match options.update {
                    update_control::UpdateMode::ReplaceAll => {
                        copy_helper(
                            source,
                            dest,
                            options,
                            context,
                            source_is_symlink,
                            source_is_fifo,
                            symlinked_files,
                        )?;
                    }
                    update_control::UpdateMode::ReplaceNone => {
                        if options.debug {
                            println!("skipped {}", dest.quote());
                        }

                        return Ok(());
                    }
                    update_control::UpdateMode::ReplaceIfOlder => {
                        let dest_metadata = fs::symlink_metadata(dest)?;

                        let src_time = source_metadata.modified()?;
                        let dest_time = dest_metadata.modified()?;
                        if src_time <= dest_time {
                            return Ok(());
                        } else {
                            options.overwrite.verify(dest)?;

                            copy_helper(
                                source,
                                dest,
                                options,
                                context,
                                source_is_symlink,
                                source_is_fifo,
                                symlinked_files,
                            )?;
                        }
                    }
                }
            } else {
                copy_helper(
                    source,
                    dest,
                    options,
                    context,
                    source_is_symlink,
                    source_is_fifo,
                    symlinked_files,
                )?;
            }
        }
        CopyMode::AttrOnly => {
            OpenOptions::new()
                .write(true)
                .truncate(false)
                .create(true)
                .open(dest)
                .unwrap();
        }
    };

    Ok(())
}

/// Calculates the permissions for the destination file in a copy operation.
///
/// If the destination file already exists, its current permissions are returned.
/// If the destination file does not exist, the source file's permissions are used,
/// with the `no-preserve` option and the umask taken into account on Unix platforms.
/// # Returns
///
/// * `Ok(Permissions)` - The calculated permissions for the destination file.
/// * `Err(CopyError)` - An error occurred while getting the metadata of the destination file.
/// Allow unused variables for Windows (on options)
#[allow(unused_variables)]
fn calculate_dest_permissions(
    dest: &Path,
    source_metadata: &Metadata,
    options: &Options,
    context: &str,
) -> CopyResult<Permissions> {
    if dest.exists() {
        Ok(dest.symlink_metadata().context(context)?.permissions())
    } else {
        #[cfg(unix)]
        {
            let mut permissions = source_metadata.permissions();
            let mode = handle_no_preserve_mode(options, permissions.mode());

            // Apply umask
            use uucore::mode::get_umask;
            let mode = mode & !get_umask();
            permissions.set_mode(mode);
            Ok(permissions)
        }
        #[cfg(not(unix))]
        {
            let permissions = source_metadata.permissions();
            Ok(permissions)
        }
    }
}

/// Copy the a file from `source` to `dest`. `source` will be dereferenced if
/// `options.dereference` is set to true. `dest` will be dereferenced only if
/// the source was not a symlink.
///
/// Behavior when copying to existing files is contingent on the
/// `options.overwrite` mode. If a file is skipped, the return type
/// should be `Error:Skipped`
///
/// The original permissions of `source` will be copied to `dest`
/// after a successful copy.
#[allow(clippy::cognitive_complexity)]
fn copy_file(
    progress_bar: &Option<ProgressBar>,
    source: &Path,
    dest: &Path,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    copied_files: &mut HashMap<FileInformation, PathBuf>,
    source_in_command_line: bool,
) -> CopyResult<()> {
    let source_is_symlink = source.is_symlink();
    let dest_is_symlink = dest.is_symlink();
    // Fail if dest is a dangling symlink or a symlink this program created previously
    if dest_is_symlink {
        if FileInformation::from_path(dest, false)
            .map(|info| symlinked_files.contains(&info))
            .unwrap_or(false)
        {
            return Err(Error::Error(format!(
                "will not copy '{}' through just-created symlink '{}'",
                source.display(),
                dest.display()
            )));
        }
        let copy_contents = options.dereference(source_in_command_line) || !source_is_symlink;
        if copy_contents
            && !dest.exists()
            && !matches!(
                options.overwrite,
                OverwriteMode::Clobber(ClobberMode::RemoveDestination)
            )
            && !is_symlink_loop(dest)
            && std::env::var_os("POSIXLY_CORRECT").is_none()
        {
            return Err(Error::Error(format!(
                "not writing through dangling symlink '{}'",
                dest.display()
            )));
        }
        if paths_refer_to_same_file(source, dest, true)
            && matches!(
                options.overwrite,
                OverwriteMode::Clobber(ClobberMode::RemoveDestination)
            )
        {
            fs::remove_file(dest)?;
        }
    }

    if are_hardlinks_to_same_file(source, dest)
        && source != dest
        && matches!(
            options.overwrite,
            OverwriteMode::Clobber(ClobberMode::RemoveDestination)
        )
    {
        fs::remove_file(dest)?;
    }

    if file_or_link_exists(dest)
        && (!options.attributes_only
            || matches!(
                options.overwrite,
                OverwriteMode::Clobber(ClobberMode::RemoveDestination)
            ))
    {
        if paths_refer_to_same_file(source, dest, true) && options.copy_mode == CopyMode::Link {
            if source_is_symlink {
                if !dest_is_symlink {
                    return Ok(());
                }
                if !options.dereference {
                    return Ok(());
                }
            } else if options.backup != BackupMode::NoBackup && !dest_is_symlink {
                if source == dest {
                    if !options.force() {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
            }
        }
        handle_existing_dest(source, dest, options, source_in_command_line, copied_files)?;
        if are_hardlinks_to_same_file(source, dest) {
            if options.copy_mode == CopyMode::Copy && options.backup != BackupMode::NoBackup {
                return Ok(());
            }
            if options.copy_mode == CopyMode::Link && (!source_is_symlink || !dest_is_symlink) {
                return Ok(());
            }
        }
    }

    if options.attributes_only
        && source_is_symlink
        && !matches!(
            options.overwrite,
            OverwriteMode::Clobber(ClobberMode::RemoveDestination)
        )
    {
        return Err(format!(
            "cannot change attribute {}: Source file is a non regular file",
            dest.quote()
        )
        .into());
    }

    if options.preserve_hard_links() {
        // if we encounter a matching device/inode pair in the source tree
        // we can arrange to create a hard link between the corresponding names
        // in the destination tree.
        if let Some(new_source) = copied_files.get(
            &FileInformation::from_path(source, options.dereference(source_in_command_line))
                .context(format!("cannot stat {}", source.quote()))?,
        ) {
            std::fs::hard_link(new_source, dest)?;
            return Ok(());
        };
    }

    if options.verbose {
        print_verbose_output(options.parents, progress_bar, source, dest);
    }

    // Calculate the context upfront before canonicalizing the path
    let context = context_for(source, dest);
    let context = context.as_str();

    let source_metadata = {
        let result = if options.dereference(source_in_command_line) {
            fs::metadata(source)
        } else {
            fs::symlink_metadata(source)
        };
        result.context(context)?
    };

    let dest_permissions = calculate_dest_permissions(dest, &source_metadata, options, context)?;

    handle_copy_mode(
        source,
        dest,
        options,
        context,
        source_metadata,
        symlinked_files,
        source_in_command_line,
    )?;

    // TODO: implement something similar to gnu's lchown
    if !dest_is_symlink {
        // Here, to match GNU semantics, we quietly ignore an error
        // if a user does not have the correct ownership to modify
        // the permissions of a file.
        //
        // FWIW, the OS will throw an error later, on the write op, if
        // the user does not have permission to write to the file.
        fs::set_permissions(dest, dest_permissions).ok();
    }

    copy_attributes(source, dest, &options.attributes)?;

    copied_files.insert(
        FileInformation::from_path(source, options.dereference(source_in_command_line))?,
        dest.to_path_buf(),
    );

    if let Some(progress_bar) = progress_bar {
        progress_bar.inc(fs::metadata(source)?.len());
    }

    Ok(())
}

#[cfg(unix)]
fn handle_no_preserve_mode(options: &Options, org_mode: u32) -> u32 {
    let (is_preserve_mode, is_explicit_no_preserve_mode) = options.preserve_mode();
    if !is_preserve_mode {
        use libc::{
            S_IRGRP, S_IROTH, S_IRUSR, S_IRWXG, S_IRWXO, S_IRWXU, S_IWGRP, S_IWOTH, S_IWUSR,
        };

        #[cfg(not(any(
            target_os = "android",
            target_os = "macos",
            target_os = "macos-12",
            target_os = "freebsd",
            target_os = "redox",
        )))]
        {
            const MODE_RW_UGO: u32 = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;
            const S_IRWXUGO: u32 = S_IRWXU | S_IRWXG | S_IRWXO;
            if is_explicit_no_preserve_mode {
                return MODE_RW_UGO;
            } else {
                return org_mode & S_IRWXUGO;
            };
        }

        #[cfg(any(
            target_os = "android",
            target_os = "macos",
            target_os = "macos-12",
            target_os = "freebsd",
            target_os = "redox",
        ))]
        {
            const MODE_RW_UGO: u32 =
                (S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH) as u32;
            const S_IRWXUGO: u32 = (S_IRWXU | S_IRWXG | S_IRWXO) as u32;
            if is_explicit_no_preserve_mode {
                return MODE_RW_UGO;
            } else {
                return org_mode & S_IRWXUGO;
            };
        }
    }

    org_mode
}

/// Copy the file from `source` to `dest` either using the normal `fs::copy` or a
/// copy-on-write scheme if --reflink is specified and the filesystem supports it.
fn copy_helper(
    source: &Path,
    dest: &Path,
    options: &Options,
    context: &str,
    source_is_symlink: bool,
    source_is_fifo: bool,
    symlinked_files: &mut HashSet<FileInformation>,
) -> CopyResult<()> {
    if options.parents {
        let parent = dest.parent().unwrap_or(dest);
        fs::create_dir_all(parent)?;
    }

    if path_ends_with_terminator(dest) && !dest.is_dir() {
        return Err(Error::NotADirectory(dest.to_path_buf()));
    }

    if source.as_os_str() == "/dev/null" {
        /* workaround a limitation of fs::copy
         * https://github.com/rust-lang/rust/issues/79390
         */
        File::create(dest).context(dest.display().to_string())?;
    } else if source_is_fifo && options.recursive && !options.copy_contents {
        #[cfg(unix)]
        copy_fifo(dest, options.overwrite)?;
    } else if source_is_symlink {
        copy_link(source, dest, symlinked_files)?;
    } else {
        let copy_debug = copy_on_write(
            source,
            dest,
            options.reflink_mode,
            options.sparse_mode,
            context,
            #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
            source_is_fifo,
        )?;

        if !options.attributes_only && options.debug {
            show_debug(&copy_debug);
        }
    }

    Ok(())
}

// "Copies" a FIFO by creating a new one. This workaround is because Rust's
// built-in fs::copy does not handle FIFOs (see rust-lang/rust/issues/79390).
#[cfg(unix)]
fn copy_fifo(dest: &Path, overwrite: OverwriteMode) -> CopyResult<()> {
    if dest.exists() {
        overwrite.verify(dest)?;
        fs::remove_file(dest)?;
    }

    let name = CString::new(dest.as_os_str().as_bytes()).unwrap();
    let err = unsafe { mkfifo(name.as_ptr(), 0o666) };
    if err == -1 {
        return Err(format!("cannot create fifo {}: File exists", dest.quote()).into());
    }
    Ok(())
}

fn copy_link(
    source: &Path,
    dest: &Path,
    symlinked_files: &mut HashSet<FileInformation>,
) -> CopyResult<()> {
    // Here, we will copy the symlink itself (actually, just recreate it)
    let link = fs::read_link(source)?;
    // we always need to remove the file to be able to create a symlink,
    // even if it is writeable.
    if dest.is_symlink() || dest.is_file() {
        fs::remove_file(dest)?;
    }
    symlink_file(&link, dest, symlinked_files)
}

/// Generate an error message if `target` is not the correct `target_type`
pub fn verify_target_type(target: &Path, target_type: &TargetType) -> CopyResult<()> {
    match (target_type, target.is_dir()) {
        (&TargetType::Directory, false) => {
            Err(format!("target: {} is not a directory", target.quote()).into())
        }
        (&TargetType::File, true) => Err(format!(
            "cannot overwrite directory {} with non-directory",
            target.quote()
        )
        .into()),
        _ => Ok(()),
    }
}

/// Remove the `root` prefix from `source` and prefix it with `target`
/// to create a file that is local to `target`
/// # Examples
///
/// ```ignore
/// assert!(uu_cp::localize_to_target(
///     &Path::new("a/source/"),
///     &Path::new("a/source/c.txt"),
///     &Path::new("target/"),
/// ).unwrap() == Path::new("target/c.txt"))
/// ```
pub fn localize_to_target(root: &Path, source: &Path, target: &Path) -> CopyResult<PathBuf> {
    let local_to_root = source.strip_prefix(root)?;
    Ok(target.join(local_to_root))
}

/// Get the total size of a slice of files and directories.
///
/// This function is much like the `du` utility, by recursively getting the sizes of files in directories.
/// Files are not deduplicated when appearing in multiple sources. If `recursive` is set to `false`, the
/// directories in `paths` will be ignored.
fn disk_usage(paths: &[PathBuf], recursive: bool) -> io::Result<u64> {
    let mut total = 0;
    for p in paths {
        let md = fs::metadata(p)?;
        if md.file_type().is_dir() {
            if recursive {
                total += disk_usage_directory(p)?;
            }
        } else {
            total += md.len();
        }
    }
    Ok(total)
}

/// A helper for `disk_usage` specialized for directories.
fn disk_usage_directory(p: &Path) -> io::Result<u64> {
    let mut total = 0;

    for entry in fs::read_dir(p)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            total += disk_usage_directory(&entry.path())?;
        } else {
            total += entry.metadata()?.len();
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {

    use crate::{aligned_ancestors, localize_to_target};
    use std::path::Path;

    #[test]
    fn test_cp_localize_to_target() {
        let root = Path::new("a/source/");
        let source = Path::new("a/source/c.txt");
        let target = Path::new("target/");
        let actual = localize_to_target(root, source, target).unwrap();
        let expected = Path::new("target/c.txt");
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_aligned_ancestors() {
        let actual = aligned_ancestors(Path::new("a/b/c"), Path::new("d/a/b/c"));
        let expected = vec![
            (Path::new("a"), Path::new("d/a")),
            (Path::new("a/b"), Path::new("d/a/b")),
        ];
        assert_eq!(actual, expected);
    }
}
