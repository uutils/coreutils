#![allow(clippy::missing_safety_doc)]

// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Joshua S. Miller <jsmiller@uchicago.edu>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) ficlone linkgs lstat nlink nlinks pathbuf reflink strs xattrs

pub mod app;

#[cfg(target_os = "linux")]
#[macro_use]
extern crate ioctl_sys;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate uucore;

use app::*;
#[cfg(windows)]
use winapi::um::fileapi::CreateFileW;
#[cfg(windows)]
use winapi::um::fileapi::GetFileInformationByHandle;

use std::borrow::Cow;

use clap::ArgMatches;
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
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf, StripPrefixError};
use std::str::FromStr;
use std::string::ToString;
use uucore::backup_control::{self, BackupMode};
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
pub type SourceSlice = Path;
pub type Target = PathBuf;
pub type TargetSlice = Path;

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

/// Possible arguments for `--reflink`.
#[derive(Copy, Clone, Eq, PartialEq)]
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

pub enum CopyMode {
    Link,
    SymLink,
    Sparse,
    Copy,
    Update,
    AttrOnly,
}

/// Re-usable, extensible copy options
#[allow(dead_code)]
pub struct Options {
    attributes_only: bool,
    backup: BackupMode,
    copy_contents: bool,
    copy_mode: CopyMode,
    dereference: bool,
    no_dereference: bool,
    no_target_dir: bool,
    one_file_system: bool,
    overwrite: OverwriteMode,
    parents: bool,
    strip_trailing_slashes: bool,
    reflink_mode: ReflinkMode,
    preserve_attributes: Vec<Attribute>,
    recursive: bool,
    backup_suffix: String,
    target_dir: Option<String>,
    update: bool,
    verbose: bool,
}

static LONG_HELP: &str = "";
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

