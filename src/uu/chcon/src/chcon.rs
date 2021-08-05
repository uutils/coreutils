// spell-checker:ignore (vars) RFILE

#![allow(clippy::upper_case_acronyms)]

use uucore::{executable, show_error, show_usage_error, show_warning};

use clap::{App, Arg};

use std::ffi::{CStr, CString, OsStr, OsString};
use std::fmt::Write;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::{fs, io, ptr, slice};

type Result<T> = std::result::Result<T, Error>;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Change the SELinux security context of each FILE to CONTEXT. \n\
                      With --reference, change the security context of each FILE to that of RFILE.";

pub mod options {
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

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... CONTEXT FILE... \n    \
         {0} [OPTION]... [-u USER] [-r ROLE] [-l RANGE] [-t TYPE] FILE... \n    \
         {0} [OPTION]... --reference=RFILE FILE...",
        executable!()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let config = uu_app().usage(usage.as_ref());

    let options = match parse_command_line(config, args) {
        Ok(r) => r,
        Err(r) => {
            if let Error::CommandLine(r) = &r {
                match r.kind {
                    clap::ErrorKind::HelpDisplayed | clap::ErrorKind::VersionDisplayed => {
                        println!("{}", r);
                        return libc::EXIT_SUCCESS;
                    }
                    _ => {}
                }
            }

            show_usage_error!("{}.\n", r);
            return libc::EXIT_FAILURE;
        }
    };

    let context = match &options.mode {
        CommandLineMode::ReferenceBased { reference } => {
            let result = selinux::FileContext::new(reference, true)
                .and_then(|r| {
                    if r.is_empty() {
                        Err(io::Error::from_raw_os_error(libc::ENODATA))
                    } else {
                        Ok(r)
                    }
                })
                .map_err(|r| Error::io1("Getting security context", reference, r));

            match result {
                Err(r) => {
                    show_error!("{}.", report_full_error(&r));
                    return libc::EXIT_FAILURE;
                }

                Ok(file_context) => SELinuxSecurityContext::File(file_context),
            }
        }

        CommandLineMode::ContextBased { context } => {
            match selinux::SecurityContext::security_check_context(context)
                .map_err(|r| Error::io1("Checking security context", context, r))
            {
                Err(r) => {
                    show_error!("{}.", report_full_error(&r));
                    return libc::EXIT_FAILURE;
                }

                Ok(Some(false)) => {
                    show_error!("Invalid security context '{}'.", context.to_string_lossy());
                    return libc::EXIT_FAILURE;
                }

                Ok(Some(true)) | Ok(None) => {}
            }

            let c_context = if let Ok(value) = os_str_to_c_string(context) {
                value
            } else {
                show_error!("Invalid security context '{}'.", context.to_string_lossy());
                return libc::EXIT_FAILURE;
            };

            SELinuxSecurityContext::String(c_context)
        }

        CommandLineMode::Custom { .. } => SELinuxSecurityContext::default(),
    };

    let root_dev_ino = if options.preserve_root && options.recursive_mode.is_recursive() {
        match get_root_dev_ino() {
            Ok(r) => Some(r),

            Err(r) => {
                show_error!("{}.", report_full_error(&r));
                return libc::EXIT_FAILURE;
            }
        }
    } else {
        None
    };

    let results = process_files(&options, &context, root_dev_ino);
    if results.is_empty() {
        return libc::EXIT_SUCCESS;
    }

