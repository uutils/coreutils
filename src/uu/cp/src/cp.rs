#![allow(clippy::missing_safety_doc)]

// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Joshua S. Miller <jsmiller@uchicago.edu>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) ficlone linkgs lstat nlink nlinks pathbuf reflink strs xattrs

#[cfg(target_os = "linux")]
#[macro_use]
extern crate ioctl_sys;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate uucore;

#[cfg(windows)]
use winapi::um::fileapi::CreateFileW;
#[cfg(windows)]
use winapi::um::fileapi::GetFileInformationByHandle;

use std::borrow::Cow;

use clap::{App, Arg, ArgMatches};
use filetime::FileTime;
use quick_error::ResultExt;
use std::collections::HashSet;
use std::env;
#[cfg(not(windows))]
use std::ffi::CString;
#[cfg(windows)]
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::{stdin, stdout, Write};
use std::mem;
#[cfg(target_os = "linux")]
use std::os::unix::io::IntoRawFd;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf, StripPrefixError};
use std::str::FromStr;
use std::string::ToString;
use uucore::fs::resolve_relative_path;
use uucore::fs::{canonicalize, CanonicalizeMode};
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(target_os = "linux")]
#[allow(clippy::missing_safety_doc)]
ioctl!(write ficlone with 0x94, 9; std::os::raw::c_int);

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        /// Simple io::Error wrapper
        IoErr(err: io::Error) { from() cause(err) display("{}", err) }

        /// Wrapper for io::Error with path context
        IoErrContext(err: io::Error, path: String) {
            display("{}: {}", path, err)
            context(path: &'a str, err: io::Error) -> (err, path.to_owned())
            cause(err)
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
        WalkDirErr(err: walkdir::Error) { from() display("{}", err) cause(err) }

        /// Simple std::path::StripPrefixError wrapper
        StripPrefixError(err: StripPrefixError) { from() }

        /// Result of a skipped file
        Skipped(reason: String) { display("{}", reason) }

        /// Result of a skipped file
        InvalidArgument(description: String) { display("{}", description) }

        /// All standard options are included as an an implementation
        /// path, but those that are not implemented yet should return
        /// a NotImplemented error.
        NotImplemented(opt: String) { display("Option '{}' not yet implemented.", opt) }
    }
}

/// Continue next iteration of loop if result of expression is error
macro_rules! or_continue(
    ($expr:expr) => (match $expr {
        Ok(temp) => temp,
        Err(error) => {
            show_error!("{}", error);
            continue
        },
    })
);

/// Prompts the user yes/no and returns `true` if they successfully
/// answered yes.
macro_rules! prompt_yes(
    ($($args:tt)+) => ({
        print!($($args)+);
        print!(" [y/N]: ");
        crash_if_err!(1, stdout().flush());
        let mut s = String::new();
        match stdin().read_line(&mut s) {
            Ok(_) => match s.char_indices().next() {
                Some((_, x)) => x == 'y' || x == 'Y',
                _ => false
            },
            _ => false
        }
    })
);

pub type CopyResult<T> = Result<T, Error>;
pub type Source = PathBuf;
pub type Target = PathBuf;

/// Specifies whether when overwrite files
#[derive(Clone, Eq, PartialEq)]
pub enum ClobberMode {
    Force,
    RemoveDestination,
    Standard,
}

/// Specifies whether when overwrite files
#[derive(Clone, Eq, PartialEq)]
pub enum OverwriteMode {
    /// [Default] Always overwrite existing files
    Clobber(ClobberMode),
    /// Prompt before overwriting a file
    Interactive(ClobberMode),
    /// Never overwrite a file
    NoClobber,
}

#[derive(Clone, Eq, PartialEq)]
pub enum ReflinkMode {
    Always,
    Auto,
    Never,
}

/// Specifies the expected file type of copy target
pub enum TargetType {
    Directory,
    File,
}

#[derive(Clone, Eq, PartialEq)]
pub enum BackupMode {
    ExistingBackup,
    NoBackup,
    NumberedBackup,
    SimpleBackup,
}

pub enum CopyMode {
    Link,
    SymLink,
    Sparse,
    Copy,
    Update,
    AttrOnly,
}

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

