// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) RFILE RDONLY CLOEXEC ENODATA relabeler readlinkat

#![cfg(any(target_os = "linux", target_os = "android"))]
#![allow(clippy::upper_case_acronyms)]

use clap::builder::ValueParser;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::translate;
use uucore::{display::Quotable, format_usage, show_error, show_warning};

use clap::{Arg, ArgAction, ArgMatches, Command};
use rustix::fs::{FileType, Mode, OFlags, Stat, openat};
use selinux::{OpaqueSecurityContext, SecurityContext};

use uucore::safe_traversal::{DirFd, FileInfo, SymlinkBehavior, open_path_at, proc_fd_path};

use core::ffi::CStr;
use std::borrow::Cow;
use std::ffi::{CString, OsStr, OsString};
use std::io;
use std::io::Write;
use std::os::fd::{AsFd, BorrowedFd, IntoRawFd, OwnedFd};
use std::path::{Path, PathBuf};

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

    let fatal =
        |e: Error| USimpleError::new(libc::EXIT_FAILURE, format!("{}.", report_full_error(&e)));

    let target = match &options.mode {
        CommandLineMode::ReferenceBased { reference } => {
            let getting = translate!("chcon-op-getting-security-context");
            let context = match SecurityContext::of_path(reference, true, false) {
                Ok(Some(context)) => context,
                Ok(None) => {
                    let err = io::Error::from_raw_os_error(libc::ENODATA);
                    return Err(fatal(Error::from_io1(getting, reference, err)));
                }
                Err(e) => return Err(fatal(Error::from_selinux(getting, e))),
            };
            let text = context
                .to_c_string()
                .map_err(|e| fatal(Error::from_selinux(getting, e)))?
                .ok_or_else(|| {
                    let err = io::Error::from_raw_os_error(libc::ENODATA);
                    fatal(Error::from_io(
                        translate!("chcon-op-creating-security-context"),
                        err,
                    ))
                })?;
            Target::Whole(text.into_owned())
        }

        CommandLineMode::ContextBased { context } => {
            let invalid = || {
                USimpleError::new(
                    libc::EXIT_FAILURE,
                    translate!("chcon-error-invalid-context", "context" => context.quote()),
                )
            };
            let c_context = os_str_to_c_string(context).map_err(|_| invalid())?;
            if SecurityContext::from_c_str(&c_context, false).check() == Some(false) {
                return Err(invalid());
            }
            Target::Whole(c_context)
        }

        CommandLineMode::Custom {
            user,
            role,
            the_type,
            range,
        } => {
            let setters: [(Setter, _); 4] = [
                (OpaqueSecurityContext::set_user, user),
                (OpaqueSecurityContext::set_role, role),
                (OpaqueSecurityContext::set_type, the_type),
                (OpaqueSecurityContext::set_range, range),
            ];
            let mut fields = Vec::new();
            for (set, value) in setters {
                if let Some(value) = value {
                    fields.push((set, os_str_to_c_string(value).map_err(fatal)?));
                }
            }
            Target::Fields(fields)
        }
    };

    let root_id = if options.preserve_root && options.recursive_mode.is_recursive() {
        Some(root_identity().map_err(fatal)?)
    } else {
        None
    };

    let mut relabeler = Relabeler {
        options: &options,
        target,
        root_id,
        status: Status {
            verbose: options.verbose,
            failed: false,
        },
        open_dirs: Vec::new(),
    };
    for path in &options.files {
        relabeler.operand(path);
    }
    if relabeler.status.failed {
        Err(libc::EXIT_FAILURE.into())
    } else {
        Ok(())
    }
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
    follow_operand_links: bool,
    mode: CommandLineMode,
    files: Vec<PathBuf>,
}

