#![crate_name = "uu_cp"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 * (c) Joshua S. Miller <jsmiller@uchicago.edu>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate clap;
extern crate walkdir;
#[cfg(target_os = "linux")]
#[macro_use] extern crate ioctl_sys;
#[macro_use] extern crate uucore;
#[macro_use] extern crate quick_error;

use clap::{Arg, App, ArgMatches};
use quick_error::ResultExt;
use std::collections::HashSet;
use std::fs;
use std::io::{BufReader, BufRead, stdin, Write};
use std::io;
use std::path::{Path, PathBuf, StripPrefixError};
use std::str::FromStr;
use uucore::fs::{canonicalize, CanonicalizeMode};
use walkdir::WalkDir;
#[cfg(target_os = "linux")] use std::os::unix::io::IntoRawFd;
use std::fs::File;
use std::fs::OpenOptions;

#[cfg(unix)] use std::os::unix::fs::PermissionsExt;

#[cfg(target_os = "linux")] ioctl!(write ficlone with 0x94, 9; std::os::raw::c_int);

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

        /// Represents the state when a non-fatal error has occured
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


/// Prompts the user yes/no and returns `true` they if successfully
/// answered yes.
macro_rules! prompt_yes(
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stdout(), $($args)+);
        pipe_write!(&mut ::std::io::stdout(), " [y/N]: ");
        pipe_flush!();
        let mut s = String::new();
        match BufReader::new(stdin()).read_line(&mut s) {
            Ok(_) => match s.char_indices().nth(0) {
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
#[derive (Clone, Eq, PartialEq)]
pub enum ClobberMode {
    Force,
    RemoveDestination,
    Standard,
}

/// Specifies whether when overwrite files
#[derive (Clone, Eq, PartialEq)]
pub enum OverwriteMode {
    /// [Default] Always overwrite existing files
    Clobber(ClobberMode),
    /// Prompt before overwriting a file
    Interactive(ClobberMode),
    /// Never overwrite a file
    NoClobber,
}

#[derive (Clone, Eq, PartialEq)]
pub enum ReflinkMode {
    Always, Auto, Never
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
}

#[derive(Clone)]
pub enum Attribute {
    #[cfg(unix)] Mode,
    Ownership,
    Timestamps,
    Context,
    Links,
    Xattr,
    All,
}

/// Re-usable, extensible copy options
#[allow(dead_code)]
pub struct Options {
    attributes_only: bool,
    backup: bool,
    copy_contents: bool,
    copy_mode: CopyMode,
    dereference: bool,
    no_target_dir: bool,
    one_file_system: bool,
    overwrite: OverwriteMode,
    parents: bool,
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
static USAGE: &str = "Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.";
static EXIT_OK: i32 = 0;
static EXIT_ERR: i32 = 1;

/// Prints the version
fn print_version() {
    println!("{} {}", executable!(), VERSION);
}

/// Prints usage/help
fn get_about(usage: &str) -> String {
    format!("Usage: {0} [OPTION]... [-T] SOURCE DEST
  or:  {0} [OPTION]... SOURCE... DIRECTORY
  or:  {0} [OPTION]... -t DIRECTORY SOURCE...
{1}", executable!(), usage)
}


// Argument constants
static OPT_ARCHIVE:                       &str = "archive";
static OPT_ATTRIBUTES_ONLY:               &str = "attributes-only";
static OPT_BACKUP:                        &str = "backup";
static OPT_CLI_SYMBOLIC_LINKS:            &str = "cli-symbolic-links";
static OPT_CONTEXT:                       &str = "context";
static OPT_COPY_CONTENTS:                 &str = "copy-contents";
static OPT_DEREFERENCE:                   &str = "dereference";
static OPT_FORCE:                         &str = "force";
static OPT_INTERACTIVE:                   &str = "interactive";
static OPT_LINK:                          &str = "link";
static OPT_NO_CLOBBER:                    &str = "no-clobber";
static OPT_NO_DEREFERENCE:                &str = "no-dereference";
static OPT_NO_DEREFERENCE_PRESERVE_LINKS: &str = "no-dereference-preserve-linkgs";
static OPT_NO_PRESERVE:                   &str = "no-preserve";
static OPT_NO_TARGET_DIRECTORY:           &str = "no-target-directory";
static OPT_ONE_FILE_SYSTEM:               &str = "one-file-system";
static OPT_PARENTS:                       &str = "parents";
static OPT_PATHS:                         &str = "paths";
static OPT_PRESERVE:                      &str = "preserve";
static OPT_PRESERVE_DEFUALT_ATTRIBUTES:   &str = "preserve-default-attributes";
static OPT_RECURSIVE:                     &str = "recursive";
static OPT_RECURSIVE_ALIAS:               &str = "recursive_alias";
static OPT_REFLINK:                       &str = "reflink";
static OPT_REMOVE_DESTINATION:            &str = "remove-destination";
static OPT_SPARSE:                        &str = "sparse";
static OPT_STRIP_TRAILING_SLASHES:        &str = "strip-trailing-slashes";
static OPT_SUFFIX:                        &str = "suffix";
static OPT_SYMBOLIC_LINK:                 &str = "symbolic-link";
static OPT_TARGET_DIRECTORY:              &str = "target-directory";
static OPT_UPDATE:                        &str = "update";
static OPT_VERBOSE:                       &str = "verbose";
static OPT_VERSION:                       &str = "version";

#[cfg(unix)]
static PRESERVABLE_ATTRIBUTES: &[&str] = &["mode", "ownership", "timestamps", "context", "links", "xattr", "all"];

#[cfg(not(unix))]
static PRESERVABLE_ATTRIBUTES: &[&str] = &["ownership", "timestamps", "context", "links", "xattr", "all"];

static DEFAULT_ATTRIBUTES: &[Attribute] = &[
    #[cfg(unix)] Attribute::Mode,
    Attribute::Ownership,
    Attribute::Timestamps,
];


pub fn uumain(args: Vec<String>) -> i32 {
    let about = get_about(USAGE);
    let matches = App::new(executable!())
        .version(VERSION)
        .about(&about[..])
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
        .arg(Arg::with_name(OPT_VERSION)
             .short("V")
             .long(OPT_VERSION)
             .help("output version information and exit"))
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
             .help("copy directories recursively"))
        .arg(Arg::with_name(OPT_RECURSIVE_ALIAS)
             .short("R")
             .help("same as -r"))
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

        // TODO: implement the following args
        .arg(Arg::with_name(OPT_ARCHIVE)
             .short("a")
             .long(OPT_ARCHIVE)
             .conflicts_with_all(&[OPT_PRESERVE_DEFUALT_ATTRIBUTES, OPT_PRESERVE, OPT_NO_PRESERVE])
             .help("NotImplemented: same as -dR --preserve=all"))
        .arg(Arg::with_name(OPT_ATTRIBUTES_ONLY)
             .long(OPT_ATTRIBUTES_ONLY)
             .conflicts_with(OPT_COPY_CONTENTS)
             .overrides_with(OPT_REFLINK)
             .help("NotImplemented: don't copy the file data, just the attributes"))
        .arg(Arg::with_name(OPT_COPY_CONTENTS)
             .long(OPT_COPY_CONTENTS)
             .conflicts_with(OPT_ATTRIBUTES_ONLY)
             .help("NotImplemented: copy contents of special files when recursive"))
        .arg(Arg::with_name(OPT_NO_DEREFERENCE_PRESERVE_LINKS)
             .short("d")
             .help("NotImplemented: same as --no-dereference --preserve=links"))
        .arg(Arg::with_name(OPT_DEREFERENCE)
             .short("L")
             .long(OPT_DEREFERENCE)
             .conflicts_with(OPT_NO_DEREFERENCE)
             .help("NotImplemented: always follow symbolic links in SOURCE"))
        .arg(Arg::with_name(OPT_NO_DEREFERENCE)
             .short("-P")
             .long(OPT_NO_DEREFERENCE)
             .conflicts_with(OPT_DEREFERENCE)
             .help("NotImplemented: never follow symbolic links in SOURCE"))
        .arg(Arg::with_name(OPT_PRESERVE_DEFUALT_ATTRIBUTES)
             .short("-p")
             .long(OPT_PRESERVE_DEFUALT_ATTRIBUTES)
             .conflicts_with_all(&[OPT_PRESERVE, OPT_NO_PRESERVE, OPT_ARCHIVE])
             .help("NotImplemented: same as --preserve=mode(unix only),ownership,timestamps"))
        .arg(Arg::with_name(OPT_PRESERVE)
             .long(OPT_PRESERVE)
             .takes_value(true)
             .multiple(true)
             .possible_values(PRESERVABLE_ATTRIBUTES)
             .value_name("ATTR_LIST")
             .conflicts_with_all(&[OPT_PRESERVE_DEFUALT_ATTRIBUTES, OPT_NO_PRESERVE, OPT_ARCHIVE])
             .help("NotImplemented: preserve the specified attributes (default: mode(unix only),ownership,timestamps),\
                    if possible additional attributes: context, links, xattr, all"))
        .arg(Arg::with_name(OPT_NO_PRESERVE)
             .long(OPT_NO_PRESERVE)
             .takes_value(true)
             .value_name("ATTR_LIST")
             .conflicts_with_all(&[OPT_PRESERVE_DEFUALT_ATTRIBUTES, OPT_PRESERVE, OPT_ARCHIVE])
             .help("NotImplemented: don't preserve the specified attributes"))
        .arg(Arg::with_name(OPT_PARENTS)
             .long(OPT_PARENTS)
             .help("NotImplemented: use full source file name under DIRECTORY"))
        .arg(Arg::with_name(OPT_SPARSE)
             .long(OPT_SPARSE)
             .takes_value(true)
             .value_name("WHEN")
             .help("NotImplemented: control creation of sparse files. See below"))
        .arg(Arg::with_name(OPT_STRIP_TRAILING_SLASHES)
             .long(OPT_STRIP_TRAILING_SLASHES)
             .help("NotImplemented: remove any trailing slashes from each SOURCE argument"))
        .arg(Arg::with_name(OPT_ONE_FILE_SYSTEM)
             .short("x")
             .long(OPT_ONE_FILE_SYSTEM)
             .help("NotImplemented: stay on this file system"))
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
        .get_matches_from(&args);

    if matches.is_present(OPT_VERSION) {
        print_version();
        return EXIT_OK;
    }

    let options = crash_if_err!(EXIT_ERR, Options::from_matches(&matches));
    let paths: Vec<String> = matches.values_of("paths")
        .map(|v| v.map(|p| p.to_string()).collect())
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
        } else {
            CopyMode::Copy
        }
    }
}

impl FromStr for Attribute {
    type Err = Error;

    fn from_str(value: &str) -> CopyResult<Attribute> {
        Ok(match &*value.to_lowercase() {
            #[cfg(unix)] "mode" => Attribute::Mode,
            "ownership" => Attribute::Ownership,
            "timestamps" => Attribute::Timestamps,
            "context" => Attribute::Context,
            "links" => Attribute::Links,
            "xattr" => Attribute::Xattr,
            "all" => Attribute::All,
            _ => return Err(Error::InvalidArgument(format!("invalid attribute '{}'", value)))
        })
    }
}

impl Options {
    fn from_matches(matches: &ArgMatches) -> CopyResult<Options> {
        let not_implemented_opts =  vec![
            OPT_ARCHIVE,
            OPT_ATTRIBUTES_ONLY,
            OPT_COPY_CONTENTS,
            OPT_NO_DEREFERENCE_PRESERVE_LINKS,
            OPT_DEREFERENCE,
            OPT_NO_DEREFERENCE,
            OPT_PRESERVE_DEFUALT_ATTRIBUTES,
            OPT_PRESERVE,
            OPT_NO_PRESERVE,
            OPT_PARENTS,
            OPT_SPARSE,
            OPT_STRIP_TRAILING_SLASHES,
            OPT_ONE_FILE_SYSTEM,
            OPT_CONTEXT,
            #[cfg(windows)] OPT_FORCE,
        ];

        for not_implemented_opt in not_implemented_opts {
            if matches.is_present(not_implemented_opt) {
                return Err(Error::NotImplemented(not_implemented_opt.to_string()))
            }
        }

        let recursive = matches.is_present(OPT_RECURSIVE)
            || matches.is_present(OPT_RECURSIVE_ALIAS)
            || matches.is_present(OPT_ARCHIVE);

        let backup = matches.is_present(OPT_BACKUP)
            || matches.is_present(OPT_SUFFIX);

        // Parse target directory options
        let no_target_dir = matches.is_present(OPT_NO_TARGET_DIRECTORY);
        let target_dir = matches.value_of(OPT_TARGET_DIRECTORY).map(|v| v.to_string());

        // Parse attributes to preserve
        let preserve_attributes: Vec<Attribute> = if matches.is_present(OPT_PRESERVE) {
            match matches.values_of(OPT_PRESERVE) {
                None => DEFAULT_ATTRIBUTES.to_vec(),
                Some(attribute_strs) => {
                    let mut attributes = Vec::new();
                    for attribute_str in attribute_strs {
                        attributes.push(Attribute::from_str(attribute_str)?);
                    }
                    attributes
                }
            }
        } else if matches.is_present(OPT_PRESERVE_DEFUALT_ATTRIBUTES) {
            DEFAULT_ATTRIBUTES.to_vec()
        } else {
            vec![]
        };
        let options = Options {
            attributes_only: matches.is_present(OPT_ATTRIBUTES_ONLY),
            copy_contents: matches.is_present(OPT_COPY_CONTENTS),
            copy_mode: CopyMode::from_matches(matches),
            dereference: matches.is_present(OPT_DEREFERENCE),
            one_file_system: matches.is_present(OPT_ONE_FILE_SYSTEM),
            overwrite: OverwriteMode::from_matches(matches),
            parents: matches.is_present(OPT_PARENTS),
            backup_suffix: matches.value_of(OPT_SUFFIX).unwrap().to_string(),
            update: matches.is_present(OPT_UPDATE),
            verbose: matches.is_present(OPT_VERBOSE),
            reflink: matches.is_present(OPT_REFLINK),
            reflink_mode: {
                if let Some(reflink) = matches.value_of(OPT_REFLINK) {
                    match reflink {
                        "always" => {
                            ReflinkMode::Always
                        },
                        "auto" => {
                            ReflinkMode::Auto
                        },
                        value => {
                            return Err(Error::InvalidArgument(format!("invalid argument '{}' for \'reflink\'", value)))
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

    if paths.len() < 1 {
        // No files specified
        return Err("missing file operand".into());
    }

    // Return an error if the user requested to copy more than one
    // file source to a file target
    if options.no_target_dir && !options.target_dir.is_some() && paths.len() > 2 {
        return Err(format!("extra operand {:?}", paths[2]).into());
    }

    let (sources, target) = match options.target_dir {
        Some(ref target) => {
            // All path arges are sources, and the target dir was
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

    Ok((sources, target))
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

    let mut non_fatal_errors = false;
    let mut seen_sources = HashSet::with_capacity(sources.len());

    for source in sources {
        if seen_sources.contains(source) {
            show_warning!("source '{}' specified more than once", source.display());

        } else if let Err(error) = copy_source(source, target, &target_type, options) {
            show_error!("{}", error);
            match error {
                Error::Skipped(_) => (),
                _ => non_fatal_errors = true,
            }
        }
        seen_sources.insert(source);
    }

    if non_fatal_errors {
        Err(Error::NotAllFilesCopied)
    } else {
        Ok(())
    }
}


fn construct_dest_path(source_path: &Path, target: &Target, target_type: &TargetType, options: &Options)
                       -> CopyResult<PathBuf>
{
    if options.no_target_dir && target.is_dir() {
        return Err(format!("cannot overwrite directory '{}' with non-directory", target.display()).into())
    }

    Ok(match *target_type {
        TargetType::Directory => {
            let root = source_path.parent().unwrap_or(source_path);
            localize_to_target(root, source_path, target)?
        },
        TargetType::File => target.to_path_buf(),
    })
}

fn copy_source(source: &Source, target: &Target, target_type: &TargetType, options: &Options)
               -> CopyResult<()>
{
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


/// Read the contents of the directory `root` and recursively copy the
/// contents to `target`.
///
/// Any errors encounted copying files in the tree will be logged but
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

    for path in WalkDir::new(root) {
        let path = or_continue!(or_continue!(path).path().canonicalize());
        let local_to_root_parent = match root_parent {
            Some(parent) => or_continue!(path.strip_prefix(&parent)).to_path_buf(),
            None         => path.clone(),
        };

        let local_to_target = target.join(&local_to_root_parent);

        if path.is_dir() && !local_to_target.exists() {
            or_continue!(fs::create_dir_all(local_to_target.clone()));
        } else if !path.is_dir() {
            copy_file(path.as_path(), local_to_target.as_path(), options)?;
        }
    }

    Ok(())
}


impl OverwriteMode {
    fn verify(&self, path: &Path) -> CopyResult<()> {
        match *self {
            OverwriteMode::NoClobber => {
                Err(Error::Skipped(format!("Not overwriting {} because of option '{}'", path.display(), OPT_NO_CLOBBER)))
            },
            OverwriteMode::Interactive(_) => {
                if prompt_yes!("{}: overwrite {}? ", executable!(), path.display()) {
                    Ok(())
                } else {
                    Err(Error::Skipped(format!("Not overwriting {} at user request", path.display())))
                }
            },
            OverwriteMode::Clobber(_) => Ok(()),
        }
    }
}


fn copy_attribute(source: &Path, dest: &Path, attribute: &Attribute) -> CopyResult<()> {
    let context = &*format!("'{}' -> '{}'", source.display().to_string(), dest.display());
    Ok(match *attribute {
        #[cfg(unix)]
        Attribute::Mode => {
            let mode = fs::metadata(source).context(context)?.permissions().mode();
            let mut dest_metadata = fs::metadata(source).context(context)?.permissions();
            dest_metadata.set_mode(mode);
        },
        Attribute::Ownership => {
            let metadata = fs::metadata(source).context(context)?;
            fs::set_permissions(dest, metadata.permissions()).context(context)?;
        },
        Attribute::Timestamps => return Err(Error::NotImplemented("preserving timestamp not implemented".to_string())),
        Attribute::Context    => return Err(Error::NotImplemented("preserving context not implemented".to_string())),
        Attribute::Links      => return Err(Error::NotImplemented("preserving links not implemented".to_string())),
        Attribute::Xattr      => return Err(Error::NotImplemented("preserving xattr not implemented".to_string())),
        Attribute::All        => return Err(Error::NotImplemented("preserving a not implemented".to_string())),
    })
}

#[cfg(not(windows))]
fn symlink_file(source: &Path, dest: &Path, context: &str) -> CopyResult<()> {
    Ok(std::os::unix::fs::symlink(source, dest).context(context)?)
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
        },
        OverwriteMode::Clobber(ClobberMode::RemoveDestination) => {
            fs::remove_file(dest)?;
        },
        _ => (),
    };

    Ok(())
}

/// Copy the a file from `source` to `dest`. No path manipulation is
/// done on either `source` or `dest`, the are used as provieded.
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
                let src_metadata = fs::metadata(source.clone())?;
                let dest_metadata = fs::metadata(dest.clone())?;

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
        return Err(format!("--reflink is only supported on linux").into());

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
                        return Err(format!("failed to clone {:?} from {:?}: {}", source, dest, std::io::Error::last_os_error()).into());
                    } else {
                        return Ok(())
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
    } else {
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
        (&TargetType::File, true) => {
            Err(format!("cannot overwrite directory '{}' with non-directory", target.display()).into())
        }
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
    let pathbuf1 = try!(canonicalize(p1, CanonicalizeMode::Normal));
    let pathbuf2 = try!(canonicalize(p2, CanonicalizeMode::Normal));

    Ok(pathbuf1 == pathbuf2)
}


#[test]
fn test_cp_localize_to_target() {
    assert!(localize_to_target(
        &Path::new("a/source/"),
        &Path::new("a/source/c.txt"),
        &Path::new("target/")
    ).unwrap() == Path::new("target/c.txt"))
}