/// Re-usable, extensible copy options
#[allow(dead_code)]
pub struct Options {
    attributes_only: bool,
    backup: bool,
    copy_contents: bool,
    copy_mode: CopyMode,
    dereference: bool,
    no_dereference: bool,
    no_target_dir: bool,
    one_file_system: bool,
    overwrite: OverwriteMode,
    parents: bool,
    strip_trailing_slashes: bool,
    reflink: bool,
    reflink_mode: ReflinkMode,
    preserve_attributes: Vec<Attribute>,
    recursive: bool,
    backup_suffix: String,
    target_dir: Option<String>,
    update: bool,
    verbose: bool,
}

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.";
static EXIT_OK: i32 = 0;
static EXIT_ERR: i32 = 1;

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... [-T] SOURCE DEST
    {0} [OPTION]... SOURCE... DIRECTORY
    {0} [OPTION]... -t DIRECTORY SOURCE...",
        executable!()
    )
}

// Argument constants
static OPT_ARCHIVE: &str = "archive";
static OPT_ATTRIBUTES_ONLY: &str = "attributes-only";
static OPT_BACKUP: &str = "backup";
static OPT_CLI_SYMBOLIC_LINKS: &str = "cli-symbolic-links";
static OPT_CONTEXT: &str = "context";
static OPT_COPY_CONTENTS: &str = "copy-contents";
static OPT_DEREFERENCE: &str = "dereference";
static OPT_FORCE: &str = "force";
static OPT_INTERACTIVE: &str = "interactive";
static OPT_LINK: &str = "link";
static OPT_NO_CLOBBER: &str = "no-clobber";
static OPT_NO_DEREFERENCE: &str = "no-dereference";
static OPT_NO_DEREFERENCE_PRESERVE_LINKS: &str = "no-dereference-preserve-linkgs";
static OPT_NO_PRESERVE: &str = "no-preserve";
static OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
static OPT_ONE_FILE_SYSTEM: &str = "one-file-system";
static OPT_PARENT: &str = "parent";
static OPT_PARENTS: &str = "parents";
static OPT_PATHS: &str = "paths";
static OPT_PRESERVE: &str = "preserve";
static OPT_PRESERVE_DEFAULT_ATTRIBUTES: &str = "preserve-default-attributes";
static OPT_RECURSIVE: &str = "recursive";
static OPT_RECURSIVE_ALIAS: &str = "recursive_alias";
static OPT_REFLINK: &str = "reflink";
static OPT_REMOVE_DESTINATION: &str = "remove-destination";
static OPT_SPARSE: &str = "sparse";
static OPT_STRIP_TRAILING_SLASHES: &str = "strip-trailing-slashes";
static OPT_SUFFIX: &str = "suffix";
static OPT_SYMBOLIC_LINK: &str = "symbolic-link";
static OPT_TARGET_DIRECTORY: &str = "target-directory";
static OPT_UPDATE: &str = "update";
static OPT_VERBOSE: &str = "verbose";

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
static PRESERVABLE_ATTRIBUTES: &[&str] = &[
    "ownership",
    "timestamps",
    "context",
    "links",
    "xattr",
    "all",
];

