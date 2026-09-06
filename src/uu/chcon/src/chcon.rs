// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) RFILE RDONLY CLOEXEC fgetfilecon fsetfilecon

#![cfg(any(target_os = "linux", target_os = "android"))]
#![allow(clippy::upper_case_acronyms)]

use clap::builder::ValueParser;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::translate;
use uucore::{display::Quotable, format_usage, show_error, show_warning};

use clap::{Arg, ArgAction, ArgMatches, Command};
use rustix::fs::{Mode, OFlags, openat};
use selinux::{OpaqueSecurityContext, SecurityContext};

use uucore::safe_traversal::{DirFd, Metadata as TraversalMetadata, SymlinkBehavior};

use core::ffi::CStr;
use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::{CString, OsStr, OsString};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::{fs, io};

mod errors;

use errors::{Error, Result, report_full_error};

pub mod options {
    pub static HELP: &str = "help";
    pub static VERBOSE: &str = "verbose";

    pub static REFERENCE: &str = "reference";

    pub static USER: &str = "user";
    pub static ROLE: &str = "role";
    pub static TYPE: &str = "type";
    pub static RANGE: &str = "range";

    pub static RECURSIVE: &str = "recursive";

    pub mod sym_links {
        pub static FOLLOW_ARG_DIR_SYM_LINK: &str = "follow-arg-dir-sym-link";
        pub static FOLLOW_DIR_SYM_LINKS: &str = "follow-dir-sym-links";
        pub static NO_FOLLOW_SYM_LINKS: &str = "no-follow-sym-links";
    }

    pub mod dereference {
        pub static DEREFERENCE: &str = "dereference";
        pub static NO_DEREFERENCE: &str = "no-dereference";
    }

    pub mod preserve_root {
        pub static PRESERVE_ROOT: &str = "preserve-root";
        pub static NO_PRESERVE_ROOT: &str = "no-preserve-root";
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let options = match parse_command_line(&matches) {
        Ok(r) => r,
        Err(r) => {
            if let Error::CommandLine(r) = r {
                return Err(r.into());
            }

            return Err(UUsageError::new(libc::EXIT_FAILURE, format!("{r}.\n")));
        }
    };

    let context = match &options.mode {
        CommandLineMode::ReferenceBased { reference } => {
            let result = match SecurityContext::of_path(reference, true, false) {
                Ok(Some(context)) => Ok(context),

                Ok(None) => {
                    let err = io::Error::from_raw_os_error(libc::ENODATA);
                    Err(Error::from_io1(
                        translate!("chcon-op-getting-security-context"),
                        reference,
                        err,
                    ))
                }

                Err(r) => Err(Error::from_selinux(
                    translate!("chcon-op-getting-security-context"),
                    r,
                )),
            };

            match result {
                Err(r) => {
                    return Err(USimpleError::new(
                        libc::EXIT_FAILURE,
                        format!("{}.", report_full_error(&r)),
                    ));
                }

                Ok(file_context) => SELinuxSecurityContext::File(file_context),
            }
        }

        CommandLineMode::ContextBased { context } => {
            let c_context = match os_str_to_c_string(context) {
                Ok(context) => context,

                Err(_r) => {
                    return Err(USimpleError::new(
                        libc::EXIT_FAILURE,
                        translate!("chcon-error-invalid-context", "context" => context.quote()),
                    ));
                }
            };

            if SecurityContext::from_c_str(&c_context, false).check() == Some(false) {
                return Err(USimpleError::new(
                    libc::EXIT_FAILURE,
                    translate!("chcon-error-invalid-context", "context" => context.quote()),
                ));
            }

            SELinuxSecurityContext::String(Some(c_context))
        }

        CommandLineMode::Custom { .. } => SELinuxSecurityContext::String(None),
    };

    let root_id = if options.preserve_root && options.recursive_mode.is_recursive() {
        match root_identity() {
            Ok(r) => Some(r),

            Err(r) => {
                return Err(USimpleError::new(
                    libc::EXIT_FAILURE,
                    format!("{}.", report_full_error(&r)),
                ));
            }
        }
    } else {
        None
    };

    let results = relabel_all(&options, &context, root_id);
    if results.is_empty() {
        return Ok(());
    }