    for result in &results {
        show_error!("{}.", report_full_error(result));
    }
    libc::EXIT_FAILURE
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .arg(
            Arg::with_name(options::dereference::DEREFERENCE)
                .long(options::dereference::DEREFERENCE)
                .conflicts_with(options::dereference::NO_DEREFERENCE)
                .help(
                    "Affect the referent of each symbolic link (this is the default), \
                     rather than the symbolic link itself.",
                ),
        )
        .arg(
            Arg::with_name(options::dereference::NO_DEREFERENCE)
                .short("h")
                .long(options::dereference::NO_DEREFERENCE)
                .help("Affect symbolic links instead of any referenced file."),
        )
        .arg(
            Arg::with_name(options::preserve_root::PRESERVE_ROOT)
                .long(options::preserve_root::PRESERVE_ROOT)
                .conflicts_with(options::preserve_root::NO_PRESERVE_ROOT)
                .help("Fail to operate recursively on '/'."),
        )
        .arg(
            Arg::with_name(options::preserve_root::NO_PRESERVE_ROOT)
                .long(options::preserve_root::NO_PRESERVE_ROOT)
                .help("Do not treat '/' specially (the default)."),
        )
        .arg(
            Arg::with_name(options::REFERENCE)
                .long(options::REFERENCE)
                .takes_value(true)
                .value_name("RFILE")
                .conflicts_with_all(&[options::USER, options::ROLE, options::TYPE, options::RANGE])
                .help(
                    "Use security context of RFILE, rather than specifying \
                     a CONTEXT value.",
                ),
        )
        .arg(
            Arg::with_name(options::USER)
                .short("u")
                .long(options::USER)
                .takes_value(true)
                .value_name("USER")
                .help("Set user USER in the target security context."),
        )
        .arg(
            Arg::with_name(options::ROLE)
                .short("r")
                .long(options::ROLE)
                .takes_value(true)
                .value_name("ROLE")
                .help("Set role ROLE in the target security context."),
        )
        .arg(
            Arg::with_name(options::TYPE)
                .short("t")
                .long(options::TYPE)
                .takes_value(true)
                .value_name("TYPE")
                .help("Set type TYPE in the target security context."),
        )
        .arg(
            Arg::with_name(options::RANGE)
                .short("l")
                .long(options::RANGE)
                .takes_value(true)
                .value_name("RANGE")
                .help("Set range RANGE in the target security context."),
        )
        .arg(
            Arg::with_name(options::RECURSIVE)
                .short("R")
                .long(options::RECURSIVE)
                .help("Operate on files and directories recursively."),
        )
        .arg(
            Arg::with_name(options::sym_links::FOLLOW_ARG_DIR_SYM_LINK)
                .short("H")
                .requires(options::RECURSIVE)
                .overrides_with_all(&[
                    options::sym_links::FOLLOW_DIR_SYM_LINKS,
                    options::sym_links::NO_FOLLOW_SYM_LINKS,
                ])
                .help(
                    "If a command line argument is a symbolic link to a directory, \
                     traverse it. Only valid when -R is specified.",
                ),
        )
        .arg(
            Arg::with_name(options::sym_links::FOLLOW_DIR_SYM_LINKS)
                .short("L")
                .requires(options::RECURSIVE)
                .overrides_with_all(&[
                    options::sym_links::FOLLOW_ARG_DIR_SYM_LINK,
                    options::sym_links::NO_FOLLOW_SYM_LINKS,
                ])
                .help(
                    "Traverse every symbolic link to a directory encountered. \
                     Only valid when -R is specified.",
                ),
        )
        .arg(
            Arg::with_name(options::sym_links::NO_FOLLOW_SYM_LINKS)
                .short("P")
                .requires(options::RECURSIVE)
                .overrides_with_all(&[
                    options::sym_links::FOLLOW_ARG_DIR_SYM_LINK,
                    options::sym_links::FOLLOW_DIR_SYM_LINKS,
                ])
                .help(
                    "Do not traverse any symbolic links (default). \
                     Only valid when -R is specified.",
                ),
        )
        .arg(
            Arg::with_name(options::VERBOSE)
                .short("v")
                .long(options::VERBOSE)
                .help("Output a diagnostic for every file processed."),
        )
        .arg(Arg::with_name("FILE").multiple(true).min_values(1))
}

fn report_full_error(mut err: &dyn std::error::Error) -> String {
    let mut desc = String::with_capacity(256);
    write!(&mut desc, "{}", err).unwrap();
    while let Some(source) = err.source() {
        err = source;
        write!(&mut desc, ". {}", err).unwrap();
    }
    desc
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("No context is specified")]
    MissingContext,

    #[error("No files are specified")]
    MissingFiles,

    #[error("{0}")]
    ArgumentsMismatch(String),

    #[error(transparent)]
    CommandLine(#[from] clap::Error),

    #[error("{operation} failed")]
    Io {
        operation: &'static str,
        source: io::Error,
    },

    #[error("{operation} failed on '{}'", .operand1.to_string_lossy())]
    Io1 {
        operation: &'static str,
        operand1: OsString,
        source: io::Error,
    },
}

impl Error {
    fn io1(operation: &'static str, operand1: impl Into<OsString>, source: io::Error) -> Self {
        Self::Io1 {
            operation,
            operand1: operand1.into(),
            source,
        }
    }