static DEFAULT_ATTRIBUTES: &[Attribute] = &[
    #[cfg(unix)]
    Attribute::Mode,
    Attribute::Ownership,
    Attribute::Timestamps,
];

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
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
             .short("b")
             .long(OPT_BACKUP)
             .help("make a backup of each existing destination file"))
        .arg(Arg::with_name(OPT_SUFFIX)
             .short("S")
             .long(OPT_SUFFIX)
             .takes_value(true)
             .default_value("~")
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
        .get_matches_from(args);

    let options = crash_if_err!(EXIT_ERR, Options::from_matches(&matches));
    let paths: Vec<String> = matches
        .values_of(OPT_PATHS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let (sources, target) = crash_if_err!(EXIT_ERR, parse_path_args(&paths, &options));

    if let Err(error) = copy(&sources, &target, &options) {
        match error {
            // Error::NotAllFilesCopied is non-fatal, but the error
            // code should still be EXIT_ERR as does GNU cp
            Error::NotAllFilesCopied => {}
            // Else we caught a fatal bubbled-up error, log it to stderr
            _ => show_error!("{}", error),
        };
        return EXIT_ERR;
    }

    EXIT_OK
}

impl ClobberMode {
    fn from_matches(matches: &ArgMatches) -> ClobberMode {
        if matches.is_present(OPT_FORCE) {
            ClobberMode::Force
        } else if matches.is_present(OPT_REMOVE_DESTINATION) {
            ClobberMode::RemoveDestination
        } else {
            ClobberMode::Standard
        }
    }
}

impl OverwriteMode {
    fn from_matches(matches: &ArgMatches) -> OverwriteMode {
        if matches.is_present(OPT_INTERACTIVE) {
            OverwriteMode::Interactive(ClobberMode::from_matches(matches))
        } else if matches.is_present(OPT_NO_CLOBBER) {
            OverwriteMode::NoClobber
        } else {
            OverwriteMode::Clobber(ClobberMode::from_matches(matches))
        }
    }
}

impl CopyMode {
    fn from_matches(matches: &ArgMatches) -> CopyMode {
        if matches.is_present(OPT_LINK) {
            CopyMode::Link
        } else if matches.is_present(OPT_SYMBOLIC_LINK) {
            CopyMode::SymLink
        } else if matches.is_present(OPT_SPARSE) {
            CopyMode::Sparse
        } else if matches.is_present(OPT_UPDATE) {
            CopyMode::Update
        } else if matches.is_present(OPT_ATTRIBUTES_ONLY) {
            CopyMode::AttrOnly
        } else {
            CopyMode::Copy
        }
    }
}

impl FromStr for Attribute {
    type Err = Error;

    fn from_str(value: &str) -> CopyResult<Attribute> {
        Ok(match &*value.to_lowercase() {
            #[cfg(unix)]
            "mode" => Attribute::Mode,
            "ownership" => Attribute::Ownership,
            "timestamps" => Attribute::Timestamps,
            "context" => Attribute::Context,
            "links" => Attribute::Links,
            "xattr" => Attribute::Xattr,
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "invalid attribute '{}'",
                    value
                )));
            }
        })
    }
}

fn add_all_attributes() -> Vec<Attribute> {
    let mut attr = Vec::new();
    #[cfg(unix)]
    attr.push(Attribute::Mode);
    attr.push(Attribute::Ownership);
    attr.push(Attribute::Timestamps);
    attr.push(Attribute::Context);
    attr.push(Attribute::Xattr);
    attr.push(Attribute::Links);
    attr
}

impl Options {
    fn from_matches(matches: &ArgMatches) -> CopyResult<Options> {
        let not_implemented_opts = vec![
            OPT_COPY_CONTENTS,
            OPT_SPARSE,
            #[cfg(not(any(windows, unix)))]
            OPT_ONE_FILE_SYSTEM,
            OPT_CONTEXT,
            #[cfg(windows)]
            OPT_FORCE,
        ];

        for not_implemented_opt in not_implemented_opts {
            if matches.is_present(not_implemented_opt) {
                return Err(Error::NotImplemented(not_implemented_opt.to_string()));
            }
        }

        let recursive = matches.is_present(OPT_RECURSIVE)
            || matches.is_present(OPT_RECURSIVE_ALIAS)
            || matches.is_present(OPT_ARCHIVE);

        let backup = matches.is_present(OPT_BACKUP) || (matches.occurrences_of(OPT_SUFFIX) > 0);

        // Parse target directory options
        let no_target_dir = matches.is_present(OPT_NO_TARGET_DIRECTORY);
        let target_dir = matches
            .value_of(OPT_TARGET_DIRECTORY)
            .map(ToString::to_string);

        // Parse attributes to preserve
        let preserve_attributes: Vec<Attribute> = if matches.is_present(OPT_PRESERVE) {
            match matches.values_of(OPT_PRESERVE) {
                None => DEFAULT_ATTRIBUTES.to_vec(),
                Some(attribute_strs) => {
                    let mut attributes = Vec::new();
                    for attribute_str in attribute_strs {
                        if attribute_str == "all" {
                            attributes = add_all_attributes();
                            break;
                        } else {
                            attributes.push(Attribute::from_str(attribute_str)?);
                        }
                    }
                    attributes
                }
            }
        } else if matches.is_present(OPT_ARCHIVE) {
            // --archive is used. Same as --preserve=all
            add_all_attributes()
        } else if matches.is_present(OPT_NO_DEREFERENCE_PRESERVE_LINKS) {
            vec![Attribute::Links]
        } else if matches.is_present(OPT_PRESERVE_DEFAULT_ATTRIBUTES) {
            DEFAULT_ATTRIBUTES.to_vec()
        } else {
            vec![]
        };

        let options = Options {
            attributes_only: matches.is_present(OPT_ATTRIBUTES_ONLY),
            copy_contents: matches.is_present(OPT_COPY_CONTENTS),
            copy_mode: CopyMode::from_matches(matches),
            dereference: matches.is_present(OPT_DEREFERENCE),
            // No dereference is set with -p, -d and --archive
            no_dereference: matches.is_present(OPT_NO_DEREFERENCE)
                || matches.is_present(OPT_NO_DEREFERENCE_PRESERVE_LINKS)
                || matches.is_present(OPT_ARCHIVE),
            one_file_system: matches.is_present(OPT_ONE_FILE_SYSTEM),
            overwrite: OverwriteMode::from_matches(matches),
            parents: matches.is_present(OPT_PARENTS),
            backup_suffix: matches.value_of(OPT_SUFFIX).unwrap().to_string(),
            update: matches.is_present(OPT_UPDATE),
            verbose: matches.is_present(OPT_VERBOSE),
            strip_trailing_slashes: matches.is_present(OPT_STRIP_TRAILING_SLASHES),
            reflink: matches.is_present(OPT_REFLINK),
            reflink_mode: {
                if let Some(reflink) = matches.value_of(OPT_REFLINK) {
                    match reflink {
                        "always" => ReflinkMode::Always,
                        "auto" => ReflinkMode::Auto,
                        value => {
                            return Err(Error::InvalidArgument(format!(
                                "invalid argument '{}' for \'reflink\'",
                                value
                            )));
                        }
                    }
                } else {
                    ReflinkMode::Never
                }
            },
            backup,
            no_target_dir,
            preserve_attributes,
            recursive,
            target_dir,
        };

        Ok(options)
    }
}