fn parse_command_line(matches: &ArgMatches) -> Result<Options> {
    let verbose = matches.get_flag(options::VERBOSE);

    let (recursive_mode, follow_operand_links) = if matches.get_flag(options::RECURSIVE) {
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
        follow_operand_links,
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

/// Open the directory behind `handle` for reading, through the handle itself
/// rather than by name, so it is the very same directory.
fn reopen_dir(handle: &OwnedFd) -> io::Result<DirFd> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let fd = openat(handle, c".", flags, Mode::empty())?;
    DirFd::from_raw_fd(fd.into_raw_fd())
}

/// Pull the underlying OS error out of a libselinux failure, so that it can be
/// reported against the user's file name instead of the internal /proc path.
fn selinux_to_io(err: selinux::errors::Error) -> io::Error {
    use selinux::errors::Error as E;
    match err {
        E::IO { source, .. }
        | E::IO1Path { source, .. }
        | E::IO1Name { source, .. }
        | E::IO1Process { source, .. } => source,
        other => io::Error::other(other),
    }
}

/// Replaces one field of a security context, as selected by -u, -r, -t or -l.
type Setter = fn(&OpaqueSecurityContext, &CStr) -> selinux::errors::Result<()>;

/// What every file is relabeled to.
enum Target {
    /// A complete context, from CONTEXT or from --reference.
    Whole(CString),
    /// Fields to replace in each file's existing context.
    Fields(Vec<(Setter, CString)>),
}

/// Outcome of the run so far, and whether verbose output is still wanted.
struct Status {
    /// Cleared after the first failure to write the verbose output, so that
    /// the write error is only reported once.
    verbose: bool,
    failed: bool,
}

impl Status {
    fn report(&mut self, err: &Error) {
        show_error!("{}.", report_full_error(err));
        self.failed = true;
    }

    fn fail(&mut self, op: String, display: &Path, err: io::Error) {
        self.report(&Error::from_io1(op, display, err));
    }

    fn announce(&mut self, display: &Path) {
        if !self.verbose {
            return;
        }
        let line = translate!(
            "chcon-verbose-changing-context",
            "util_name" => uucore::util_name(),
            "file" => display.quote()
        );
        let mut out = io::stdout().lock();
        if let Err(e) = writeln!(out, "{line}").and_then(|()| out.flush()) {
            self.verbose = false;
            self.report(&Error::Write(e));
        }
    }
}

/// Walks the operands and relabels what they name.
struct Relabeler<'a> {
    options: &'a Options,
    target: Target,
    root_id: Option<FileInfo>,
    status: Status,
    /// Identities of the directories currently being walked, outermost first.
    open_dirs: Vec<FileInfo>,
}

impl Relabeler<'_> {
    fn operand(&mut self, path: &Path) {
        let follow = if self.options.follow_operand_links {
            SymlinkBehavior::Follow
        } else {
            SymlinkBehavior::NoFollow
        };
        let object = match open_path_at(rustix::fs::CWD, path, follow) {
            Ok(fd) => fd,
            Err(e) => return self.status.fail(translate!("chcon-op-accessing"), path, e),
        };

        if !self.options.recursive_mode.is_recursive() {
            self.label(object.as_fd(), path);
        } else if let Some(st) = self.stat(&object, path) {
            self.object(&object, &st, path, false);
        }
    }

    /// The file status of `object`, or `None` after reporting the failure.
    fn stat(&mut self, object: &OwnedFd, display: &Path) -> Option<Stat> {
        rustix::fs::fstat(object)
            .inspect_err(|e| {
                let op = translate!("chcon-op-getting-meta-data");
                self.status.fail(op, display, (*e).into());
            })
            .ok()
    }

    /// Handle an object reached during a recursive walk.
    ///
    /// `through_link` tells whether it was reached by resolving a symbolic link.
    fn object(&mut self, object: &OwnedFd, st: &Stat, display: &Path, through_link: bool) {
        if FileType::from_raw_mode(st.st_mode) == FileType::Directory {
            self.directory(object, display, FileInfo::from_stat(st), through_link);
        } else {
            self.label(object.as_fd(), display);
        }
    }

    /// Relabel everything below a directory, then the directory itself.
    fn directory(&mut self, object: &OwnedFd, display: &Path, id: FileInfo, through_link: bool) {
        if self.root_id.as_ref() == Some(&id) {
            warn_recursive_root(display);
            self.status.failed = true;
            return;
        }

        if self.open_dirs.contains(&id) {
            // With -L, a link back to a directory that is being walked is
            // expected and is simply not descended. Reaching one without a
            // link means the file system itself loops: report it and leave
            // the directory alone.
            if through_link {
                self.label(object.as_fd(), display);
            } else {
                warn_directory_cycle(display);
                self.status.failed = true;
            }
            return;
        }

        // Reopen the very same directory for reading, through its handle.
        let listing = reopen_dir(object).and_then(|dir| dir.read_dir().map(|names| (dir, names)));

        match listing {
            Ok((dir, names)) => {
                self.open_dirs.push(id);
                for name in names {
                    self.entry(&dir, &name, &display.join(&name));
                }
                self.open_dirs.pop();
            }
            // A directory that cannot be read is reported, not relabeled: its
            // context would otherwise change while its contents were left as is.
            Err(e) => {
                return self
                    .status
                    .fail(translate!("chcon-op-reading-directory"), display, e);
            }
        }

        self.label(object.as_fd(), display);
    }

    /// Handle one entry of a directory being walked.
    fn entry(&mut self, dir: &DirFd, name: &OsStr, display: &Path) {
        let entry = match open_path_at(dir, name, SymlinkBehavior::NoFollow) {
            Ok(fd) => fd,
            Err(e) => {
                return self
                    .status
                    .fail(translate!("chcon-op-accessing"), display, e);
            }
        };

        let Some(st) = self.stat(&entry, display) else {
            return;
        };
        if FileType::from_raw_mode(st.st_mode) != FileType::Symlink {
            return self.object(&entry, &st, display, false);
        }

        let walk = match self.options.recursive_mode {
            // -P: the link itself is relabeled.
            RecursiveMode::RecursiveButDoNotFollowSymLinks | RecursiveMode::NotRecursive => {
                return self.label(entry.as_fd(), display);
            }
            // -H: links below an operand are not walked, but what they point
            // to is relabeled.
            RecursiveMode::RecursiveAndFollowArgDirSymLinks => false,
            // -L: links are resolved, and directories they lead to are walked.
            RecursiveMode::RecursiveAndFollowAllDirSymLinks => true,
        };
        // Read the link we just examined through its handle, then resolve
        // its target from the same directory, so the name is looked up once.
        let target = match rustix::fs::readlinkat(&entry, c"", Vec::new()) {
            Ok(target) => target,
            Err(e) => {
                return self
                    .status
                    .fail(translate!("chcon-op-accessing"), display, e.into());
            }
        };
        match open_path_at(dir, target.as_c_str(), SymlinkBehavior::Follow) {
            Ok(referent) if walk => {
                if let Some(st) = self.stat(&referent, display) {
                    self.object(&referent, &st, display, true);
                }
            }
            Ok(referent) => self.label(referent.as_fd(), display),
            Err(e) => self
                .status
                .fail(translate!("chcon-op-accessing"), display, e),
        }
    }

    /// Relabel the object behind `object`, named `display` in messages.
    fn label(&mut self, object: BorrowedFd<'_>, display: &Path) {
        let path = proc_fd_path(object);
        self.status.announce(display);

        let context: Cow<'_, CStr> = match &self.target {
            Target::Whole(text) => Cow::Borrowed(text),
            Target::Fields(fields) => match with_fields(&path, display, fields) {
                Ok(Some(text)) => Cow::Owned(text),
                // The fields already have the requested values.
                Ok(None) => return,
                Err(e) => return self.status.report(&e),
            },
        };

        if let Err(e) =
            SecurityContext::from_c_str(&context, false).set_for_path(&path, true, false)
        {
            let op = translate!("chcon-op-setting-security-context");
            self.status.fail(op, display, selinux_to_io(e));
        }
    }
}