    #[cfg(unix)]
    fn io1_c_str(operation: &'static str, operand1: &CStr, source: io::Error) -> Self {
        if operand1.to_bytes().is_empty() {
            Self::Io { operation, source }
        } else {
            use std::os::unix::ffi::OsStrExt;

            Self::io1(operation, OsStr::from_bytes(operand1.to_bytes()), source)
        }
    }
}

#[derive(Debug)]
struct Options {
    verbose: bool,
    dereference: bool,
    preserve_root: bool,
    recursive_mode: RecursiveMode,
    affect_symlink_referent: bool,
    mode: CommandLineMode,
    files: Vec<PathBuf>,
}

fn parse_command_line(config: clap::App, args: impl uucore::Args) -> Result<Options> {
    let matches = config.get_matches_from_safe(args)?;

    let verbose = matches.is_present(options::VERBOSE);

    let (recursive_mode, affect_symlink_referent) = if matches.is_present(options::RECURSIVE) {
        if matches.is_present(options::sym_links::FOLLOW_DIR_SYM_LINKS) {
            if matches.is_present(options::dereference::NO_DEREFERENCE) {
                return Err(Error::ArgumentsMismatch(format!(
                    "'--{}' with '--{}' require '-P'",
                    options::RECURSIVE,
                    options::dereference::NO_DEREFERENCE
                )));
            }

            (RecursiveMode::RecursiveAndFollowAllDirSymLinks, true)
        } else if matches.is_present(options::sym_links::FOLLOW_ARG_DIR_SYM_LINK) {
            if matches.is_present(options::dereference::NO_DEREFERENCE) {
                return Err(Error::ArgumentsMismatch(format!(
                    "'--{}' with '--{}' require '-P'",
                    options::RECURSIVE,
                    options::dereference::NO_DEREFERENCE
                )));
            }

            (RecursiveMode::RecursiveAndFollowArgDirSymLinks, true)
        } else {
            if matches.is_present(options::dereference::DEREFERENCE) {
                return Err(Error::ArgumentsMismatch(format!(
                    "'--{}' with '--{}' require either '-H' or '-L'",
                    options::RECURSIVE,
                    options::dereference::DEREFERENCE
                )));
            }

            (RecursiveMode::RecursiveButDoNotFollowSymLinks, false)
        }
    } else {
        let no_dereference = matches.is_present(options::dereference::NO_DEREFERENCE);
        (RecursiveMode::NotRecursive, !no_dereference)
    };

    // By default, dereference.
    let dereference = !matches.is_present(options::dereference::NO_DEREFERENCE);

    // By default, do not preserve root.
    let preserve_root = matches.is_present(options::preserve_root::PRESERVE_ROOT);

    let mut files = matches.values_of_os("FILE").unwrap_or_default();

    let mode = if let Some(path) = matches.value_of_os(options::REFERENCE) {
        CommandLineMode::ReferenceBased {
            reference: PathBuf::from(path),
        }
    } else if matches.is_present(options::USER)
        || matches.is_present(options::ROLE)
        || matches.is_present(options::TYPE)
        || matches.is_present(options::RANGE)
    {
        CommandLineMode::Custom {
            user: matches.value_of_os(options::USER).map(Into::into),
            role: matches.value_of_os(options::ROLE).map(Into::into),
            the_type: matches.value_of_os(options::TYPE).map(Into::into),
            range: matches.value_of_os(options::RANGE).map(Into::into),
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
        dereference,
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
            RecursiveMode::NotRecursive => false,

            RecursiveMode::RecursiveButDoNotFollowSymLinks
            | RecursiveMode::RecursiveAndFollowAllDirSymLinks
            | RecursiveMode::RecursiveAndFollowArgDirSymLinks => true,
        }
    }

    fn fts_open_options(self) -> c_int {
        match self {
            RecursiveMode::NotRecursive | RecursiveMode::RecursiveButDoNotFollowSymLinks => {
                fts_sys::FTS_PHYSICAL
            }

            RecursiveMode::RecursiveAndFollowAllDirSymLinks => fts_sys::FTS_LOGICAL,

            RecursiveMode::RecursiveAndFollowArgDirSymLinks => {
                fts_sys::FTS_PHYSICAL | fts_sys::FTS_COMFOLLOW
            }
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

fn process_files(
    options: &Options,
    context: &SELinuxSecurityContext,
    root_dev_ino: Option<(libc::ino_t, libc::dev_t)>,
) -> Vec<Error> {
    let fts_options = options.recursive_mode.fts_open_options();
    let mut fts = match fts::FTS::new(options.files.iter(), fts_options, None) {
        Ok(fts) => fts,
        Err(source) => {
            return vec![Error::Io {
                operation: "fts_open()",
                source,
            }]
        }
    };

    let mut results = vec![];

    loop {
        match fts.read_next_entry() {
            Ok(true) => {
                if let Err(err) = process_file(options, context, &mut fts, root_dev_ino) {
                    results.push(err);
                }
            }

            Ok(false) => break,

            Err(source) => {
                results.push(Error::Io {
                    operation: "fts_read()",
                    source,
                });

                break;
            }
        }
    }
    results
}

fn process_file(
    options: &Options,
    context: &SELinuxSecurityContext,
    fts: &mut fts::FTS,
    root_dev_ino: Option<(libc::ino_t, libc::dev_t)>,
) -> Result<()> {
    let entry = fts.last_entry_mut().unwrap();

    let file_full_name = if entry.fts_path.is_null() {
        None
    } else {
        let fts_path_size = usize::from(entry.fts_pathlen).saturating_add(1);

        // SAFETY: `entry.fts_path` is a non-null pointer that is assumed to be valid.
        let bytes = unsafe { slice::from_raw_parts(entry.fts_path.cast(), fts_path_size) };
        CStr::from_bytes_with_nul(bytes).ok()
    }
    .ok_or_else(|| Error::Io {
        operation: "File name validation",
        source: io::ErrorKind::InvalidInput.into(),
    })?;

    let fts_access_path = ptr_to_c_str(entry.fts_accpath)
        .map_err(|r| Error::io1_c_str("File name validation", file_full_name, r))?;

    let err = |s, k: io::ErrorKind| Error::io1_c_str(s, file_full_name, k.into());

    let fts_err = |s| {
        let r = io::Error::from_raw_os_error(entry.fts_errno);
        Err(Error::io1_c_str(s, file_full_name, r))
    };

    // SAFETY: If `entry.fts_statp` is not null, then is is assumed to be valid.
    let file_dev_ino = unsafe { entry.fts_statp.as_ref() }
        .map(|stat| (stat.st_ino, stat.st_dev))
        .ok_or_else(|| err("Getting meta data", io::ErrorKind::InvalidInput))?;

    let mut result = Ok(());

    match c_int::from(entry.fts_info) {
        fts_sys::FTS_D => {
            if options.recursive_mode.is_recursive() {
                if root_dev_ino_check(root_dev_ino, file_dev_ino) {
                    // This happens e.g., with "chcon -R --preserve-root ... /"
                    // and with "chcon -RH --preserve-root ... symlink-to-root".
                    root_dev_ino_warn(file_full_name);

                    // Tell fts not to traverse into this hierarchy.
                    let _ignored = fts.set(fts_sys::FTS_SKIP);

                    // Ensure that we do not process "/" on the second visit.
                    let _ignored = fts.read_next_entry();

                    return Err(err("Modifying root path", io::ErrorKind::PermissionDenied));
                }

                return Ok(());
            }
        }

        fts_sys::FTS_DP => {
            if !options.recursive_mode.is_recursive() {
                return Ok(());
            }
        }

        fts_sys::FTS_NS => {
            // For a top-level file or directory, this FTS_NS (stat failed) indicator is determined
            // at the time of the initial fts_open call. With programs like chmod, chown, and chgrp,
            // that modify permissions, it is possible that the file in question is accessible when
            // control reaches this point. So, if this is the first time we've seen the FTS_NS for
            // this file, tell fts_read to stat it "again".
            if entry.fts_level == 0 && entry.fts_number == 0 {
                entry.fts_number = 1;
                let _ignored = fts.set(fts_sys::FTS_AGAIN);
                return Ok(());
            }

            result = fts_err("Accessing");
        }

        fts_sys::FTS_ERR => result = fts_err("Accessing"),

        fts_sys::FTS_DNR => result = fts_err("Reading directory"),

        fts_sys::FTS_DC => {
            if cycle_warning_required(options.recursive_mode.fts_open_options(), entry) {
                emit_cycle_warning(file_full_name);
                return Err(err("Reading cyclic directory", io::ErrorKind::InvalidData));
            }
        }

        _ => {}
    }

    if c_int::from(entry.fts_info) == fts_sys::FTS_DP
        && result.is_ok()
        && root_dev_ino_check(root_dev_ino, file_dev_ino)
    {
        root_dev_ino_warn(file_full_name);
        result = Err(err("Modifying root path", io::ErrorKind::PermissionDenied));
    }

    if result.is_ok() {
        if options.verbose {
            println!(
                "{}: Changing security context of: {}",
                executable!(),
                file_full_name.to_string_lossy()
            );
        }

        result = change_file_context(options, context, fts_access_path);
    }

    if !options.recursive_mode.is_recursive() {
        let _ignored = fts.set(fts_sys::FTS_SKIP);
    }
    result
}

fn set_file_security_context(
    path: &Path,
    context: *const c_char,
    follow_symbolic_links: bool,
) -> Result<()> {
    let mut file_context = selinux::FileContext::from_ptr(context as *mut c_char);
    if file_context.context.is_null() {
        Err(io::Error::from(io::ErrorKind::InvalidInput))
    } else {
        file_context.set_for_file(path, follow_symbolic_links)
    }
    .map_err(|r| Error::io1("Setting security context", path, r))
}

fn change_file_context(
    options: &Options,
    context: &SELinuxSecurityContext,
    file: &CStr,
) -> Result<()> {
    match &options.mode {
        CommandLineMode::Custom {
            user,
            role,
            the_type,
            range,
        } => {
            let path = PathBuf::from(c_str_to_os_string(file));
            let file_context = selinux::FileContext::new(&path, options.affect_symlink_referent)
                .map_err(|r| Error::io1("Getting security context", &path, r))?;

            // If the file doesn't have a context, and we're not setting all of the context
            // components, there isn't really an obvious default. Thus, we just give up.
            if file_context.is_empty() {
                return Err(Error::io1(
                    "Applying partial security context to unlabeled file",
                    path,
                    io::ErrorKind::InvalidInput.into(),
                ));
            }

            let mut se_context = selinux::SecurityContext::new(file_context.as_ptr())
                .map_err(|r| Error::io1("Creating security context", &path, r))?;

            if let Some(user) = user {
                se_context
                    .set_user(user)
                    .map_err(|r| Error::io1("Setting security context user", &path, r))?;
            }

            if let Some(role) = role {
                se_context
                    .set_role(role)
                    .map_err(|r| Error::io1("Setting security context role", &path, r))?;
            }

            if let Some(the_type) = the_type {
                se_context
                    .set_type(the_type)
                    .map_err(|r| Error::io1("Setting security context type", &path, r))?;
            }

            if let Some(range) = range {
                se_context
                    .set_range(range)
                    .map_err(|r| Error::io1("Setting security context range", &path, r))?;
            }

            let context_string = se_context
                .str_bytes()
                .map_err(|r| Error::io1("Getting security context", &path, r))?;

            if !file_context.is_empty() && file_context.as_bytes() == context_string {
                Ok(()) // Nothing to change.
            } else {
                set_file_security_context(
                    &path,
                    context_string.as_ptr().cast(),
                    options.affect_symlink_referent,
                )
            }
        }

        CommandLineMode::ReferenceBased { .. } | CommandLineMode::ContextBased { .. } => {
            let path = PathBuf::from(c_str_to_os_string(file));
            let ctx_ptr = context.as_ptr() as *mut c_char;
            set_file_security_context(&path, ctx_ptr, options.affect_symlink_referent)
        }
    }
}

#[cfg(unix)]
fn c_str_to_os_string(s: &CStr) -> OsString {
    use std::os::unix::ffi::OsStringExt;

    OsString::from_vec(s.to_bytes().to_vec())
}

#[cfg(unix)]
pub(crate) fn os_str_to_c_string(s: &OsStr) -> io::Result<CString> {
    use std::os::unix::ffi::OsStrExt;

    CString::new(s.as_bytes()).map_err(|_r| io::ErrorKind::InvalidInput.into())
}

/// SAFETY:
/// - If `p` is not null, then it is assumed to be a valid null-terminated C string.
/// - The returned `CStr` must not live more than the data pointed-to by `p`.
fn ptr_to_c_str<'s>(p: *const c_char) -> io::Result<&'s CStr> {
    ptr::NonNull::new(p as *mut c_char)
        .map(|p| unsafe { CStr::from_ptr(p.as_ptr()) })
        .ok_or_else(|| io::ErrorKind::InvalidInput.into())
}

/// Call `lstat()` to get the device and inode numbers for `/`.
#[cfg(unix)]
fn get_root_dev_ino() -> Result<(libc::ino_t, libc::dev_t)> {
    use std::os::unix::fs::MetadataExt;

    fs::symlink_metadata("/")
        .map(|md| (md.ino(), md.dev()))
        .map_err(|r| Error::io1("std::fs::symlink_metadata", "/", r))
}

fn root_dev_ino_check(
    root_dev_ino: Option<(libc::ino_t, libc::dev_t)>,
    dir_dev_ino: (libc::ino_t, libc::dev_t),
) -> bool {
    root_dev_ino.map_or(false, |root_dev_ino| root_dev_ino == dir_dev_ino)
}

fn root_dev_ino_warn(dir_name: &CStr) {
    if dir_name.to_bytes() == b"/" {
        show_warning!(
            "It is dangerous to operate recursively on '/'. \
             Use --{} to override this failsafe.",
            options::preserve_root::NO_PRESERVE_ROOT,
        );
    } else {
        show_warning!(
            "It is dangerous to operate recursively on '{}' (same as '/'). \
             Use --{} to override this failsafe.",
            dir_name.to_string_lossy(),
            options::preserve_root::NO_PRESERVE_ROOT,
        );
    }
}

// When fts_read returns FTS_DC to indicate a directory cycle, it may or may not indicate
// a real problem.
// When a program like chgrp performs a recursive traversal that requires traversing symbolic links,
// it is *not* a problem.
// However, when invoked with "-P -R", it deserves a warning.
// The fts_options parameter records the options that control this aspect of fts's behavior,
// so test that.
fn cycle_warning_required(fts_options: c_int, entry: &fts_sys::FTSENT) -> bool {
    // When dereferencing no symlinks, or when dereferencing only those listed on the command line
    // and we're not processing a command-line argument, then a cycle is a serious problem.
    ((fts_options & fts_sys::FTS_PHYSICAL) != 0)
        && (((fts_options & fts_sys::FTS_COMFOLLOW) == 0) || entry.fts_level != 0)
}

fn emit_cycle_warning(file_name: &CStr) {
    show_warning!(
        "Circular directory structure.\n\
This almost certainly means that you have a corrupted file system.\n\
NOTIFY YOUR SYSTEM MANAGER.\n\
The following directory is part of the cycle '{}'.",
        file_name.to_string_lossy()
    )
}

#[derive(Debug)]
enum SELinuxSecurityContext {
    File(selinux::FileContext),
    String(CString),
}

impl Default for SELinuxSecurityContext {
    fn default() -> Self {
        Self::String(CString::default())
    }
}

impl SELinuxSecurityContext {
    #[cfg(unix)]
    fn as_ptr(&self) -> *const c_char {
        match self {
            SELinuxSecurityContext::File(context) => context.as_ptr(),
            SELinuxSecurityContext::String(context) => context.to_bytes_with_nul().as_ptr().cast(),
        }
    }
}

mod fts {
    use std::ffi::{CStr, CString, OsStr};
    use std::os::raw::c_int;
    use std::{io, iter, ptr};

    use super::os_str_to_c_string;

    pub(crate) type FTSOpenCallBack = unsafe extern "C" fn(
        arg1: *mut *const fts_sys::FTSENT,
        arg2: *mut *const fts_sys::FTSENT,
    ) -> c_int;

    #[derive(Debug)]
    pub(crate) struct FTS {
        fts: ptr::NonNull<fts_sys::FTS>,
        entry: *mut fts_sys::FTSENT,
    }

    impl FTS {
        pub(crate) fn new<I>(
            paths: I,
            options: c_int,
            compar: Option<FTSOpenCallBack>,
        ) -> io::Result<Self>
        where
            I: IntoIterator,
            I::Item: AsRef<OsStr>,
        {
            let files_paths = paths
                .into_iter()
                .map(|s| os_str_to_c_string(s.as_ref()))
                .collect::<io::Result<Vec<_>>>()?;

            if files_paths.is_empty() {
                return Err(io::ErrorKind::InvalidInput.into());
            }

            let path_argv = files_paths
                .iter()
                .map(CString::as_ref)
                .map(CStr::as_ptr)
                .chain(iter::once(ptr::null()))
                .collect::<Vec<_>>();

            // SAFETY: We assume calling fts_open() is safe:
            // - `path_argv` is an array holding at least one path, and null-terminated.
            let r = unsafe { fts_sys::fts_open(path_argv.as_ptr().cast(), options, compar) };
            let fts = ptr::NonNull::new(r).ok_or_else(io::Error::last_os_error)?;

            Ok(Self {
                fts,
                entry: ptr::null_mut(),
            })
        }

        pub(crate) fn read_next_entry(&mut self) -> io::Result<bool> {
            // SAFETY: We assume calling fts_read() is safe with a non-null `fts`
            // pointer assumed to be valid.
            self.entry = unsafe { fts_sys::fts_read(self.fts.as_ptr()) };
            if self.entry.is_null() {
                let r = io::Error::last_os_error();
                if let Some(0) = r.raw_os_error() {
                    Ok(false)
                } else {
                    Err(r)
                }
            } else {
                Ok(true)
            }
        }

        pub(crate) fn last_entry_mut(&mut self) -> Option<&mut fts_sys::FTSENT> {
            // SAFETY: If `self.entry` is not null, then is is assumed to be valid.
            unsafe { self.entry.as_mut() }
        }

        pub(crate) fn set(&mut self, instr: c_int) -> io::Result<()> {
            let fts = self.fts.as_ptr();

            let entry = self
                .last_entry_mut()
                .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;

            // SAFETY: We assume calling fts_set() is safe with non-null `fts`
            // and `entry` pointers assumed to be valid.
            if unsafe { fts_sys::fts_set(fts, entry, instr) } == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    impl Drop for FTS {
        fn drop(&mut self) {
            // SAFETY: We assume calling fts_close() is safe with a non-null `fts`
            // pointer assumed to be valid.
            unsafe { fts_sys::fts_close(self.fts.as_ptr()) };
        }
    }
}

mod selinux {
    use std::ffi::OsStr;
    use std::os::raw::c_char;
    use std::path::Path;
    use std::{io, ptr, slice};

    use super::os_str_to_c_string;

    #[derive(Debug)]
    pub(crate) struct SecurityContext(ptr::NonNull<selinux_sys::context_s_t>);

    impl SecurityContext {
        pub(crate) fn new(context_str: *const c_char) -> io::Result<Self> {
            if context_str.is_null() {
                Err(io::ErrorKind::InvalidInput.into())
            } else {
                // SAFETY: We assume calling context_new() is safe with
                // a non-null `context_str` pointer assumed to be valid.
                let p = unsafe { selinux_sys::context_new(context_str) };
                ptr::NonNull::new(p)
                    .ok_or_else(io::Error::last_os_error)
                    .map(Self)
            }
        }

        pub(crate) fn is_selinux_enabled() -> bool {
            // SAFETY: We assume calling is_selinux_enabled() is always safe.
            unsafe { selinux_sys::is_selinux_enabled() != 0 }
        }

        pub(crate) fn security_check_context(context: &OsStr) -> io::Result<Option<bool>> {
            let c_context = os_str_to_c_string(context)?;

            // SAFETY: We assume calling security_check_context() is safe with
            // a non-null `context` pointer assumed to be valid.
            if unsafe { selinux_sys::security_check_context(c_context.as_ptr()) } == 0 {
                Ok(Some(true))
            } else if Self::is_selinux_enabled() {
                Ok(Some(false))
            } else {
                Ok(None)
            }
        }

        pub(crate) fn str_bytes(&self) -> io::Result<&[u8]> {
            // SAFETY: We assume calling context_str() is safe with
            // a non-null `context` pointer assumed to be valid.
            let p = unsafe { selinux_sys::context_str(self.0.as_ptr()) };
            if p.is_null() {
                Err(io::ErrorKind::InvalidInput.into())
            } else {
                let len = unsafe { libc::strlen(p.cast()) }.saturating_add(1);
                Ok(unsafe { slice::from_raw_parts(p.cast(), len) })
            }
        }

        pub(crate) fn set_user(&mut self, user: &OsStr) -> io::Result<()> {
            let c_user = os_str_to_c_string(user)?;

            // SAFETY: We assume calling context_user_set() is safe with non-null
            // `context` and `user` pointers assumed to be valid.
            if unsafe { selinux_sys::context_user_set(self.0.as_ptr(), c_user.as_ptr()) } == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }

        pub(crate) fn set_role(&mut self, role: &OsStr) -> io::Result<()> {
            let c_role = os_str_to_c_string(role)?;

            // SAFETY: We assume calling context_role_set() is safe with non-null
            // `context` and `role` pointers assumed to be valid.
            if unsafe { selinux_sys::context_role_set(self.0.as_ptr(), c_role.as_ptr()) } == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }

        pub(crate) fn set_type(&mut self, the_type: &OsStr) -> io::Result<()> {
            let c_type = os_str_to_c_string(the_type)?;

            // SAFETY: We assume calling context_type_set() is safe with non-null
            // `context` and `the_type` pointers assumed to be valid.
            if unsafe { selinux_sys::context_type_set(self.0.as_ptr(), c_type.as_ptr()) } == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }

        pub(crate) fn set_range(&mut self, range: &OsStr) -> io::Result<()> {
            let c_range = os_str_to_c_string(range)?;

            // SAFETY: We assume calling context_range_set() is safe with non-null
            // `context` and `range` pointers assumed to be valid.
            if unsafe { selinux_sys::context_range_set(self.0.as_ptr(), c_range.as_ptr()) } == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }

    impl Drop for SecurityContext {
        fn drop(&mut self) {
            // SAFETY: We assume calling context_free() is safe with
            // a non-null `context` pointer assumed to be valid.
            unsafe { selinux_sys::context_free(self.0.as_ptr()) }
        }
    }

    #[derive(Debug)]
    pub(crate) struct FileContext {
        pub context: *mut c_char,
        pub len: usize,
        pub allocated: bool,
    }

    impl FileContext {
        pub(crate) fn new(path: &Path, follow_symbolic_links: bool) -> io::Result<Self> {
            let c_path = os_str_to_c_string(path.as_os_str())?;
            let mut context: *mut c_char = ptr::null_mut();

            // SAFETY: We assume calling getfilecon()/lgetfilecon() is safe with
            // non-null `path` and `context` pointers assumed to be valid.
            let len = if follow_symbolic_links {
                unsafe { selinux_sys::getfilecon(c_path.as_ptr(), &mut context) }
            } else {
                unsafe { selinux_sys::lgetfilecon(c_path.as_ptr(), &mut context) }
            };

            if len == -1 {
                let err = io::Error::last_os_error();
                if let Some(libc::ENODATA) = err.raw_os_error() {
                    Ok(Self::default())
                } else {
                    Err(err)
                }
            } else if context.is_null() {
                Ok(Self::default())
            } else {
                Ok(Self {
                    context,
                    len: len as usize,
                    allocated: true,
                })
            }
        }

        pub(crate) fn from_ptr(context: *mut c_char) -> Self {
            if context.is_null() {
                Self::default()
            } else {
                // SAFETY: We assume calling strlen() is safe with a non-null
                // `context` pointer assumed to be valid.
                let len = unsafe { libc::strlen(context) };
                Self {
                    context,
                    len,
                    allocated: false,
                }
            }
        }

        pub(crate) fn set_for_file(
            &mut self,
            path: &Path,
            follow_symbolic_links: bool,
        ) -> io::Result<()> {
            let c_path = os_str_to_c_string(path.as_os_str())?;

            // SAFETY: We assume calling setfilecon()/lsetfilecon() is safe with
            // non-null `path` and `context` pointers assumed to be valid.
            let r = if follow_symbolic_links {
                unsafe { selinux_sys::setfilecon(c_path.as_ptr(), self.context) }
            } else {
                unsafe { selinux_sys::lsetfilecon(c_path.as_ptr(), self.context) }
            };

            if r == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        pub(crate) fn as_ptr(&self) -> *const c_char {
            self.context
        }

        pub(crate) fn is_empty(&self) -> bool {
            self.context.is_null() || self.len == 0
        }

        pub(crate) fn as_bytes(&self) -> &[u8] {
            if self.context.is_null() {
                &[]
            } else {
                // SAFETY: `self.0.context` is a non-null pointer that is assumed to be valid.
                unsafe { slice::from_raw_parts(self.context.cast(), self.len) }
            }
        }
    }

    impl Default for FileContext {
        fn default() -> Self {
            Self {
                context: ptr::null_mut(),
                len: 0,
                allocated: false,
            }
        }
    }

    impl Drop for FileContext {
        fn drop(&mut self) {
            if self.allocated && !self.context.is_null() {
                // SAFETY: We assume calling freecon() is safe with a non-null
                // `context` pointer assumed to be valid.
                unsafe { selinux_sys::freecon(self.context) }
            }
        }
    }
}