impl TargetType {
    /// Return TargetType required for `target`.
    ///
    /// Treat target as a dir if we have multiple sources or the target
    /// exists and already is a directory
    fn determine(sources: &[Source], target: &Target) -> TargetType {
        if sources.len() > 1 || target.is_dir() {
            TargetType::Directory
        } else {
            TargetType::File
        }
    }
}

/// Returns tuple of (Source paths, Target)
fn parse_path_args(path_args: &[String], options: &Options) -> CopyResult<(Vec<Source>, Target)> {
    let mut paths = path_args.iter().map(PathBuf::from).collect::<Vec<_>>();

    if paths.is_empty() {
        // No files specified
        return Err("missing file operand".into());
    }

    // Return an error if the user requested to copy more than one
    // file source to a file target
    if options.no_target_dir && options.target_dir.is_none() && paths.len() > 2 {
        return Err(format!("extra operand {:?}", paths[2]).into());
    }

    let (mut sources, target) = match options.target_dir {
        Some(ref target) => {
            // All path args are sources, and the target dir was
            // specified separately
            (paths, PathBuf::from(target))
        }
        None => {
            // If there was no explicit target-dir, then use the last
            // path_arg
            let target = paths.pop().unwrap();
            (paths, target)
        }
    };

    if options.strip_trailing_slashes {
        for source in sources.iter_mut() {
            *source = source.components().as_path().to_owned()
        }
    }

    Ok((sources, target))
}