/// The current context of the file at `path`, with `fields` replaced, or `None`
/// if that leaves it unchanged.
fn with_fields(
    path: &Path,
    display: &Path,
    fields: &[(Setter, CString)],
) -> Result<Option<CString>> {
    let building = |e| {
        Error::from_io1(
            translate!("chcon-op-creating-security-context"),
            display,
            selinux_to_io(e),
        )
    };

    let current = SecurityContext::of_path(path, true, false)
        .map_err(|e| {
            Error::from_io1(
                translate!("chcon-op-getting-security-context"),
                display,
                selinux_to_io(e),
            )
        })?
        // An unlabeled file has no fields to keep, so -u/-r/-t/-l cannot apply.
        .ok_or_else(|| {
            Error::from_io1(
                translate!("chcon-op-applying-partial-context"),
                display,
                io::Error::from_raw_os_error(libc::ENODATA),
            )
        })?;

    let current = current
        .to_c_string()
        .map_err(building)?
        .ok_or_else(|| building(selinux::errors::Error::UnexpectedSecurityContextFormat))?;
    let parts = OpaqueSecurityContext::from_c_str(&current).map_err(building)?;
    for (set, value) in fields {
        set(&parts, value).map_err(building)?;
    }
    let wanted = parts.to_c_string().map_err(building)?;
    Ok((wanted.as_c_str() != current.as_ref()).then_some(wanted))
}

#[cfg(unix)]
pub(crate) fn os_str_to_c_string(s: &OsStr) -> Result<CString> {
    use std::os::unix::ffi::OsStrExt;

    CString::new(s.as_bytes())
        .map_err(|_r| Error::from_io("CString::new()", io::ErrorKind::InvalidInput.into()))
}

/// Identify the root directory by device and inode, without following it.
fn root_identity() -> Result<FileInfo> {
    rustix::fs::lstat("/")
        .map(|st| FileInfo::from_stat(&st))
        .map_err(|e| Error::from_io1(translate!("chcon-op-getting-meta-data"), "/", e.into()))
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
