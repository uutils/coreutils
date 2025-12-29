// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) copydir ficlone fiemap ftruncate linkgs lstat nlink nlinks pathbuf pwrite reflink strs xattrs symlinked deduplicated advcpmv nushell IRWXG IRWXO IRWXU IRWXUGO IRWXU IRWXG IRWXO IRWXUGO

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::Display;
use std::fs::{self, Metadata, OpenOptions, Permissions};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf, StripPrefixError};
use std::{fmt, io};
#[cfg(all(unix, not(target_os = "android")))]
use uucore::fsxattr::copy_xattrs;
use uucore::translate;

use clap::{Arg, ArgAction, ArgMatches, Command, builder::ValueParser, value_parser};
use filetime::FileTime;
use indicatif::{ProgressBar, ProgressStyle};
use thiserror::Error;

use platform::copy_on_write;
use uucore::display::Quotable;
use uucore::error::{UError, UResult, UUsageError, set_exit_code};
#[cfg(unix)]
use uucore::fs::make_fifo;
use uucore::fs::{
    FileInformation, MissingHandling, ResolveMode, are_hardlinks_to_same_file, canonicalize,
    get_filename, is_symlink_loop, normalize_path, path_ends_with_terminator,
    paths_refer_to_same_file,
};
use uucore::{backup_control, update_control};
// These are exposed for projects (e.g. nushell) that want to create an `Options` value, which
// requires these enum.
pub use uucore::{backup_control::BackupMode, update_control::UpdateMode};
use uucore::{
    format_usage, parser::shortcut_value_parser::ShortcutValueParser, prompt_yes, show_error,
    show_warning,
};

use crate::copydir::copy_directory;

mod copydir;
mod platform;

#[derive(Debug, Error)]
pub enum CpError {
    /// Simple [`io::Error`] wrapper
    #[error("{0}")]
    IoErr(#[from] io::Error),

    /// Wrapper for [`io::Error`] with path context
    #[error("{1}: {0}")]
    IoErrContext(io::Error, String),

    /// General copy error
    #[error("{0}")]
    Error(String),

    /// Represents the state when a non-fatal error has occurred
    /// and not all files were copied.
    #[error("{}", translate!("cp-error-not-all-files-copied"))]
    NotAllFilesCopied,

    /// Simple [`walkdir::Error`] wrapper
    #[error("{0}")]
    WalkDirErr(#[from] walkdir::Error),

    /// Simple [`StripPrefixError`] wrapper
    #[error(transparent)]
    StripPrefixError(#[from] StripPrefixError),

    /// Result of a skipped file
    /// Currently happens when "no" is selected in interactive mode or when
    /// `no-clobber` flag is set and destination is already present.
    /// `exit with error` is used to determine which exit code should be returned.
    #[error("Skipped copying file (exit with error = {0})")]
    Skipped(bool),

    /// Invalid argument error
    #[error("{0}")]
    InvalidArgument(String),

    /// All standard options are included as an implementation
    /// path, but those that are not implemented yet should return
    /// a `NotImplemented` error.
    #[error("{}", translate!("cp-error-option-not-implemented", "option" => 0))]
    NotImplemented(String),

    /// Invalid arguments to backup
    #[error(transparent)]
    Backup(#[from] BackupError),

    #[error("{}", translate!("cp-error-not-a-directory", "path" => .0.quote()))]
    NotADirectory(PathBuf),
}

// Manual impl for &str
impl From<&'static str> for CpError {
    fn from(s: &'static str) -> Self {
        Self::Error(s.to_string())
    }
}

impl From<String> for CpError {
    fn from(s: String) -> Self {
        Self::Error(s)
    }
}

#[derive(Debug)]
pub struct BackupError(String);

impl Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            translate!("cp-error-backup-format", "error" => self.0.clone(), "exec" => uucore::execution_phrase())
        )
    }
}

impl std::error::Error for BackupError {}

impl UError for CpError {
    fn code(&self) -> i32 {
        EXIT_ERR
    }
}

pub type CopyResult<T> = Result<T, CpError>;

/// Specifies how to overwrite files.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ClobberMode {
    Force,
    RemoveDestination,
    #[default]
    Standard,
}

/// Specifies whether files should be overwritten.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OverwriteMode {
    /// [Default] Always overwrite existing files
    Clobber(ClobberMode),
    /// Prompt before overwriting a file
    Interactive(ClobberMode),
    /// Never overwrite a file
    NoClobber,
}

impl Default for OverwriteMode {
    fn default() -> Self {
        Self::Clobber(ClobberMode::default())
    }
}

/// Possible arguments for `--reflink`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReflinkMode {
    Always,
    Auto,
    Never,
}

impl Default for ReflinkMode {
    #[allow(clippy::derivable_impls)]
    fn default() -> Self {
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
        {
            Self::Auto
        }
        #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
        {
            Self::Never
        }
    }
}

/// Possible arguments for `--sparse`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum SparseMode {
    Always,
    #[default]
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
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum CopyMode {
    Link,
    SymLink,
    #[default]
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Attributes {
    #[cfg(unix)]
    pub ownership: Preserve,
    pub mode: Preserve,
    pub timestamps: Preserve,
    pub context: Preserve,
    pub links: Preserve,
    pub xattr: Preserve,
}

impl Default for Attributes {
    fn default() -> Self {
        Self::NONE
    }
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
#[derive(Debug, Clone, Eq, PartialEq)]
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
    /// -Z
    pub set_selinux_context: bool,
    // --context
    pub context: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            attributes_only: false,
            backup: BackupMode::default(),
            copy_contents: false,
            cli_dereference: false,
            copy_mode: CopyMode::default(),
            dereference: false,
            no_target_dir: false,
            one_file_system: false,
            overwrite: OverwriteMode::default(),
            parents: false,
            sparse_mode: SparseMode::default(),
            strip_trailing_slashes: false,
            reflink_mode: ReflinkMode::default(),
            attributes: Attributes::default(),
            recursive: false,
            backup_suffix: backup_control::DEFAULT_BACKUP_SUFFIX.to_owned(),
            target_dir: None,
            update: UpdateMode::default(),
            debug: false,
            verbose: false,
            progress_bar: false,
            set_selinux_context: false,
            context: None,
        }
    }
}

/// Enum representing if a file has been skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PerformedAction {
    Copied,
    Skipped,
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

impl Display for OffloadReflinkDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::No => translate!("cp-debug-enum-no"),
            Self::Yes => translate!("cp-debug-enum-yes"),
            Self::Avoided => translate!("cp-debug-enum-avoided"),
            Self::Unsupported => translate!("cp-debug-enum-unsupported"),
            Self::Unknown => translate!("cp-debug-enum-unknown"),
        };
        write!(f, "{msg}")
    }
}