fn preserve_hardlinks(
    hard_links: &mut Vec<(String, u64)>,
    source: &std::path::PathBuf,
    dest: std::path::PathBuf,
    found_hard_link: &mut bool,
) -> CopyResult<()> {
    // Redox does not currently support hard links
    #[cfg(not(target_os = "redox"))]
    {
        if !source.is_dir() {
            unsafe {
                let inode: u64;
                let nlinks: u64;
                #[cfg(unix)]
                {
                    let src_path = CString::new(source.as_os_str().to_str().unwrap()).unwrap();
                    let mut stat = mem::zeroed();
                    if libc::lstat(src_path.as_ptr(), &mut stat) < 0 {
                        return Err(format!(
                            "cannot stat {:?}: {}",
                            src_path,
                            std::io::Error::last_os_error()
                        )
                        .into());
                    }
                    inode = stat.st_ino as u64;
                    nlinks = stat.st_nlink as u64;
                }
                #[cfg(windows)]
                {
                    let src_path: Vec<u16> = OsStr::new(source).encode_wide().collect();
                    #[allow(deprecated)]
                    let stat = mem::uninitialized();
                    let handle = CreateFileW(
                        src_path.as_ptr(),
                        winapi::um::winnt::GENERIC_READ,
                        winapi::um::winnt::FILE_SHARE_READ,
                        std::ptr::null_mut(),
                        0,
                        0,
                        std::ptr::null_mut(),
                    );
                    if GetFileInformationByHandle(handle, stat) != 0 {
                        return Err(format!(
                            "cannot get file information {:?}: {}",
                            source,
                            std::io::Error::last_os_error()
                        )
                        .into());
                    }
                    inode = ((*stat).nFileIndexHigh as u64) << 32 | (*stat).nFileIndexLow as u64;
                    nlinks = (*stat).nNumberOfLinks as u64;
                }

                for hard_link in hard_links.iter() {
                    if hard_link.1 == inode {
                        std::fs::hard_link(hard_link.0.clone(), dest.clone()).unwrap();
                        *found_hard_link = true;
                    }
                }
                if !(*found_hard_link) && nlinks > 1 {
                    hard_links.push((dest.to_str().unwrap().to_string(), inode));
                }
            }
        }
    }
    Ok(())
}

/// Copy all `sources` to `target`.  Returns an
/// `Err(Error::NotAllFilesCopied)` if at least one non-fatal error was
/// encountered.
///
/// Behavior depends on `options`, see [`Options`] for details.
///
/// [`Options`]: ./struct.Options.html
fn copy(sources: &[Source], target: &Target, options: &Options) -> CopyResult<()> {
    let target_type = TargetType::determine(sources, target);
    verify_target_type(target, &target_type)?;

    let mut preserve_hard_links = false;
    for attribute in &options.preserve_attributes {
        if *attribute == Attribute::Links {
            preserve_hard_links = true;
        }
    }

    let mut hard_links: Vec<(String, u64)> = vec![];

    let mut non_fatal_errors = false;
    let mut seen_sources = HashSet::with_capacity(sources.len());
    for source in sources {
        if seen_sources.contains(source) {
            show_warning!("source '{}' specified more than once", source.display());
        } else {
            let mut found_hard_link = false;
            if preserve_hard_links {
                let dest = construct_dest_path(source, target, &target_type, options)?;
                preserve_hardlinks(&mut hard_links, source, dest, &mut found_hard_link).unwrap();
            }
            if !found_hard_link {
                if let Err(error) = copy_source(source, target, &target_type, options) {
                    match error {
                        // When using --no-clobber, we don't want to show
                        // an error message
                        Error::NotAllFilesCopied => (),
                        Error::Skipped(_) => {
                            show_error!("{}", error);
                        }
                        _ => {
                            show_error!("{}", error);
                            non_fatal_errors = true
                        }
                    }
                }
            }
            seen_sources.insert(source);
        }
    }
    if non_fatal_errors {
        Err(Error::NotAllFilesCopied)
    } else {
        Ok(())
    }
}