    for result in &results {
        show_error!("{}.", report_full_error(result));
    }
    Err(libc::EXIT_FAILURE.into())
}

pub fn uu_app() -> Command {
    let cmd = Command::new("chcon")
        .version(uucore::crate_version!())
        .about(translate!("chcon-about"))
        .override_usage(format_usage(&translate!("chcon-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        .args_override_self(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::dereference::DEREFERENCE)
                .long(options::dereference::DEREFERENCE)
                .overrides_with(options::dereference::NO_DEREFERENCE)
                .help(translate!("chcon-help-dereference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::dereference::NO_DEREFERENCE)
                .short('h')
                .long(options::dereference::NO_DEREFERENCE)
                .help(translate!("chcon-help-no-dereference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("help")
                .long("help")
                .help(translate!("help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::preserve_root::PRESERVE_ROOT)
                .long(options::preserve_root::PRESERVE_ROOT)
                .overrides_with(options::preserve_root::NO_PRESERVE_ROOT)
                .help(translate!("chcon-help-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::preserve_root::NO_PRESERVE_ROOT)
                .long(options::preserve_root::NO_PRESERVE_ROOT)
                .help(translate!("chcon-help-no-preserve-root"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long(options::REFERENCE)
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath)
                .conflicts_with_all([options::USER, options::ROLE, options::TYPE, options::RANGE])
                .help(translate!("chcon-help-reference"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::USER)
                .short('u')
                .long(options::USER)
                .value_name("USER")
                .value_hint(clap::ValueHint::Username)
                .help(translate!("chcon-help-user"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::ROLE)
                .short('r')
                .long(options::ROLE)
                .value_name("ROLE")
                .help(translate!("chcon-help-role"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::TYPE)
                .short('t')
                .long(options::TYPE)
                .value_name("TYPE")
                .help(translate!("chcon-help-type"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::RANGE)
                .short('l')
                .long(options::RANGE)
                .value_name("RANGE")
                .help(translate!("chcon-help-range"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help(translate!("chcon-help-recursive"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sym_links::FOLLOW_ARG_DIR_SYM_LINK)
                .short('H')
                .requires(options::RECURSIVE)
                .overrides_with_all([
                    options::sym_links::FOLLOW_DIR_SYM_LINKS,
                    options::sym_links::NO_FOLLOW_SYM_LINKS,
                ])
                .help(translate!("chcon-help-follow-arg-dir-symlink"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sym_links::FOLLOW_DIR_SYM_LINKS)
                .short('L')
                .requires(options::RECURSIVE)
                .overrides_with_all([
                    options::sym_links::FOLLOW_ARG_DIR_SYM_LINK,
                    options::sym_links::NO_FOLLOW_SYM_LINKS,
                ])
                .help(translate!("chcon-help-follow-dir-symlinks"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sym_links::NO_FOLLOW_SYM_LINKS)
                .short('P')
                .requires(options::RECURSIVE)
                .overrides_with_all([
                    options::sym_links::FOLLOW_ARG_DIR_SYM_LINK,
                    options::sym_links::FOLLOW_DIR_SYM_LINKS,
                ])
                .help(translate!("chcon-help-no-follow-symlinks"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help(translate!("chcon-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("FILE")
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .num_args(1..)
                .value_parser(ValueParser::os_string()),
        )
}

#[derive(Debug)]
struct Options {
    verbose: bool,
    preserve_root: bool,
    recursive_mode: RecursiveMode,
    affect_symlink_referent: bool,
    mode: CommandLineMode,
    files: Vec<PathBuf>,
}

fn parse_command_line(matches: &ArgMatches) -> Result<Options> {
    let verbose = matches.get_flag(options::VERBOSE);

    let (recursive_mode, affect_symlink_referent) = if matches.get_flag(options::RECURSIVE) {
        if matches.get_flag(options::sym_links::FOLLOW_DIR_SYM_LINKS) {
            if matches.get_flag(options::dereference::NO_DEREFERENCE) {
                return Err(Error::ArgumentsMismatch(translate!(
                    "chcon-error-recursive-no-dereference-require-p"
                )));
            }

            (RecursiveMode::RecursiveAndFollowAllDirSymLinks, true)
        } else if matches.get_flag(options::sym_links::FOLLOW_ARG_DIR_SYM_LINK) {
            if matches.get_flag(options::dereference::NO_DEREFERENCE) {
                return Err(Error::ArgumentsMismatch(translate!(
                    "chcon-error-recursive-no-dereference-require-p"
                )));
            }

            (RecursiveMode::RecursiveAndFollowArgDirSymLinks, true)
        } else {
            if matches.get_flag(options::dereference::DEREFERENCE) {
                return Err(Error::ArgumentsMismatch(translate!(
                    "chcon-error-recursive-dereference-require-h-or-l"
                )));
            }

            (RecursiveMode::RecursiveButDoNotFollowSymLinks, false)
        }
    } else {
        let no_dereference = matches.get_flag(options::dereference::NO_DEREFERENCE);
        (RecursiveMode::NotRecursive, !no_dereference)
    };

    // By default, do not preserve root.
    let preserve_root = matches.get_flag(options::preserve_root::PRESERVE_ROOT);

    let mut files = matches.get_many::<OsString>("FILE").unwrap_or_default();

    let mode = if let Some(path) = matches.get_one::<OsString>(options::REFERENCE) {
        CommandLineMode::ReferenceBased {
            reference: PathBuf::from(path),
        }
    } else if matches.contains_id(options::USER)
        || matches.contains_id(options::ROLE)
        || matches.contains_id(options::TYPE)
        || matches.contains_id(options::RANGE)
    {
        CommandLineMode::Custom {
            user: matches.get_one::<OsString>(options::USER).map(Into::into),
            role: matches.get_one::<OsString>(options::ROLE).map(Into::into),
            the_type: matches.get_one::<OsString>(options::TYPE).map(Into::into),
            range: matches.get_one::<OsString>(options::RANGE).map(Into::into),
        }
    } else if let Some(context) = files.next() {
        CommandLineMode::ContextBased {
            context: context.into(),
        }
    } else {
        return Err(Error::MissingContext);
    };

    let files: Vec<_> = files.map(PathBuf::from).collect();
    if files.is_empty() {
        return Err(Error::MissingFiles);
    }

    Ok(Options {
        verbose,
        preserve_root,
        recursive_mode,
        affect_symlink_referent,
        mode,
        files,
    })
}

#[derive(Debug, Copy, Clone)]
enum RecursiveMode {
    NotRecursive,
    /// Do not traverse any symbolic links.
    RecursiveButDoNotFollowSymLinks,
    /// Traverse every symbolic link to a directory encountered.
    RecursiveAndFollowAllDirSymLinks,
    /// If a command line argument is a symbolic link to a directory, traverse it.
    RecursiveAndFollowArgDirSymLinks,
}

impl RecursiveMode {
    fn is_recursive(self) -> bool {
        match self {
            Self::NotRecursive => false,

            Self::RecursiveButDoNotFollowSymLinks
            | Self::RecursiveAndFollowAllDirSymLinks
            | Self::RecursiveAndFollowArgDirSymLinks => true,
        }
    }
}

#[derive(Debug)]
enum CommandLineMode {
    ReferenceBased {
        reference: PathBuf,
    },
    ContextBased {
        context: OsString,
    },
    Custom {
        user: Option<OsString>,
        role: Option<OsString>,
        the_type: Option<OsString>,
        range: Option<OsString>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DeviceAndINode {
    device_id: u64,
    inode: u64,
}

#[cfg(unix)]
impl From<fs::Metadata> for DeviceAndINode {
    fn from(md: fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;

        Self {
            device_id: md.dev(),
            inode: md.ino(),
        }
    }
}

impl From<&TraversalMetadata> for DeviceAndINode {
    fn from(md: &TraversalMetadata) -> Self {
        let info = md.file_info();
        Self {
            device_id: info.device(),
            inode: info.inode(),
        }
    }
}

impl TryFrom<&libc::stat> for DeviceAndINode {
    type Error = Error;

    #[allow(clippy::useless_conversion)]
    fn try_from(st: &libc::stat) -> Result<Self> {
        let device_id = u64::try_from(st.st_dev).map_err(|_r| Error::OutOfRange)?;
        let inode = u64::try_from(st.st_ino).map_err(|_r| Error::OutOfRange)?;
        Ok(Self { device_id, inode })
    }
}

/// Whether a symlink at this position should be resolved before it is examined.
///
/// `-P` never resolves, `-L` always does, and `-H` resolves only the operands
/// named on the command line.
fn follows_symlinks(mode: RecursiveMode, top_level: bool) -> SymlinkBehavior {
    match mode {
        RecursiveMode::RecursiveAndFollowAllDirSymLinks => SymlinkBehavior::Follow,
        RecursiveMode::RecursiveAndFollowArgDirSymLinks => top_level.into(),
        RecursiveMode::NotRecursive | RecursiveMode::RecursiveButDoNotFollowSymLinks => {
            SymlinkBehavior::NoFollow
        }
    }
}

/// Whether a directory's contents were fully dealt with.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Descent {
    Complete,
    Abandoned,
}

/// Whether revisiting a directory indicates real damage rather than an ordinary
/// consequence of following symlinks.
fn cycle_is_alarming(mode: RecursiveMode, top_level: bool) -> bool {
    match mode {
        RecursiveMode::RecursiveAndFollowAllDirSymLinks => false,
        RecursiveMode::RecursiveAndFollowArgDirSymLinks => !top_level,
        RecursiveMode::NotRecursive | RecursiveMode::RecursiveButDoNotFollowSymLinks => true,
    }
}

fn relabel_all(
    options: &Options,
    context: &SELinuxSecurityContext,
    root_id: Option<DeviceAndINode>,
) -> Vec<Error> {
    let mut errors = Vec::default();

    // Operands are named relative to where we were started, so anchor them to a
    // descriptor for that directory rather than re-resolving each path.
    let start = match DirFd::open(Path::new("."), SymlinkBehavior::Follow) {
        Ok(dir) => dir,
        Err(err) => {
            errors.push(Error::from_io1(translate!("chcon-op-accessing"), ".", err));
            return errors;
        }
    };

    let mut walk = Walk {
        options,
        context,
        root_id,
        // Directories reached on the way down, so a symlink loop is noticed
        // instead of followed forever.
        open_ancestors: HashSet::new(),
        errors,
    };

    for operand in &options.files {
        let behavior = follows_symlinks(options.recursive_mode, true);
        walk.visit(&start, operand.as_os_str(), operand, behavior, true);
    }

    walk.errors
}

/// State carried down the walk. Directories are relabelled on the way back up,
/// so a context that forbids reading one is applied only once we are done with it.
struct Walk<'a> {
    options: &'a Options,
    context: &'a SELinuxSecurityContext<'a>,
    root_id: Option<DeviceAndINode>,
    open_ancestors: HashSet<DeviceAndINode>,
    errors: Vec<Error>,
}

impl Walk<'_> {
    fn fail(&mut self, op: String, display: &Path, err: io::Error) {
        self.errors.push(Error::from_io1(op, display, err));
    }

    fn reject(&mut self, op: String, display: &Path, kind: io::ErrorKind) {
        self.errors.push(Error::from_io1(op, display, kind.into()));
    }

    fn visit(
        &mut self,
        parent: &DirFd,
        name: &OsStr,
        display: &Path,
        behavior: SymlinkBehavior,
        top_level: bool,
    ) {
        let meta = match parent.metadata_at(name, behavior) {
            Ok(meta) => meta,
            Err(err) => return self.fail(translate!("chcon-op-accessing"), display, err),
        };
        let id = DeviceAndINode::from(&meta);

        let recursive = self.options.recursive_mode.is_recursive();

        // `--preserve-root` guards recursion only: naming `/` outright is still
        // allowed, it is descending into it that is refused.
        if recursive && self.root_id == Some(id) {
            warn_recursive_root(display);
            return self.reject(
                translate!("chcon-op-modifying-root-path"),
                display,
                io::ErrorKind::PermissionDenied,
            );
        }

        // A symlink is only descended when this position resolves symlinks; the
        // metadata above already reflects that choice.
        let descended = if recursive
            && meta.is_dir()
            && (behavior.should_follow() || !meta.file_type().is_symlink())
        {
            self.descend(parent, name, display, behavior, id, top_level)
        } else {
            Descent::Complete
        };

        // A directory we could not read is reported, not relabelled: its context
        // would otherwise change while its contents were left untouched.
        if descended == Descent::Complete {
            self.relabel(parent, name, display);
        }
    }

    fn descend(
        &mut self,
        parent: &DirFd,
        name: &OsStr,
        display: &Path,
        behavior: SymlinkBehavior,
        id: DeviceAndINode,
        top_level: bool,
    ) -> Descent {
        if !self.open_ancestors.insert(id) {
            // Following symlinks is expected to reach the same directory twice, so
            // only a walk that stays on the physical tree treats this as damage.
            if !cycle_is_alarming(self.options.recursive_mode, top_level) {
                return Descent::Complete;
            }
            warn_directory_cycle(display);
            self.reject(
                translate!("chcon-op-reading-cyclic-directory"),
                display,
                io::ErrorKind::InvalidData,
            );
            return Descent::Abandoned;
        }

        let outcome = match parent.open_subdir(name, behavior) {
            Ok(dir) => match dir.read_dir() {
                Ok(children) => {
                    let inner = follows_symlinks(self.options.recursive_mode, false);
                    for child in children {
                        self.visit(&dir, &child, &display.join(&child), inner, false);
                    }
                    Descent::Complete
                }
                Err(err) => {
                    self.fail(translate!("chcon-op-reading-directory"), display, err);
                    Descent::Abandoned
                }
            },
            Err(err) => {
                self.fail(translate!("chcon-op-reading-directory"), display, err);
                Descent::Abandoned
            }
        };

        // Sibling subtrees may legitimately reach this directory again, so only
        // the path currently being walked counts as a cycle.
        self.open_ancestors.remove(&id);
        outcome
    }

    fn relabel(&mut self, parent: &DirFd, name: &OsStr, display: &Path) {
        if self.options.verbose {
            println!(
                "{}",
                translate!("chcon-verbose-changing-context", "util_name" => "chcon", "file" => display.quote())
            );
        }

        if let Err(err) = apply_context(
            self.options,
            self.context,
            parent.as_fd(),
            Path::new(name),
            display,
        ) {
            self.errors.push(err);
        }
    }
}

fn open_target_fd(
    traversal_dir_fd: BorrowedFd<'_>,
    target_path: &Path,
    affect_symlink_referent: bool,
    display_path: &Path,
) -> Result<OwnedFd> {
    // Anchor the open to the traversal directory fd so a concurrent rename or
    // symlink swap cannot redirect the relabel off-tree. O_PATH hands back the
    // entry itself without opening it for I/O: it never blocks on a FIFO and
    // does not require read permission, yet the SELinux get/set still work
    // through /proc/self/fd (see apply_context). O_NOFOLLOW keeps us on
    // the symlink itself unless we were asked to act on its referent.
    let mut flags = OFlags::PATH | OFlags::CLOEXEC;
    if !affect_symlink_referent {
        flags |= OFlags::NOFOLLOW;
    }

    openat(traversal_dir_fd, target_path, flags, Mode::empty())
        .map_err(io::Error::from)
        .map_err(|err| Error::from_io1(translate!("chcon-op-accessing"), display_path, err))
}

fn apply_context(
    options: &Options,
    context: &SELinuxSecurityContext,
    traversal_dir_fd: BorrowedFd<'_>,
    target_path: &Path,
    display_path: &Path,
) -> Result<()> {
    type SetValueProc = fn(&OpaqueSecurityContext, &CStr) -> selinux::errors::Result<()>;

    let target_fd = open_target_fd(
        traversal_dir_fd,
        target_path,
        options.affect_symlink_referent,
        display_path,
    )?;

    // The fd is O_PATH, so fgetfilecon/fsetfilecon would fail with EBADF. Reach
    // the same inode through its /proc/self/fd entry and always dereference it:
    // the open above already encoded the follow/no-follow choice, so this magic
    // symlink resolves to exactly the inode we anchored. `target_fd` must stay
    // alive for as long as this path is used.
    let target = PathBuf::from(format!("/proc/self/fd/{}", target_fd.as_raw_fd()));

    match &options.mode {
        CommandLineMode::Custom {
            user,
            role,
            the_type,
            range,
        } => {
            let err0 = || -> Result<()> {
                // Setting only part of a context needs an existing one to merge into.
                // When the file carries none there is nothing sensible to assume, so
                // report it rather than invent a default.
                let op = translate!("chcon-op-applying-partial-context");
                let err = io::ErrorKind::InvalidInput.into();
                Err(Error::from_io1(op, display_path, err))
            };

            let file_context = match SecurityContext::of_path(&target, true, false) {
                Ok(Some(context)) => context,

                Ok(None) => return err0(),
                Err(r) => {
                    return Err(Error::from_selinux(
                        translate!("chcon-op-getting-security-context"),
                        r,
                    ));
                }
            };

            let c_file_context = match file_context.to_c_string() {
                Ok(Some(context)) => context,

                Ok(None) => return err0(),
                Err(r) => {
                    return Err(Error::from_selinux(
                        translate!("chcon-op-getting-security-context"),
                        r,
                    ));
                }
            };

            let se_context =
                OpaqueSecurityContext::from_c_str(c_file_context.as_ref()).map_err(|_r| {
                    let err = io::ErrorKind::InvalidInput.into();
                    Error::from_io1(
                        translate!("chcon-op-creating-security-context"),
                        display_path,
                        err,
                    )
                })?;

            let list: &[(&Option<OsString>, SetValueProc)] = &[
                (user, OpaqueSecurityContext::set_user),
                (role, OpaqueSecurityContext::set_role),
                (the_type, OpaqueSecurityContext::set_type),
                (range, OpaqueSecurityContext::set_range),
            ];

            for (new_value, set_value_proc) in list {
                if let Some(new_value) = new_value {
                    let c_new_value = os_str_to_c_string(new_value).map_err(|_r| {
                        let err = io::ErrorKind::InvalidInput.into();
                        Error::from_io1(
                            translate!("chcon-op-creating-security-context"),
                            display_path,
                            err,
                        )
                    })?;

                    set_value_proc(&se_context, &c_new_value).map_err(|r| {
                        Error::from_selinux(translate!("chcon-op-setting-security-context-user"), r)
                    })?;
                }
            }

            let context_string = se_context.to_c_string().map_err(|r| {
                Error::from_selinux(translate!("chcon-op-getting-security-context"), r)
            })?;

            if c_file_context.as_ref().to_bytes() == context_string.as_ref().to_bytes() {
                Ok(()) // Nothing to change.
            } else {
                SecurityContext::from_c_str(&context_string, false)
                    .set_for_path(&target, true, false)
                    .map_err(|r| {
                        Error::from_selinux(translate!("chcon-op-setting-security-context"), r)
                    })
            }
        }

        CommandLineMode::ReferenceBased { .. } | CommandLineMode::ContextBased { .. } => {
            if let Some(c_context) = context.to_c_string()? {
                SecurityContext::from_c_str(c_context.as_ref(), false)
                    .set_for_path(&target, true, false)
                    .map_err(|r| {
                        Error::from_selinux(translate!("chcon-op-setting-security-context"), r)
                    })
            } else {
                let err = io::ErrorKind::InvalidInput.into();
                Err(Error::from_io1(
                    translate!("chcon-op-setting-security-context"),
                    display_path,
                    err,
                ))
            }
        }
    }
}

#[cfg(unix)]
pub(crate) fn os_str_to_c_string(s: &OsStr) -> Result<CString> {
    use std::os::unix::ffi::OsStrExt;

    CString::new(s.as_bytes())
        .map_err(|_r| Error::from_io("CString::new()", io::ErrorKind::InvalidInput.into()))
}

/// Identify the root directory by device and inode, without following it.
#[cfg(unix)]
fn root_identity() -> Result<DeviceAndINode> {
    fs::symlink_metadata("/")
        .map(DeviceAndINode::from)
        .map_err(|r| Error::from_io1("std::fs::symlink_metadata", "/", r))
}

fn warn_recursive_root(dir_name: &Path) {
    if dir_name.as_os_str() == "/" {
        show_warning!(
            "{}",
            translate!("chcon-warning-dangerous-recursive-root", "option" => options::preserve_root::NO_PRESERVE_ROOT)
        );
    } else {
        show_warning!(
            "{}",
            translate!("chcon-warning-dangerous-recursive-dir", "dir" => dir_name.quote(), "option" => options::preserve_root::NO_PRESERVE_ROOT)
        );
    }
}

fn warn_directory_cycle(file_name: &Path) {
    show_warning!(
        "{}",
        translate!("chcon-warning-circular-directory", "file" => file_name.quote())
    );
}

#[derive(Debug)]
enum SELinuxSecurityContext<'t> {
    File(SecurityContext<'t>),
    String(Option<CString>),
}

impl SELinuxSecurityContext<'_> {
    fn to_c_string(&self) -> Result<Option<Cow<'_, CStr>>> {
        match self {
            Self::File(context) => context
                .to_c_string()
                .map_err(|r| Error::from_selinux("SELinuxSecurityContext::to_c_string()", r)),

            Self::String(context) => Ok(context.as_deref().map(Cow::Borrowed)),
        }
    }
}