impl Display for SparseDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::No => translate!("cp-debug-enum-no"),
            Self::Zeros => translate!("cp-debug-enum-zeros"),
            Self::SeekHole => translate!("cp-debug-enum-seek-hole"),
            Self::SeekHoleZeros => translate!("cp-debug-enum-seek-hole-zeros"),
            Self::Unsupported => translate!("cp-debug-enum-unsupported"),
            Self::Unknown => translate!("cp-debug-enum-unknown"),
        };
        write!(f, "{msg}")
    }
}

/// This function prints the debug information of a file copy operation if
/// no hard link or symbolic link is required, and data copy is required.
/// It prints the debug information of the offload, reflink, and sparse detection actions.
fn show_debug(copy_debug: &CopyDebug) {
    println!(
        "{}",
        translate!("cp-debug-copy-offload", "offload" => copy_debug.offload, "reflink" => copy_debug.reflink, "sparse" => copy_debug.sparse_detection)
    );
}

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
    pub const SELINUX: &str = "Z";
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

const PRESERVE_DEFAULT_VALUES: &str = if cfg!(unix) {
    "mode,ownership,timestamp"
} else {
    "mode,timestamp"
};

pub fn uu_app() -> Command {
    const MODE_ARGS: &[&str] = &[
        options::LINK,
        options::REFLINK,
        options::SYMBOLIC_LINK,
        options::ATTRIBUTES_ONLY,
        options::COPY_CONTENTS,
    ];
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("cp-about"))
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("cp-usage")))
        .after_help(format!(
            "{}\n\n{}",
            translate!("cp-after-help"),
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
                .help(translate!("cp-help-target-directory")),
        )
        .arg(
            Arg::new(options::NO_TARGET_DIRECTORY)
                .short('T')
                .long(options::NO_TARGET_DIRECTORY)
                .conflicts_with(options::TARGET_DIRECTORY)
                .help(translate!("cp-help-no-target-directory"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INTERACTIVE)
                .short('i')
                .long(options::INTERACTIVE)
                .overrides_with(options::NO_CLOBBER)
                .help(translate!("cp-help-interactive"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LINK)
                .short('l')
                .long(options::LINK)
                .overrides_with_all(MODE_ARGS)
                .help(translate!("cp-help-link"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CLOBBER)
                .short('n')
                .long(options::NO_CLOBBER)
                .overrides_with(options::INTERACTIVE)
                .help(translate!("cp-help-no-clobber"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .visible_short_alias('r')
                .long(options::RECURSIVE)
                // --archive sets this option
                .help(translate!("cp-help-recursive"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRIP_TRAILING_SLASHES)
                .long(options::STRIP_TRAILING_SLASHES)
                .help(translate!("cp-help-strip-trailing-slashes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEBUG)
                .long(options::DEBUG)
                .help(translate!("cp-help-debug"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help(translate!("cp-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SYMBOLIC_LINK)
                .short('s')
                .long(options::SYMBOLIC_LINK)
                .overrides_with_all(MODE_ARGS)
                .help(translate!("cp-help-symbolic-link"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORCE)
                .short('f')
                .long(options::FORCE)
                .help(translate!("cp-help-force"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REMOVE_DESTINATION)
                .long(options::REMOVE_DESTINATION)
                .overrides_with(options::FORCE)
                .help(translate!("cp-help-remove-destination"))
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
                .help(translate!("cp-help-reflink")),
        )
        .arg(
            Arg::new(options::ATTRIBUTES_ONLY)
                .long(options::ATTRIBUTES_ONLY)
                .overrides_with_all(MODE_ARGS)
                .help(translate!("cp-help-attributes-only"))
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
                .help(translate!("cp-help-preserve")),
        )
        .arg(
            Arg::new(options::PRESERVE_DEFAULT_ATTRIBUTES)
                .short('p')
                .long(options::PRESERVE_DEFAULT_ATTRIBUTES)
                .help(translate!("cp-help-preserve-default"))
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
                .help(translate!("cp-help-no-preserve")),
        )
        .arg(
            Arg::new(options::PARENTS)
                .long(options::PARENTS)
                .alias(options::PARENT)
                .help(translate!("cp-help-parents"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE)
                .short('P')
                .long(options::NO_DEREFERENCE)
                .overrides_with(options::DEREFERENCE)
                // -d sets this option
                .help(translate!("cp-help-no-dereference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .overrides_with(options::NO_DEREFERENCE)
                .help(translate!("cp-help-dereference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CLI_SYMBOLIC_LINKS)
                .short('H')
                .help(translate!("cp-help-cli-symbolic-links"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARCHIVE)
                .short('a')
                .long(options::ARCHIVE)
                .help(translate!("cp-help-archive"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE_PRESERVE_LINKS)
                .short('d')
                .help(translate!("cp-help-no-dereference-preserve-links"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONE_FILE_SYSTEM)
                .short('x')
                .long(options::ONE_FILE_SYSTEM)
                .help(translate!("cp-help-one-file-system"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SPARSE)
                .long(options::SPARSE)
                .value_name("WHEN")
                .value_parser(ShortcutValueParser::new(["never", "auto", "always"]))
                .help(translate!("cp-help-sparse")),
        )
        .arg(
            Arg::new(options::SELINUX)
                .short('Z')
                .help(translate!("cp-help-selinux"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .value_parser(value_parser!(String))
                .help(translate!("cp-help-context"))
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value(""),
        )
        .arg(
            // The 'g' short flag is modeled after advcpmv
            // See this repo: https://github.com/jarun/advcpmv
            Arg::new(options::PROGRESS_BAR)
                .long(options::PROGRESS_BAR)
                .short('g')
                .action(ArgAction::SetTrue)
                .help(translate!("cp-help-progress")),
        )
        // TODO: implement the following args
        .arg(
            Arg::new(options::COPY_CONTENTS)
                .long(options::COPY_CONTENTS)
                .overrides_with(options::ATTRIBUTES_ONLY)
                .help(translate!("cp-help-copy-contents"))
                .action(ArgAction::SetTrue),
        )
        // END TODO
        .arg(
            Arg::new(options::PATHS)
                .action(ArgAction::Append)
                .num_args(1..)
                .required(true)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(ValueParser::os_string()),
        )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let options = Options::from_matches(&matches)?;

    if options.overwrite == OverwriteMode::NoClobber && options.backup != BackupMode::None {
        return Err(UUsageError::new(
            EXIT_ERR,
            translate!("cp-error-backup-mutually-exclusive"),
        ));
    }

    let paths: Vec<PathBuf> = matches
        .get_many::<OsString>(options::PATHS)
        .map(|v| v.map(PathBuf::from).collect())
        .unwrap_or_default();

    let (sources, target) = parse_path_args(paths, &options)?;

    if let Err(error) = copy(&sources, &target, &options) {
        match error {
            // Error::NotAllFilesCopied is non-fatal, but the error
            // code should still be EXIT_ERR as does GNU cp
            CpError::NotAllFilesCopied => {}
            // Else we caught a fatal bubbled-up error, log it to stderr
            _ => show_error!("{error}"),
        }
        set_exit_code(EXIT_ERR);
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

    /// Set the field to `Preserve::No { explicit: true }` if the corresponding field
    /// in other is set to `Preserve::Yes { .. }`.
    pub fn diff(self, other: &Self) -> Self {
        fn update_preserve_field(current: Preserve, other: Preserve) -> Preserve {
            if matches!(other, Preserve::Yes { .. }) {
                Preserve::No { explicit: true }
            } else {
                current
            }
        }
        Self {
            #[cfg(unix)]
            ownership: update_preserve_field(self.ownership, other.ownership),
            mode: update_preserve_field(self.mode, other.mode),
            timestamps: update_preserve_field(self.timestamps, other.timestamps),
            context: update_preserve_field(self.context, other.context),
            links: update_preserve_field(self.links, other.links),
            xattr: update_preserve_field(self.xattr, other.xattr),
        }
    }

    pub fn parse_iter<T>(values: impl Iterator<Item = T>) -> CopyResult<Self>
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
    fn parse_single_string(value: &str) -> CopyResult<Self> {
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
                return Err(CpError::InvalidArgument(
                    translate!("cp-error-invalid-attribute", "value" => value.quote()),
                ));
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
        ];

        for not_implemented_opt in not_implemented_opts {
            if matches.contains_id(not_implemented_opt)
                && matches.value_source(not_implemented_opt)
                    == Some(clap::parser::ValueSource::CommandLine)
            {
                return Err(CpError::NotImplemented(not_implemented_opt.to_string()));
            }
        }

        let recursive = matches.get_flag(options::RECURSIVE) || matches.get_flag(options::ARCHIVE);

        let backup_mode = match backup_control::determine_backup_mode(matches) {
            Err(e) => return Err(CpError::Backup(BackupError(format!("{e}")))),
            Ok(mode) => mode,
        };
        let update_mode = update_control::determine_update_mode(matches);

        if backup_mode != BackupMode::None
            && matches
                .get_one::<String>(update_control::arguments::OPT_UPDATE)
                .is_some_and(|v| v == "none" || v == "none-fail")
        {
            return Err(CpError::InvalidArgument(
                translate!("cp-error-invalid-backup-argument").to_string(),
            ));
        }

        let backup_suffix = backup_control::determine_backup_suffix(matches);

        let overwrite = OverwriteMode::from_matches(matches);

        // Parse target directory options
        let no_target_dir = matches.get_flag(options::NO_TARGET_DIRECTORY);
        let target_dir = matches
            .get_one::<PathBuf>(options::TARGET_DIRECTORY)
            .cloned();

        if let Some(dir) = &target_dir {
            if !dir.is_dir() {
                return Err(CpError::NotADirectory(dir.clone()));
            }
        }
        // cp follows POSIX conventions for overriding options such as "-a",
        // "-d", "--preserve", and "--no-preserve". We can use clap's
        // override-all behavior to achieve this, but there's a challenge: when
        // clap overrides an argument, it removes all traces of it from the
        // match. This poses a problem because flags like "-a" expand to "-dR
        // --preserve=all", and we only want to override the "--preserve=all"
        // part. Additionally, we need to handle multiple occurrences of the
        // same flags. To address this, we create an overriding order from the
        // matches here.
        let mut overriding_order: Vec<(usize, &str, Vec<&String>)> = vec![];
        // We iterate through each overriding option, adding each occurrence of
        // the option along with its value and index as a tuple, and push it to
        // `overriding_order`.
        for option in [
            options::PRESERVE,
            options::NO_PRESERVE,
            options::ARCHIVE,
            options::PRESERVE_DEFAULT_ATTRIBUTES,
            options::NO_DEREFERENCE_PRESERVE_LINKS,
        ] {
            if let (Ok(Some(val)), Some(index)) = (
                matches.try_get_one::<bool>(option),
                // even though it says in the doc that `index_of` would give us
                // the first index of the argument, when it comes to flag it
                // gives us the last index where the flag appeared (probably
                // because it overrides itself). Since it is a flag and it would
                // have same value across the occurrences we just need the last
                // index.
                matches.index_of(option),
            ) {
                if *val {
                    overriding_order.push((index, option, vec![]));
                }
            } else if let (Some(occurrences), Some(mut indices)) = (
                matches.get_occurrences::<String>(option),
                matches.indices_of(option),
            ) {
                occurrences.for_each(|val| {
                    if let Some(index) = indices.next() {
                        let val = val.collect::<Vec<&String>>();
                        // As mentioned in the documentation of the indices_of
                        // function, it provides the indices of the individual
                        // values. Therefore, to get the index of the first
                        // value of the next occurrence in the next iteration,
                        // we need to advance the indices iterator by the length
                        // of the current occurrence's values.
                        for _ in 1..val.len() {
                            indices.next();
                        }
                        overriding_order.push((index, option, val));
                    }
                });
            }
        }
        overriding_order.sort_by(|a, b| a.0.cmp(&b.0));

        let mut attributes = Attributes::NONE;

        // Iterate through the `overriding_order` and adjust the attributes accordingly.
        for (_, option, val) in overriding_order {
            match option {
                options::ARCHIVE => {
                    attributes = Attributes::ALL;
                }
                options::PRESERVE_DEFAULT_ATTRIBUTES => {
                    attributes = attributes.union(&Attributes::DEFAULT);
                }
                options::NO_DEREFERENCE_PRESERVE_LINKS => {
                    attributes = attributes.union(&Attributes::LINKS);
                }
                options::PRESERVE => {
                    attributes = attributes.union(&Attributes::parse_iter(val.into_iter())?);
                }
                options::NO_PRESERVE => {
                    if !val.is_empty() {
                        attributes = attributes.diff(&Attributes::parse_iter(val.into_iter())?);
                    }
                }
                _ => (),
            }
        }

        #[cfg(not(feature = "selinux"))]
        if let Preserve::Yes { required } = attributes.context {
            let selinux_disabled_error = CpError::Error(translate!("cp-error-selinux-not-enabled"));
            if required {
                return Err(selinux_disabled_error);
            }
            show_error_if_needed(&selinux_disabled_error);
        }

        // Extract the SELinux related flags and options
        let set_selinux_context = matches.get_flag(options::SELINUX);

        let context = if matches.contains_id(options::CONTEXT) {
            matches.get_one::<String>(options::CONTEXT).cloned()
        } else {
            None
        };

        let options = Self {
            attributes_only: matches.get_flag(options::ATTRIBUTES_ONLY),
            copy_contents: matches.get_flag(options::COPY_CONTENTS),
            cli_dereference: matches.get_flag(options::CLI_SYMBOLIC_LINKS),
            copy_mode: CopyMode::from_matches(matches),
            // No dereference is set with -p, -d and --archive
            dereference: !(matches.get_flag(options::NO_DEREFERENCE)
                || matches.get_flag(options::NO_DEREFERENCE_PRESERVE_LINKS)
                || matches.get_flag(options::ARCHIVE)
                // cp normally follows the link only when not copying recursively or when
                // --link (-l) is used
                || (recursive && CopyMode::from_matches(matches)!= CopyMode::Link ))
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
                            return Err(CpError::InvalidArgument(
                                translate!("cp-error-invalid-argument", "arg" => value.quote(), "option" => "reflink"),
                            ));
                        }
                    }
                } else {
                    ReflinkMode::default()
                }
            },
            sparse_mode: {
                if let Some(val) = matches.get_one::<String>(options::SPARSE) {
                    match val.as_str() {
                        "always" => SparseMode::Always,
                        "auto" => SparseMode::Auto,
                        "never" => SparseMode::Never,
                        _ => {
                            return Err(CpError::InvalidArgument(
                                translate!("cp-error-invalid-argument", "arg" => val, "option" => "sparse"),
                            ));
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
            set_selinux_context: set_selinux_context || context.is_some(),
            context,
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
    /// Return [`TargetType`] required for `target`.
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
        return Err(translate!("cp-error-missing-file-operand").into());
    } else if paths.len() == 1 && options.target_dir.is_none() {
        // Only one file specified
        return Err(translate!("cp-error-missing-destination-operand",
                       "source" => paths[0].quote())
        .into());
    }

    // Return an error if the user requested to copy more than one
    // file source to a file target
    if options.no_target_dir && options.target_dir.is_none() && paths.len() > 2 {
        return Err(translate!("cp-error-extra-operand",
                              "operand" => paths[2].quote())
        .into());
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
        // clippy::assigning_clones added with Rust 1.78
        // Rust version = 1.76 on OpenBSD stable/7.5
        #[cfg_attr(not(target_os = "openbsd"), allow(clippy::assigning_clones))]
        for source in &mut paths {
            *source = source.components().as_path().to_owned();
        }
    }

    Ok((paths, target))
}

/// When handling errors, we don't always want to show them to the user. This function handles that.
fn show_error_if_needed(error: &CpError) {
    match error {
        // When using --no-clobber, we don't want to show
        // an error message
        CpError::NotAllFilesCopied => {
            // Need to return an error code
        }
        CpError::Skipped(_) => {
            // touch a b && echo "n"|cp -i a b && echo $?
            // should return an error from GNU 9.2
        }
        _ => {
            show_error!("{error}");
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
    let mut created_parent_dirs: HashSet<PathBuf> = HashSet::new();

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
        let normalized_source = normalize_path(source);
        if options.backup == BackupMode::None && seen_sources.contains(&normalized_source) {
            let file_type = if source.symlink_metadata()?.file_type().is_dir() {
                "directory"
            } else {
                "file"
            };
            let msg = translate!("cp-warning-source-specified-more-than-once", "file_type" => file_type, "source" => source.quote());
            show_warning!("{msg}");
        } else {
            let dest = construct_dest_path(source, target, target_type, options)
                .unwrap_or_else(|_| target.to_path_buf());

            if fs::metadata(&dest).is_ok()
                && !fs::symlink_metadata(&dest)?.file_type().is_symlink()
                // if both `source` and `dest` are symlinks, it should be considered as an overwrite.
                || fs::metadata(source).is_ok()
                    && fs::symlink_metadata(source)?.file_type().is_symlink()
                || matches!(options.copy_mode, CopyMode::SymLink)
            {
                // There is already a file and it isn't a symlink (managed in a different place)
                if copied_destinations.contains(&dest) && options.backup != BackupMode::Numbered {
                    // If the target was already created in this cp call, check if it's a directory.
                    // Directories should be merged (GNU cp behavior), but files should not be overwritten.
                    let dest_is_dir = fs::metadata(&dest).is_ok_and(|m| m.is_dir());
                    let source_is_dir = fs::metadata(source).is_ok_and(|m| m.is_dir());

                    // Only prevent overwriting if both source and dest are files (not directories)
                    // Directories should be merged, which is handled by copy_directory
                    if !dest_is_dir || !source_is_dir {
                        // If the target file was already created in this cp call, do not overwrite
                        return Err(CpError::Error(
                            translate!("cp-error-will-not-overwrite-just-created", "dest" => dest.quote(), "source" => source.quote()),
                        ));
                    }
                }
            }

            if let Err(error) = copy_source(
                progress_bar.as_ref(),
                source,
                target,
                target_type,
                options,
                &mut symlinked_files,
                &copied_destinations,
                &mut copied_files,
                &mut created_parent_dirs,
            ) {
                show_error_if_needed(&error);
                if !matches!(error, CpError::Skipped(false)) {
                    non_fatal_errors = true;
                }
            } else {
                copied_destinations.insert(dest.clone());
            }
        }
        seen_sources.insert(normalized_source);
    }

    if let Some(pb) = progress_bar {
        pb.finish();
    }

    if non_fatal_errors {
        Err(CpError::NotAllFilesCopied)
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
        return Err(
            translate!("cp-error-cannot-overwrite-directory-with-non-directory",
                              "dir" => target.quote())
            .into(),
        );
    }

    if options.parents && !target.is_dir() {
        return Err(translate!("cp-error-with-parents-dest-must-be-dir").into());
    }

    Ok(match target_type {
        TargetType::Directory => {
            let root = if options.parents {
                if source_path.has_root() && cfg!(unix) {
                    Path::new("/")
                } else {
                    Path::new("")
                }
            } else {
                if source_path == Path::new(".") && target.is_dir() {
                    // Special case: when copying current directory (.) to an existing directory,
                    // return the target path directly instead of trying to construct a path
                    // relative to the source's parent. This ensures we copy the contents of
                    // the current directory into the target directory, not create a subdirectory.
                    return Ok(target.to_path_buf());
                }
                source_path.parent().unwrap_or(source_path)
            };
            localize_to_target(root, source_path, target)?
        }
        TargetType::File => target.to_path_buf(),
    })
}
#[allow(clippy::too_many_arguments)]
fn copy_source(
    progress_bar: Option<&ProgressBar>,
    source: &Path,
    target: &Path,
    target_type: TargetType,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    copied_destinations: &HashSet<PathBuf>,
    copied_files: &mut HashMap<FileInformation, PathBuf>,
    created_parent_dirs: &mut HashSet<PathBuf>,
) -> CopyResult<()> {
    let source_path = Path::new(&source);
    if source_path.is_dir() && (options.dereference || !source_path.is_symlink()) {
        // Copy as directory
        copy_directory(
            progress_bar,
            source,
            target,
            options,
            symlinked_files,
            copied_destinations,
            copied_files,
            created_parent_dirs,
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
            copied_destinations,
            copied_files,
            created_parent_dirs,
            true,
        );
        if options.parents {
            for (x, y) in aligned_ancestors(source, dest.as_path()) {
                if let Ok(src) = canonicalize(x, MissingHandling::Normal, ResolveMode::Physical) {
                    copy_attributes(&src, y, &options.attributes)?;
                }
            }
        }
        res
    }
}

/// If `path` does not have `S_IWUSR` set, returns a tuple of the file's
/// mode in octal (index 0) and human-readable (index 1) formats.
///
/// If the destination of a copy operation is a file that is not writeable to
/// the owner (bit `S_IWUSR`), extra information needs to be added to the
/// interactive mode prompt: the mode (permissions) of the file in octal and
/// human-readable format.
// TODO
// The destination metadata can be read multiple times in the course of a single execution of `cp`.
// This fix adds yet another metadata read.
// Should this metadata be read once and then reused throughout the execution?
// https://github.com/uutils/coreutils/issues/6658
#[allow(clippy::if_not_else)]
fn file_mode_for_interactive_overwrite(
    #[cfg_attr(not(unix), allow(unused_variables))] path: &Path,
) -> Option<(String, String)> {
    // Retain outer braces to ensure only one branch is included
    {
        #[cfg(unix)]
        {
            use libc::{S_IWUSR, mode_t};
            use std::os::unix::prelude::MetadataExt;

            match path.metadata() {
                Ok(me) => {
                    // Cast is necessary on some platforms
                    let mode: mode_t = me.mode() as mode_t;

                    // It looks like this extra information is added to the prompt iff the file's user write bit is 0
                    //  write permission, owner
                    if uucore::has!(mode, S_IWUSR) {
                        None
                    } else {
                        // Discard leading digits
                        let mode_without_leading_digits = mode & 0o7777;

                        Some((
                            format!("{mode_without_leading_digits:04o}"),
                            uucore::fs::display_permissions_unix(mode, false),
                        ))
                    }
                }
                // TODO: How should failure to read the metadata be handled? Ignoring for now.
                Err(_) => None,
            }
        }

        #[cfg(not(unix))]
        {
            None
        }
    }
}

impl OverwriteMode {
    fn verify(&self, path: &Path, debug: bool) -> CopyResult<()> {
        match *self {
            Self::NoClobber => {
                if debug {
                    println!("{}", translate!("cp-debug-skipped", "path" => path.quote()));
                }
                Err(CpError::Skipped(false))
            }
            Self::Interactive(_) => {
                let prompt_yes_result = if let Some((octal, human_readable)) =
                    file_mode_for_interactive_overwrite(path)
                {
                    let prompt_msg =
                        translate!("cp-prompt-overwrite-with-mode", "path" => path.quote());
                    prompt_yes!("{prompt_msg} {octal} ({human_readable})?")
                } else {
                    let prompt_msg = translate!("cp-prompt-overwrite", "path" => path.quote());
                    prompt_yes!("{prompt_msg}")
                };

                if prompt_yes_result {
                    Ok(())
                } else {
                    Err(CpError::Skipped(true))
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
    }
    Ok(())
}

/// Copies extended attributes (xattrs) from `source` to `dest`, ensuring that `dest` is temporarily
/// user-writable if needed and restoring its original permissions afterward. This avoids "Operation
/// not permitted" errors on read-only files. Returns an error if permission or metadata operations fail,
/// or if xattr copying fails.
#[cfg(all(unix, not(target_os = "android")))]
fn copy_extended_attrs(source: &Path, dest: &Path) -> CopyResult<()> {
    let metadata = fs::symlink_metadata(dest)?;

    // Check if the destination file is currently read-only for the user.
    let mut perms = metadata.permissions();
    let was_readonly = perms.readonly();

    // Temporarily grant user write if it was read-only.
    if was_readonly {
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(dest, perms)?;
    }

    // Perform the xattr copy and capture any potential error,
    // so we can restore permissions before returning.
    let copy_xattrs_result = copy_xattrs(source, dest);

    // Restore read-only if we changed it.
    if was_readonly {
        let mut revert_perms = fs::symlink_metadata(dest)?.permissions();
        revert_perms.set_readonly(true);
        fs::set_permissions(dest, revert_perms)?;
    }

    // If copying xattrs failed, propagate that error now.
    copy_xattrs_result?;

    Ok(())
}

/// Copy the specified attributes from one path to another.
pub(crate) fn copy_attributes(
    source: &Path,
    dest: &Path,
    attributes: &Attributes,
) -> CopyResult<()> {
    let context = &*format!("{} -> {}", source.quote(), dest.quote());
    let source_metadata =
        fs::symlink_metadata(source).map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

    // Ownership must be changed first to avoid interfering with mode change.
    #[cfg(unix)]
    handle_preserve(&attributes.ownership, || -> CopyResult<()> {
        use std::os::unix::prelude::MetadataExt;
        use uucore::perms::Verbosity;
        use uucore::perms::VerbosityLevel;
        use uucore::perms::wrap_chown;

        let dest_uid = source_metadata.uid();
        let dest_gid = source_metadata.gid();
        let meta = &dest
            .symlink_metadata()
            .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;

        let try_chown = {
            |uid| {
                wrap_chown(
                    dest,
                    meta,
                    uid,
                    Some(dest_gid),
                    false,
                    Verbosity {
                        groups_only: false,
                        level: VerbosityLevel::Silent,
                    },
                )
            }
        };
        // gnu compatibility: cp doesn't report an error if it fails to set the ownership,
        // and will fall back to changing only the gid if possible.
        if try_chown(Some(dest_uid)).is_err() {
            let _ = try_chown(None);
        }
        Ok(())
    })?;

    handle_preserve(&attributes.mode, || -> CopyResult<()> {
        // The `chmod()` system call that underlies the
        // `fs::set_permissions()` call is unable to change the
        // permissions of a symbolic link. In that case, we just
        // do nothing, since every symbolic link has the same
        // permissions.
        if !dest.is_symlink() {
            fs::set_permissions(dest, source_metadata.permissions())
                .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?;
            // FIXME: Implement this for windows as well
            #[cfg(feature = "feat_acl")]
            exacl::getfacl(source, None)
                .and_then(|acl| exacl::setfacl(&[dest], &acl, None))
                .map_err(|err| CpError::Error(err.to_string()))?;
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

    #[cfg(feature = "selinux")]
    handle_preserve(&attributes.context, || -> CopyResult<()> {
        // Get the source context and apply it to the destination
        if let Ok(context) = selinux::SecurityContext::of_path(source, false, false) {
            if let Some(context) = context {
                if let Err(e) = context.set_for_path(dest, false, false) {
                    return Err(CpError::Error(
                        translate!("cp-error-selinux-set-context", "path" => dest.quote(), "error" => e),
                    ));
                }
            }
        } else {
            return Err(CpError::Error(
                translate!("cp-error-selinux-get-context", "path" => source.quote()),
            ));
        }
        Ok(())
    })?;

    handle_preserve(&attributes.xattr, || -> CopyResult<()> {
        #[cfg(all(unix, not(target_os = "android")))]
        {
            copy_extended_attrs(source, dest)?;
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
        std::os::unix::fs::symlink(source, dest).map_err(|e| {
            CpError::IoErrContext(
                e,
                translate!("cp-error-cannot-create-symlink",
                           "dest" => get_filename(dest).unwrap_or("?").quote(),
                           "source" => get_filename(source).unwrap_or("?").quote()),
            )
        })?;
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(source, dest).map_err(|e| {
            CpError::IoErrContext(
                e,
                translate!("cp-error-cannot-create-symlink",
                           "dest" => get_filename(dest).unwrap_or("?").quote(),
                           "source" => get_filename(source).unwrap_or("?").quote()),
            )
        })?;
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
/// if `is_dest_symlink` flag is set to true dest will be renamed to `backup_path`
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
    if options.backup != BackupMode::None {
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
    // If source and dest are both the same symlink but with different names, then allow the copy.
    // This can occur, for example, if source and dest are both hardlinks to the same symlink.
    if dest_is_symlink
        && source_is_symlink
        && source.file_name() != dest.file_name()
        && !options.dereference
    {
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
    copied_files: &HashMap<FileInformation, PathBuf>,
) -> CopyResult<()> {
    // Disallow copying a file to itself, unless `--force` and
    // `--backup` are both specified.
    if is_forbidden_to_copy_to_same_file(source, dest, options, source_in_command_line) {
        return Err(translate!("cp-error-same-file",
                       "source" => source.quote(),
                       "dest" => dest.quote())
        .into());
    }

    if options.update == UpdateMode::None {
        if options.debug {
            println!("skipped {}", dest.quote());
        }
        return Err(CpError::Skipped(false));
    }

    if options.update != UpdateMode::IfOlder {
        options.overwrite.verify(dest, options.debug)?;
    }

    let mut is_dest_removed = false;
    let backup_path = backup_control::get_backup_path(options.backup, dest, &options.backup_suffix);
    if let Some(backup_path) = backup_path {
        if paths_refer_to_same_file(source, &backup_path, true) {
            return Err(translate!("cp-error-backing-up-destroy-source", "dest" => dest.quote(), "source" => source.quote())
            .into());
        }
        is_dest_removed = dest.is_symlink();
        backup_dest(dest, &backup_path, is_dest_removed)?;
    }
    if !is_dest_removed {
        delete_dest_if_needed_and_allowed(
            source,
            dest,
            options,
            source_in_command_line,
            copied_files,
        )?;
    }

    Ok(())
}

/// Checks if:
/// * `dest` needs to be deleted before the copy operation can proceed
/// * the provided options allow this deletion
///
/// If so, deletes `dest`.
fn delete_dest_if_needed_and_allowed(
    source: &Path,
    dest: &Path,
    options: &Options,
    source_in_command_line: bool,
    copied_files: &HashMap<FileInformation, PathBuf>,
) -> CopyResult<()> {
    let delete_dest = match options.overwrite {
        OverwriteMode::Clobber(cl) | OverwriteMode::Interactive(cl) => {
            match cl {
                ClobberMode::Force => {
                    // TODO
                    // Using `readonly` here to check if `dest` needs to be deleted is not correct:
                    // "On Unix-based platforms this checks if any of the owner, group or others write permission bits are set. It does not check if the current user is in the file's assigned group. It also does not check ACLs. Therefore the return value of this function cannot be relied upon to predict whether attempts to read or write the file will actually succeed."
                    // This results in some copy operations failing, because this necessary deletion is being skipped.
                    is_symlink_loop(dest) || fs::metadata(dest)?.permissions().readonly()
                }
                ClobberMode::RemoveDestination => true,
                ClobberMode::Standard => {
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
                    options.preserve_hard_links() &&
                            // only try to remove dest file only if the current source
                            // is hardlink to a file that is already copied
                            copied_files.contains_key(
                                &FileInformation::from_path(
                                    source,
                                    options.dereference(source_in_command_line)
                                ).map_err(|e| CpError::IoErrContext(e, format!("cannot stat {}", source.quote())))?
                            )
                }
            }
        }
        OverwriteMode::NoClobber => false,
    };

    if delete_dest {
        delete_path(dest, options)
    } else {
        Ok(())
    }
}

fn delete_path(path: &Path, options: &Options) -> CopyResult<()> {
    // Windows requires clearing readonly attribute before deletion when using --force
    #[cfg(windows)]
    if options.force() {
        if let Ok(mut perms) = fs::metadata(path).map(|m| m.permissions()) {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = fs::set_permissions(path, perms);
        }
    }

    match fs::remove_file(path) {
        Ok(()) => {
            if options.verbose {
                println!(
                    "{}",
                    translate!("cp-verbose-removed", "path" => path.quote())
                );
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            // target could have been deleted earlier (e.g. same-file with --remove-destination)
        }
        Err(err) => return Err(err.into()),
    }

    Ok(())
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
    progress_bar: Option<&ProgressBar>,
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
            println!(
                "{}",
                translate!("cp-verbose-created-directory", "source" => x.display(), "dest" => y.display())
            );
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
#[allow(clippy::too_many_arguments)]
fn handle_copy_mode(
    source: &Path,
    dest: &Path,
    options: &Options,
    context: &str,
    source_metadata: &Metadata,
    symlinked_files: &mut HashSet<FileInformation>,
    source_in_command_line: bool,
    source_is_fifo: bool,
    source_is_socket: bool,
    created_parent_dirs: &mut HashSet<PathBuf>,
    #[cfg(unix)] source_is_stream: bool,
) -> CopyResult<PerformedAction> {
    let source_is_symlink = source_metadata.is_symlink();

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
            .map_err(|e| {
                CpError::IoErrContext(
                    e,
                    translate!("cp-error-cannot-create-hard-link", "dest" => get_filename(dest).unwrap_or("?").quote(), "source" => get_filename(source).unwrap_or("?").quote())
                )
            })?;
        }
        CopyMode::Copy => {
            copy_helper(
                source,
                dest,
                options,
                context,
                source_is_symlink,
                source_is_fifo,
                source_is_socket,
                symlinked_files,
                created_parent_dirs,
                #[cfg(unix)]
                source_is_stream,
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
                    UpdateMode::All => {
                        copy_helper(
                            source,
                            dest,
                            options,
                            context,
                            source_is_symlink,
                            source_is_fifo,
                            source_is_socket,
                            symlinked_files,
                            created_parent_dirs,
                            #[cfg(unix)]
                            source_is_stream,
                        )?;
                    }
                    UpdateMode::None => {
                        if options.debug {
                            println!("skipped {}", dest.quote());
                        }

                        return Ok(PerformedAction::Skipped);
                    }
                    UpdateMode::NoneFail => {
                        return Err(CpError::Error(
                            translate!("cp-error-not-replacing", "file" => dest.quote()),
                        ));
                    }
                    UpdateMode::IfOlder => {
                        let dest_metadata = fs::symlink_metadata(dest)?;

                        let src_time = source_metadata.modified()?;
                        let dest_time = dest_metadata.modified()?;
                        if src_time <= dest_time {
                            return Ok(PerformedAction::Skipped);
                        }

                        options.overwrite.verify(dest, options.debug)?;

                        copy_helper(
                            source,
                            dest,
                            options,
                            context,
                            source_is_symlink,
                            source_is_fifo,
                            source_is_socket,
                            symlinked_files,
                            created_parent_dirs,
                            #[cfg(unix)]
                            source_is_stream,
                        )?;
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
                    source_is_socket,
                    symlinked_files,
                    created_parent_dirs,
                    #[cfg(unix)]
                    source_is_stream,
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
    }

    Ok(PerformedAction::Copied)
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
///
// Allow unused variables for Windows (on options)
#[allow(unused_variables)]
fn calculate_dest_permissions(
    dest_metadata: Option<&Metadata>,
    dest: &Path,
    source_metadata: &Metadata,
    options: &Options,
    context: &str,
) -> CopyResult<Permissions> {
    if let Some(metadata) = dest_metadata {
        Ok(metadata.permissions())
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
#[allow(clippy::cognitive_complexity, clippy::too_many_arguments)]
fn copy_file(
    progress_bar: Option<&ProgressBar>,
    source: &Path,
    dest: &Path,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    copied_destinations: &HashSet<PathBuf>,
    copied_files: &mut HashMap<FileInformation, PathBuf>,
    created_parent_dirs: &mut HashSet<PathBuf>,
    source_in_command_line: bool,
) -> CopyResult<()> {
    let source_is_symlink = source.is_symlink();
    let initial_dest_metadata = dest.symlink_metadata().ok();
    let dest_is_symlink = initial_dest_metadata
        .as_ref()
        .is_some_and(|md| md.file_type().is_symlink());
    let dest_target_exists = dest.try_exists().unwrap_or(false);
    // Fail if dest is a dangling symlink or a symlink this program created previously
    if dest_is_symlink {
        if FileInformation::from_path(dest, false)
            .map(|info| symlinked_files.contains(&info))
            .unwrap_or(false)
        {
            return Err(CpError::Error(
                translate!("cp-error-will-not-copy-through-symlink", "source" => source.quote(), "dest" => dest.quote()),
            ));
        }
        // Fail if cp tries to copy two sources of the same name into a single symlink
        // Example: "cp file1 dir1/file1 tmp" where "tmp" is a directory containing a symlink "file1" pointing to a file named "foo".
        // foo will contain the contents of "file1" and "dir1/file1" will not be copied over to "tmp/file1"
        if copied_destinations.contains(dest) {
            return Err(CpError::Error(
                translate!("cp-error-will-not-copy-through-symlink", "source" => source.quote(), "dest" => dest.quote()),
            ));
        }

        let copy_contents = options.dereference(source_in_command_line) || !source_is_symlink;
        if copy_contents
            && !dest_target_exists
            && !matches!(
                options.overwrite,
                OverwriteMode::Clobber(ClobberMode::RemoveDestination)
            )
            && !is_symlink_loop(dest)
            && std::env::var_os("POSIXLY_CORRECT").is_none()
        {
            return Err(CpError::Error(
                translate!("cp-error-not-writing-dangling-symlink", "dest" => dest.quote()),
            ));
        }
        if paths_refer_to_same_file(source, dest, true)
            && matches!(
                options.overwrite,
                OverwriteMode::Clobber(ClobberMode::RemoveDestination)
            )
            && options.backup == BackupMode::None
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

    if initial_dest_metadata.is_some()
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
            } else if options.backup != BackupMode::None && !dest_is_symlink {
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
            if options.copy_mode == CopyMode::Copy {
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
        return Err(translate!("cp-error-cannot-change-attribute", "dest" => dest.quote()).into());
    }

    if options.preserve_hard_links() {
        // if we encounter a matching device/inode pair in the source tree
        // we can arrange to create a hard link between the corresponding names
        // in the destination tree.
        if let Some(new_source) = copied_files.get(
            &FileInformation::from_path(source, options.dereference(source_in_command_line))
                .map_err(|e| CpError::IoErrContext(e, format!("cannot stat {}", source.quote())))?,
        ) {
            fs::hard_link(new_source, dest)?;

            if options.verbose {
                print_verbose_output(options.parents, progress_bar, source, dest);
            }

            return Ok(());
        }
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
        // this is just for gnu tests compatibility
        result.map_err(|err| {
            if err.to_string().contains("No such file or directory") {
                return translate!("cp-error-cannot-stat", "source" => source.quote());
            }
            err.to_string()
        })?
    };

    let dest_metadata = dest.symlink_metadata().ok();

    let dest_permissions = calculate_dest_permissions(
        dest_metadata.as_ref(),
        dest,
        &source_metadata,
        options,
        context,
    )?;

    #[cfg(unix)]
    let source_is_fifo = source_metadata.file_type().is_fifo();
    #[cfg(unix)]
    let source_is_socket = source_metadata.file_type().is_socket();
    #[cfg(not(unix))]
    let source_is_fifo = false;
    #[cfg(not(unix))]
    let source_is_socket = false;

    let source_is_stream = is_stream(&source_metadata);

    let performed_action = handle_copy_mode(
        source,
        dest,
        options,
        context,
        &source_metadata,
        symlinked_files,
        source_in_command_line,
        source_is_fifo,
        source_is_socket,
        created_parent_dirs,
        #[cfg(unix)]
        source_is_stream,
    )?;

    if options.verbose && performed_action != PerformedAction::Skipped {
        print_verbose_output(options.parents, progress_bar, source, dest);
    }

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

    if options.dereference(source_in_command_line) {
        if let Ok(src) = canonicalize(source, MissingHandling::Normal, ResolveMode::Physical) {
            if src.exists() {
                copy_attributes(&src, dest, &options.attributes)?;
            }
        }
    } else if source_is_stream && !source.exists() {
        // Some stream files may not exist after we have copied it,
        // like anonymous pipes. Thus, we can't really copy its
        // attributes. However, this is already handled in the stream
        // copy function (see `copy_stream` under platform/linux.rs).
    } else {
        copy_attributes(source, dest, &options.attributes)?;
    }

    #[cfg(feature = "selinux")]
    if options.set_selinux_context && uucore::selinux::is_selinux_enabled() {
        // Set the given selinux permissions on the copied file.
        if let Err(e) =
            uucore::selinux::set_selinux_security_context(dest, options.context.as_ref())
        {
            return Err(CpError::Error(
                translate!("cp-error-selinux-error", "error" => e),
            ));
        }
    }

    copied_files.insert(
        FileInformation::from_path(source, options.dereference(source_in_command_line))?,
        dest.to_path_buf(),
    );

    if let Some(progress_bar) = progress_bar {
        progress_bar.inc(source_metadata.len());
    }

    Ok(())
}

fn is_stream(metadata: &Metadata) -> bool {
    #[cfg(unix)]
    {
        let file_type = metadata.file_type();
        file_type.is_fifo() || file_type.is_char_device() || file_type.is_block_device()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        false
    }
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
            target_os = "freebsd",
            target_os = "redox",
        )))]
        {
            const MODE_RW_UGO: u32 = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;
            const S_IRWXUGO: u32 = S_IRWXU | S_IRWXG | S_IRWXO;
            return if is_explicit_no_preserve_mode {
                MODE_RW_UGO
            } else {
                org_mode & S_IRWXUGO
            };
        }

        #[cfg(any(
            target_os = "android",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "redox",
        ))]
        {
            const MODE_RW_UGO: u32 =
                (S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH) as u32;
            const S_IRWXUGO: u32 = (S_IRWXU | S_IRWXG | S_IRWXO) as u32;
            return if is_explicit_no_preserve_mode {
                MODE_RW_UGO
            } else {
                org_mode & S_IRWXUGO
            };
        }
    }

    org_mode
}

/// Copy the file from `source` to `dest` either using the normal `fs::copy` or a
/// copy-on-write scheme if --reflink is specified and the filesystem supports it.
#[allow(clippy::too_many_arguments)]
fn copy_helper(
    source: &Path,
    dest: &Path,
    options: &Options,
    context: &str,
    source_is_symlink: bool,
    source_is_fifo: bool,
    source_is_socket: bool,
    symlinked_files: &mut HashSet<FileInformation>,
    created_parent_dirs: &mut HashSet<PathBuf>,
    #[cfg(unix)] source_is_stream: bool,
) -> CopyResult<()> {
    if options.parents {
        let parent = dest.parent().unwrap_or(dest);
        if created_parent_dirs.insert(parent.to_path_buf()) {
            fs::create_dir_all(parent)?;
        }
    }

    if path_ends_with_terminator(dest) && !dest.is_dir() {
        return Err(CpError::NotADirectory(dest.to_path_buf()));
    }

    if source_is_socket && options.recursive && !options.copy_contents {
        #[cfg(unix)]
        copy_socket(dest, options.overwrite, options.debug)?;
    } else if source_is_fifo && options.recursive && !options.copy_contents {
        #[cfg(unix)]
        copy_fifo(dest, options.overwrite, options.debug)?;
    } else if source_is_symlink {
        copy_link(source, dest, symlinked_files, options)?;
    } else {
        let copy_debug = copy_on_write(
            source,
            dest,
            options.reflink_mode,
            options.sparse_mode,
            context,
            #[cfg(unix)]
            source_is_stream,
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
fn copy_fifo(dest: &Path, overwrite: OverwriteMode, debug: bool) -> CopyResult<()> {
    if dest.exists() {
        overwrite.verify(dest, debug)?;
        fs::remove_file(dest)?;
    }

    make_fifo(dest)
        .map_err(|_| translate!("cp-error-cannot-create-fifo", "path" => dest.quote()).into())
}

#[cfg(unix)]
fn copy_socket(dest: &Path, overwrite: OverwriteMode, debug: bool) -> CopyResult<()> {
    if dest.exists() {
        overwrite.verify(dest, debug)?;
        fs::remove_file(dest)?;
    }

    UnixListener::bind(dest)?;
    Ok(())
}

fn copy_link(
    source: &Path,
    dest: &Path,
    symlinked_files: &mut HashSet<FileInformation>,
    options: &Options,
) -> CopyResult<()> {
    // Here, we will copy the symlink itself (actually, just recreate it)
    let link = fs::read_link(source)?;
    // we always need to remove the file to be able to create a symlink,
    // even if it is writeable.
    if dest.is_symlink() || dest.is_file() {
        delete_path(dest, options)?;
    }
    symlink_file(&link, dest, symlinked_files)?;
    copy_attributes(source, dest, &options.attributes)
}

/// Generate an error message if `target` is not the correct `target_type`
pub fn verify_target_type(target: &Path, target_type: &TargetType) -> CopyResult<()> {
    match (target_type, target.is_dir()) {
        (&TargetType::Directory, false) => Err(translate!("cp-error-target-not-directory", "target" => target.quote())
        .into()),
        (&TargetType::File, true) => Err(translate!("cp-error-cannot-overwrite-directory-with-non-directory", "dir" => target.quote())
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

    use crate::{Attributes, Preserve, aligned_ancestors, localize_to_target};
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
    #[test]
    fn test_diff_attrs() {
        assert_eq!(
            Attributes::ALL.diff(&Attributes {
                context: Preserve::Yes { required: true },
                xattr: Preserve::Yes { required: true },
                ..Attributes::ALL
            }),
            Attributes {
                #[cfg(unix)]
                ownership: Preserve::No { explicit: true },
                mode: Preserve::No { explicit: true },
                timestamps: Preserve::No { explicit: true },
                context: Preserve::No { explicit: true },
                links: Preserve::No { explicit: true },
                xattr: Preserve::No { explicit: true }
            }
        );
        assert_eq!(
            Attributes {
                context: Preserve::Yes { required: true },
                xattr: Preserve::Yes { required: true },
                ..Attributes::ALL
            }
            .diff(&Attributes::NONE),
            Attributes {
                context: Preserve::Yes { required: true },
                xattr: Preserve::Yes { required: true },
                ..Attributes::ALL
            }
        );
        assert_eq!(
            Attributes::NONE.diff(&Attributes {
                context: Preserve::Yes { required: true },
                xattr: Preserve::Yes { required: true },
                ..Attributes::ALL
            }),
            Attributes {
                #[cfg(unix)]
                ownership: Preserve::No { explicit: true },
                mode: Preserve::No { explicit: true },
                timestamps: Preserve::No { explicit: true },
                context: Preserve::No { explicit: true },
                links: Preserve::No { explicit: true },
                xattr: Preserve::No { explicit: true }
            }
        );
    }
}