fn construct_dest_path(
    source_path: &Path,
    target: &Target,
    target_type: &TargetType,
    options: &Options,
) -> CopyResult<PathBuf> {
    if options.no_target_dir && target.is_dir() {
        return Err(format!(
            "cannot overwrite directory '{}' with non-directory",
            target.display()
        )
        .into());
    }

    if options.parents && !target.is_dir() {
        return Err("with --parents, the destination must be a directory".into());
    }

    Ok(match *target_type {
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
    source: &Source,
    target: &Target,
    target_type: &TargetType,
    options: &Options,
) -> CopyResult<()> {
    let source_path = Path::new(&source);
    if source_path.is_dir() {
        // Copy as directory
        copy_directory(source, target, options)
    } else {
        // Copy as file
        let dest = construct_dest_path(source_path, target, target_type, options)?;
        copy_file(source_path, dest.as_path(), options)
    }
}

#[cfg(target_os = "windows")]
fn adjust_canonicalization(p: &Path) -> Cow<Path> {
    // In some cases, \\? can be missing on some Windows paths.  Add it at the
    // beginning unless the path is prefixed with a device namespace.
    const VERBATIM_PREFIX: &str = r#"\\?"#;
    const DEVICE_NS_PREFIX: &str = r#"\\."#;

    let has_prefix = p
        .components()
        .next()
        .and_then(|comp| comp.as_os_str().to_str())
        .map(|p_str| p_str.starts_with(VERBATIM_PREFIX) || p_str.starts_with(DEVICE_NS_PREFIX))
        .unwrap_or_default();

    if has_prefix {
        p.into()
    } else {
        Path::new(VERBATIM_PREFIX).join(p).into()
    }
}

/// Read the contents of the directory `root` and recursively copy the
/// contents to `target`.
///
/// Any errors encountered copying files in the tree will be logged but
/// will not cause a short-circuit.
fn copy_directory(root: &Path, target: &Target, options: &Options) -> CopyResult<()> {
    if !options.recursive {
        return Err(format!("omitting directory '{}'", root.display()).into());
    }

    let root_path = Path::new(&root).canonicalize()?;

    let root_parent = if target.exists() {
        root_path.parent()
    } else {
        Some(root_path.as_path())
    };

    #[cfg(unix)]
    let mut hard_links: Vec<(String, u64)> = vec![];
    let mut preserve_hard_links = false;
    for attribute in &options.preserve_attributes {
        if *attribute == Attribute::Links {
            preserve_hard_links = true;
        }
    }

    // This should be changed once Redox supports hardlinks
    #[cfg(any(windows, target_os = "redox"))]
    let mut hard_links: Vec<(String, u64)> = vec![];

    for path in WalkDir::new(root).same_file_system(options.one_file_system) {
        let p = or_continue!(path);
        let is_symlink = fs::symlink_metadata(p.path())?.file_type().is_symlink();
        let path = if (options.no_dereference || options.dereference) && is_symlink {
            // we are dealing with a symlink. Don't follow it
            match env::current_dir() {
                Ok(cwd) => cwd.join(resolve_relative_path(p.path())),
                Err(e) => crash!(1, "failed to get current directory {}", e),
            }
        } else {
            or_continue!(p.path().canonicalize())
        };

        let local_to_root_parent = match root_parent {
            Some(parent) => {
                #[cfg(windows)]
                {
                    // On Windows, some paths are starting with \\?
                    // but not always, so, make sure that we are consistent for strip_prefix
                    // See https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file for more info
                    let parent_can = adjust_canonicalization(parent);
                    let path_can = adjust_canonicalization(&path);

                    or_continue!(&path_can.strip_prefix(&parent_can)).to_path_buf()
                }
                #[cfg(not(windows))]
                {
                    or_continue!(path.strip_prefix(&parent)).to_path_buf()
                }
            }
            None => path.clone(),
        };

        let local_to_target = target.join(&local_to_root_parent);

        if path.is_dir() && !local_to_target.exists() {
            or_continue!(fs::create_dir_all(local_to_target.clone()));
        } else if !path.is_dir() {
            if preserve_hard_links {
                let mut found_hard_link = false;
                let source = path.to_path_buf();
                let dest = local_to_target.as_path().to_path_buf();
                preserve_hardlinks(&mut hard_links, &source, dest, &mut found_hard_link).unwrap();
                if !found_hard_link {
                    match copy_file(path.as_path(), local_to_target.as_path(), options) {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            if fs::symlink_metadata(&source)?.file_type().is_symlink() {
                                // silent the error with a symlink
                                // In case we do --archive, we might copy the symlink
                                // before the file itself
                                Ok(())
                            } else {
                                Err(err)
                            }
                        }
                    }?;
                }
            } else {
                copy_file(path.as_path(), local_to_target.as_path(), options)?;
            }
        }
    }

    Ok(())
}

impl OverwriteMode {
    fn verify(&self, path: &Path) -> CopyResult<()> {
        match *self {
            OverwriteMode::NoClobber => Err(Error::NotAllFilesCopied),
            OverwriteMode::Interactive(_) => {
                if prompt_yes!("{}: overwrite {}? ", executable!(), path.display()) {
                    Ok(())
                } else {
                    Err(Error::Skipped(format!(
                        "Not overwriting {} at user request",
                        path.display()
                    )))
                }
            }
            OverwriteMode::Clobber(_) => Ok(()),
        }
    }
}

fn copy_attribute(source: &Path, dest: &Path, attribute: &Attribute) -> CopyResult<()> {
    let context = &*format!("'{}' -> '{}'", source.display().to_string(), dest.display());
    match *attribute {
        #[cfg(unix)]
        Attribute::Mode => {
            let mode = fs::metadata(source).context(context)?.permissions().mode();
            let mut dest_metadata = fs::metadata(source).context(context)?.permissions();
            dest_metadata.set_mode(mode);
        }
        Attribute::Ownership => {
            let metadata = fs::metadata(source).context(context)?;
            fs::set_permissions(dest, metadata.permissions()).context(context)?;
        }
        Attribute::Timestamps => {
            let metadata = fs::metadata(source)?;
            filetime::set_file_times(
                Path::new(dest),
                FileTime::from_last_access_time(&metadata),
                FileTime::from_last_modification_time(&metadata),
            )?;
        }
        Attribute::Context => {}
        Attribute::Links => {}
        Attribute::Xattr => {
            #[cfg(unix)]
            {
                let xattrs = xattr::list(source)?;
                for attr in xattrs {
                    if let Some(attr_value) = xattr::get(source, attr.clone())? {
                        crash_if_err!(EXIT_ERR, xattr::set(dest, attr, &attr_value[..]));
                    }
                }
            }
            #[cfg(not(unix))]
            {
                return Err("XAttrs are only supported on unix.".to_string().into());
            }
        }
    };

    Ok(())
}

#[cfg(not(windows))]
fn symlink_file(source: &Path, dest: &Path, context: &str) -> CopyResult<()> {
    match std::os::unix::fs::symlink(source, dest).context(context) {
        Ok(_) => Ok(()),
        Err(_) => Ok(()),
    }
}

#[cfg(windows)]
fn symlink_file(source: &Path, dest: &Path, context: &str) -> CopyResult<()> {
    Ok(std::os::windows::fs::symlink_file(source, dest).context(context)?)
}

fn context_for(src: &Path, dest: &Path) -> String {
    format!("'{}' -> '{}'", src.display(), dest.display())
}

/// Implements a relatively naive backup that is not as full featured
/// as GNU cp.  No CONTROL version control method argument is taken
/// for backups.
/// TODO: Add version control methods
fn backup_file(path: &Path, suffix: &str) -> CopyResult<PathBuf> {
    let mut backup_path = path.to_path_buf().into_os_string();
    backup_path.push(suffix);
    fs::copy(path, &backup_path)?;
    Ok(backup_path.into())
}

fn handle_existing_dest(source: &Path, dest: &Path, options: &Options) -> CopyResult<()> {
    if paths_refer_to_same_file(source, dest)? {
        return Err(format!("{}: same file", context_for(source, dest)).into());
    }

    options.overwrite.verify(dest)?;

    if options.backup {
        backup_file(dest, &options.backup_suffix)?;
    }

    match options.overwrite {
        OverwriteMode::Clobber(ClobberMode::Force) => {
            if fs::metadata(dest)?.permissions().readonly() {
                fs::remove_file(dest)?;
            }
        }
        OverwriteMode::Clobber(ClobberMode::RemoveDestination) => {
            fs::remove_file(dest)?;
        }
        _ => (),
    };

    Ok(())
}

/// Copy the a file from `source` to `dest`. No path manipulation is
/// done on either `source` or `dest`, the are used as provided.
///
/// Behavior when copying to existing files is contingent on the
/// `options.overwrite` mode. If a file is skipped, the return type
/// should be `Error:Skipped`
///
/// The original permissions of `source` will be copied to `dest`
/// after a successful copy.
fn copy_file(source: &Path, dest: &Path, options: &Options) -> CopyResult<()> {
    if dest.exists() {
        handle_existing_dest(source, dest, options)?;
    }

    if options.verbose {
        println!("{}", context_for(source, dest));
    }

    #[allow(unused)]
    {
        // TODO: implement --preserve flag
        let mut preserve_context = false;
        for attribute in &options.preserve_attributes {
            if *attribute == Attribute::Context {
                preserve_context = true;
            }
        }
    }
    match options.copy_mode {
        CopyMode::Link => {
            fs::hard_link(source, dest).context(&*context_for(source, dest))?;
        }
        CopyMode::Copy => {
            copy_helper(source, dest, options)?;
        }
        CopyMode::SymLink => {
            symlink_file(source, dest, &*context_for(source, dest))?;
        }
        CopyMode::Sparse => return Err(Error::NotImplemented(OPT_SPARSE.to_string())),
        CopyMode::Update => {
            if dest.exists() {
                let src_metadata = fs::metadata(source)?;
                let dest_metadata = fs::metadata(dest)?;

                let src_time = src_metadata.modified()?;
                let dest_time = dest_metadata.modified()?;
                if src_time <= dest_time {
                    return Ok(());
                } else {
                    copy_helper(source, dest, options)?;
                }
            } else {
                copy_helper(source, dest, options)?;
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
    for attribute in &options.preserve_attributes {
        copy_attribute(source, dest, attribute)?;
    }
    Ok(())
}

///Copy the file from `source` to `dest` either using the normal `fs::copy` or the
///`FICLONE` ioctl if --reflink is specified and the filesystem supports it.
fn copy_helper(source: &Path, dest: &Path, options: &Options) -> CopyResult<()> {
    if options.reflink {
        #[cfg(not(target_os = "linux"))]
        return Err("--reflink is only supported on linux".to_string().into());

        #[cfg(target_os = "linux")]
        {
            let src_file = File::open(source).unwrap().into_raw_fd();
            let dst_file = OpenOptions::new()
                .write(true)
                .truncate(false)
                .create(true)
                .open(dest)
                .unwrap()
                .into_raw_fd();
            match options.reflink_mode {
                ReflinkMode::Always => unsafe {
                    let result = ficlone(dst_file, src_file as *const i32);
                    if result != 0 {
                        return Err(format!(
                            "failed to clone {:?} from {:?}: {}",
                            source,
                            dest,
                            std::io::Error::last_os_error()
                        )
                        .into());
                    } else {
                        return Ok(());
                    }
                },
                ReflinkMode::Auto => unsafe {
                    let result = ficlone(dst_file, src_file as *const i32);
                    if result != 0 {
                        fs::copy(source, dest).context(&*context_for(source, dest))?;
                    }
                },
                ReflinkMode::Never => {}
            }
        }
    } else if options.no_dereference && fs::symlink_metadata(&source)?.file_type().is_symlink() {
        // Here, we will copy the symlink itself (actually, just recreate it)
        let link = fs::read_link(&source)?;
        let dest: Cow<'_, Path> = if dest.is_dir() {
            match source.file_name() {
                Some(name) => dest.join(name).into(),
                None => crash!(
                    EXIT_ERR,
                    "cannot stat ‘{}’: No such file or directory",
                    source.display()
                ),
            }
        } else {
            dest.into()
        };
        symlink_file(&link, &dest, &*context_for(&link, &dest))?;
    } else if source.to_string_lossy() == "/dev/null" {
        /* workaround a limitation of fs::copy
         * https://github.com/rust-lang/rust/issues/79390
         */
        File::create(dest)?;
    } else {
        if options.parents {
            let parent = dest.parent().unwrap_or(dest);
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, dest).context(&*context_for(source, dest))?;
    }

    Ok(())
}

/// Generate an error message if `target` is not the correct `target_type`
pub fn verify_target_type(target: &Path, target_type: &TargetType) -> CopyResult<()> {
    match (target_type, target.is_dir()) {
        (&TargetType::Directory, false) => {
            Err(format!("target: '{}' is not a directory", target.display()).into())
        }
        (&TargetType::File, true) => Err(format!(
            "cannot overwrite directory '{}' with non-directory",
            target.display()
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
    let local_to_root = source.strip_prefix(&root)?;
    Ok(target.join(&local_to_root))
}

pub fn paths_refer_to_same_file(p1: &Path, p2: &Path) -> io::Result<bool> {
    // We have to take symlinks and relative paths into account.
    let pathbuf1 = canonicalize(p1, CanonicalizeMode::Normal)?;
    let pathbuf2 = canonicalize(p2, CanonicalizeMode::Normal)?;

    Ok(pathbuf1 == pathbuf2)
}

#[test]
fn test_cp_localize_to_target() {
    assert!(
        localize_to_target(
            &Path::new("a/source/"),
            &Path::new("a/source/c.txt"),
            &Path::new("target/")
        )
        .unwrap()
            == Path::new("target/c.txt")
    )
}