static DEFAULT_ATTRIBUTES: &[Attribute] = &[
    #[cfg(unix)]
    Attribute::Mode,
    Attribute::Ownership,
    Attribute::Timestamps,
];

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = get_app(executable!())
        .usage(&usage[..])
        .after_help(&*format!(
            "{}\n{}",
            LONG_HELP,
            backup_control::BACKUP_CONTROL_LONG_HELP
        ))
        .get_matches_from(args);

    let options = crash_if_err!(EXIT_ERR, Options::from_matches(&matches));

    if options.overwrite == OverwriteMode::NoClobber && options.backup != BackupMode::NoBackup {
        show_usage_error!("options --backup and --no-clobber are mutually exclusive");
        return 1;
    }

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
    use Attribute::*;

    #[cfg(target_os = "windows")]
    let attr = vec![Ownership, Timestamps, Context, Xattr, Links];

    #[cfg(not(target_os = "windows"))]
    let mut attr = vec![Ownership, Timestamps, Context, Xattr, Links];

    #[cfg(unix)]
    attr.insert(0, Mode);

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

        let backup_mode = backup_control::determine_backup_mode(
            matches.is_present(OPT_BACKUP_NO_ARG) || matches.is_present(OPT_BACKUP),
            matches.value_of(OPT_BACKUP),
        );
        let backup_suffix = backup_control::determine_backup_suffix(matches.value_of(OPT_SUFFIX));

        let overwrite = OverwriteMode::from_matches(matches);

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
            parents: matches.is_present(OPT_PARENTS),
            update: matches.is_present(OPT_UPDATE),
            verbose: matches.is_present(OPT_VERBOSE),
            strip_trailing_slashes: matches.is_present(OPT_STRIP_TRAILING_SLASHES),
            reflink_mode: {
                if let Some(reflink) = matches.value_of(OPT_REFLINK) {
                    match reflink {
                        "always" => ReflinkMode::Always,
                        "auto" => ReflinkMode::Auto,
                        "never" => ReflinkMode::Never,
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
            backup: backup_mode,
            backup_suffix,
            overwrite,
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
    fn determine(sources: &[Source], target: &TargetSlice) -> TargetType {
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

    let target = match options.target_dir {
        Some(ref target) => {
            // All path args are sources, and the target dir was
            // specified separately
            PathBuf::from(target)
        }
        None => {
            // If there was no explicit target-dir, then use the last
            // path_arg
            paths.pop().unwrap()
        }
    };

    if options.strip_trailing_slashes {
        for source in paths.iter_mut() {
            *source = source.components().as_path().to_owned()
        }
    }

    Ok((paths, target))
}

fn preserve_hardlinks(
    hard_links: &mut Vec<(String, u64)>,
    source: &std::path::Path,
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
fn copy(sources: &[Source], target: &TargetSlice, options: &Options) -> CopyResult<()> {
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
    target: &TargetSlice,
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
    source: &SourceSlice,
    target: &TargetSlice,
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
fn copy_directory(root: &Path, target: &TargetSlice, options: &Options) -> CopyResult<()> {
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
#[allow(clippy::unnecessary_wraps)] // needed for windows version
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

/// Implements a simple backup copy for the destination file.
/// TODO: for the backup, should this function be replaced by `copy_file(...)`?
fn backup_dest(dest: &Path, backup_path: &Path) -> CopyResult<PathBuf> {
    fs::copy(dest, &backup_path)?;
    Ok(backup_path.into())
}

fn handle_existing_dest(source: &Path, dest: &Path, options: &Options) -> CopyResult<()> {
    if paths_refer_to_same_file(source, dest)? {
        return Err(format!("{}: same file", context_for(source, dest)).into());
    }

    options.overwrite.verify(dest)?;

    let backup_path = backup_control::get_backup_path(options.backup, dest, &options.backup_suffix);
    if let Some(backup_path) = backup_path {
        backup_dest(dest, &backup_path)?;
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

/// Copy the file from `source` to `dest` either using the normal `fs::copy` or a
/// copy-on-write scheme if --reflink is specified and the filesystem supports it.
fn copy_helper(source: &Path, dest: &Path, options: &Options) -> CopyResult<()> {
    if options.reflink_mode != ReflinkMode::Never {
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err("--reflink is only supported on linux and macOS"
            .to_string()
            .into());

        #[cfg(target_os = "macos")]
        copy_on_write_macos(source, dest, options.reflink_mode)?;

        #[cfg(target_os = "linux")]
        copy_on_write_linux(source, dest, options.reflink_mode)?;
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

/// Copies `source` to `dest` using copy-on-write if possible.
#[cfg(target_os = "linux")]
fn copy_on_write_linux(source: &Path, dest: &Path, mode: ReflinkMode) -> CopyResult<()> {
    debug_assert!(mode != ReflinkMode::Never);

    let src_file = File::open(source).context(&*context_for(source, dest))?;
    let dst_file = OpenOptions::new()
        .write(true)
        .truncate(false)
        .create(true)
        .open(dest)
        .context(&*context_for(source, dest))?;
    match mode {
        ReflinkMode::Always => unsafe {
            let result = ficlone(dst_file.as_raw_fd(), src_file.as_raw_fd() as *const i32);
            if result != 0 {
                Err(format!(
                    "failed to clone {:?} from {:?}: {}",
                    source,
                    dest,
                    std::io::Error::last_os_error()
                )
                .into())
            } else {
                Ok(())
            }
        },
        ReflinkMode::Auto => unsafe {
            let result = ficlone(dst_file.as_raw_fd(), src_file.as_raw_fd() as *const i32);
            if result != 0 {
                fs::copy(source, dest).context(&*context_for(source, dest))?;
            }
            Ok(())
        },
        ReflinkMode::Never => unreachable!(),
    }
}

/// Copies `source` to `dest` using copy-on-write if possible.
#[cfg(target_os = "macos")]
fn copy_on_write_macos(source: &Path, dest: &Path, mode: ReflinkMode) -> CopyResult<()> {
    debug_assert!(mode != ReflinkMode::Never);

    // Extract paths in a form suitable to be passed to a syscall.
    // The unwrap() is safe because they come from the command-line and so contain non nul
    // character.
    use std::os::unix::ffi::OsStrExt;
    let src = CString::new(source.as_os_str().as_bytes()).unwrap();
    let dst = CString::new(dest.as_os_str().as_bytes()).unwrap();

    // clonefile(2) was introduced in macOS 10.12 so we cannot statically link against it
    // for backward compatibility.
    let clonefile = CString::new("clonefile").unwrap();
    let raw_pfn = unsafe { libc::dlsym(libc::RTLD_NEXT, clonefile.as_ptr()) };

    let mut error = 0;
    if !raw_pfn.is_null() {
        // Call clonefile(2).
        // Safety: Casting a C function pointer to a rust function value is one of the few
        // blessed uses of `transmute()`.
        unsafe {
            let pfn: extern "C" fn(
                src: *const libc::c_char,
                dst: *const libc::c_char,
                flags: u32,
            ) -> libc::c_int = std::mem::transmute(raw_pfn);
            error = pfn(src.as_ptr(), dst.as_ptr(), 0);
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists {
                // clonefile(2) fails if the destination exists.  Remove it and try again.  Do not
                // bother to check if removal worked because we're going to try to clone again.
                let _ = fs::remove_file(dest);
                error = pfn(src.as_ptr(), dst.as_ptr(), 0);
            }
        }
    }

    if raw_pfn.is_null() || error != 0 {
        // clonefile(2) is either not supported or it errored out (possibly because the FS does not
        // support COW).
        match mode {
            ReflinkMode::Always => {
                return Err(
                    format!("failed to clone {:?} from {:?}: {}", source, dest, error).into(),
                )
            }
            ReflinkMode::Auto => fs::copy(source, dest).context(&*context_for(source, dest))?,
            ReflinkMode::Never => unreachable!(),
        };
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
