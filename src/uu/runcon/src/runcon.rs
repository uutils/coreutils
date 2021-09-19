// spell-checker:ignore (vars) RFILE

use uucore::{show_error, show_usage_error};

use clap::{App, Arg};
use selinux::{OpaqueSecurityContext, SecurityClass, SecurityContext};

use std::borrow::Cow;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::os::raw::c_char;
use std::os::unix::ffi::OsStrExt;
use std::{io, ptr};

mod errors;

use errors::{report_full_error, Error, Result};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str = "Run command with specified security context.";
const DESCRIPTION: &str = "Run COMMAND with completely-specified CONTEXT, or with current or \
                      transitioned security context modified by one or more of \
                      LEVEL, ROLE, TYPE, and USER.\n\n\
                      If none of --compute, --type, --user, --role or --range is specified, \
                      then the first argument is used as the complete context.\n\n\
                      Note that only carefully-chosen contexts are likely to successfully run.\n\n\
                      With neither CONTEXT nor COMMAND are specified, \
                      then this prints the current security context.";

pub mod options {
    pub const COMPUTE: &str = "compute";

    pub const USER: &str = "user";
    pub const ROLE: &str = "role";
    pub const TYPE: &str = "type";
    pub const RANGE: &str = "range";
}

// This list is NOT exhaustive. This command might perform an `execvp()` to run
// a different program. When that happens successfully, the exit status of this
// process will be the exit status of that program.
mod error_exit_status {
    pub const SUCCESS: i32 = libc::EXIT_SUCCESS;
    pub const NOT_FOUND: i32 = 127;
    pub const COULD_NOT_EXECUTE: i32 = 126;
    pub const ANOTHER_ERROR: i32 = libc::EXIT_FAILURE;
}

fn get_usage() -> String {
    format!(
        "{0} [CONTEXT COMMAND [ARG...]]\n    \
         {0} [-c] [-u USER] [-r ROLE] [-t TYPE] [-l RANGE] COMMAND [ARG...]",
        uucore::execution_phrase()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let config = uu_app().usage(usage.as_ref());

    let options = match parse_command_line(config, args) {
        Ok(r) => r,
        Err(r) => {
            if let Error::CommandLine(ref r) = r {
                match r.kind {
                    clap::ErrorKind::HelpDisplayed | clap::ErrorKind::VersionDisplayed => {
                        println!("{}", r);
                        return error_exit_status::SUCCESS;
                    }
                    _ => {}
                }
            }

            show_usage_error!("{}.\n", r);
            return error_exit_status::ANOTHER_ERROR;
        }
    };

    match &options.mode {
        CommandLineMode::Print => {
            if let Err(r) = print_current_context() {
                show_error!("{}", report_full_error(&r));
                return error_exit_status::ANOTHER_ERROR;
            }
        }

        CommandLineMode::PlainContext { context, command } => {
            let (exit_status, err) =
                if let Err(err) = get_plain_context(context).and_then(set_next_exec_context) {
                    (error_exit_status::ANOTHER_ERROR, err)
                } else {
                    // On successful execution, the following call never returns,
                    // and this process image is replaced.
                    execute_command(command, &options.arguments)
                };

            show_error!("{}", report_full_error(&err));
            return exit_status;
        }

        CommandLineMode::CustomContext {
            compute_transition_context,
            user,
            role,
            the_type,
            range,
            command,
        } => {
            if let Some(command) = command {
                let (exit_status, err) = if let Err(err) = get_custom_context(
                    *compute_transition_context,
                    user.as_deref(),
                    role.as_deref(),
                    the_type.as_deref(),
                    range.as_deref(),
                    command,
                )
                .and_then(set_next_exec_context)
                {
                    (error_exit_status::ANOTHER_ERROR, err)
                } else {
                    // On successful execution, the following call never returns,
                    // and this process image is replaced.
                    execute_command(command, &options.arguments)
                };

                show_error!("{}", report_full_error(&err));
                return exit_status;
            } else if let Err(r) = print_current_context() {
                show_error!("{}", report_full_error(&r));
                return error_exit_status::ANOTHER_ERROR;
            }
        }
    }

    error_exit_status::SUCCESS
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(VERSION)
        .about(ABOUT)
        .after_help(DESCRIPTION)
        .arg(
            Arg::with_name(options::COMPUTE)
                .short("c")
                .long(options::COMPUTE)
                .takes_value(false)
                .help("Compute process transition context before modifying."),
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
        .arg(Arg::with_name("ARG").multiple(true))
        // Once "ARG" is parsed, everything after that belongs to it.
        //
        // This is not how POSIX does things, but this is how the GNU implementation
        // parses its command line.
        .setting(clap::AppSettings::TrailingVarArg)
}

#[derive(Debug)]
enum CommandLineMode {
    Print,

    PlainContext {
        context: OsString,
        command: OsString,
    },

    CustomContext {
        /// Compute process transition context before modifying.
        compute_transition_context: bool,

        /// Use the current context with the specified user.
        user: Option<OsString>,

        /// Use the current context with the specified role.
        role: Option<OsString>,

        /// Use the current context with the specified type.
        the_type: Option<OsString>,

        /// Use the current context with the specified range.
        range: Option<OsString>,

        // `command` can be `None`, in which case we're dealing with this syntax:
        // runcon [-c] [-u USER] [-r ROLE] [-t TYPE] [-l RANGE]
        //
        // This syntax is undocumented, but it is accepted by the GNU implementation,
        // so we do the same for compatibility.
        command: Option<OsString>,
    },
}

#[derive(Debug)]
struct Options {
    mode: CommandLineMode,
    arguments: Vec<OsString>,
}

fn parse_command_line(config: App, args: impl uucore::Args) -> Result<Options> {
    let matches = config.get_matches_from_safe(args)?;

    let compute_transition_context = matches.is_present(options::COMPUTE);

    let mut args = matches
        .values_of_os("ARG")
        .unwrap_or_default()
        .map(OsString::from);

    if compute_transition_context
        || matches.is_present(options::USER)
        || matches.is_present(options::ROLE)
        || matches.is_present(options::TYPE)
        || matches.is_present(options::RANGE)
    {
        // runcon [-c] [-u USER] [-r ROLE] [-t TYPE] [-l RANGE] [COMMAND [args]]

        let mode = CommandLineMode::CustomContext {
            compute_transition_context,
            user: matches.value_of_os(options::USER).map(Into::into),
            role: matches.value_of_os(options::ROLE).map(Into::into),
            the_type: matches.value_of_os(options::TYPE).map(Into::into),
            range: matches.value_of_os(options::RANGE).map(Into::into),
            command: args.next(),
        };

        Ok(Options {
            mode,
            arguments: args.collect(),
        })
    } else if let Some(context) = args.next() {
        // runcon CONTEXT COMMAND [args]

        args.next()
            .ok_or(Error::MissingCommand)
            .map(move |command| Options {
                mode: CommandLineMode::PlainContext { context, command },
                arguments: args.collect(),
            })
    } else {
        // runcon

        Ok(Options {
            mode: CommandLineMode::Print,
            arguments: Vec::default(),
        })
    }
}

fn print_current_context() -> Result<()> {
    let op = "Getting security context of the current process";
    let context = SecurityContext::current(false).map_err(|r| Error::from_selinux(op, r))?;

    let context = context
        .to_c_string()
        .map_err(|r| Error::from_selinux(op, r))?;

    if let Some(context) = context {
        let context = context.as_ref().to_str()?;
        println!("{}", context);
    } else {
        println!();
    }
    Ok(())
}

fn set_next_exec_context(context: OpaqueSecurityContext) -> Result<()> {
    let c_context = context
        .to_c_string()
        .map_err(|r| Error::from_selinux("Creating new context", r))?;

    let sc = SecurityContext::from_c_str(&c_context, false);

    if sc.check() != Some(true) {
        let ctx = OsStr::from_bytes(c_context.as_bytes());
        let err = io::ErrorKind::InvalidInput.into();
        return Err(Error::from_io1("Checking security context", ctx, err));
    }

    sc.set_for_next_exec()
        .map_err(|r| Error::from_selinux("Setting new security context", r))
}

fn get_plain_context(context: &OsStr) -> Result<OpaqueSecurityContext> {
    if selinux::kernel_support() == selinux::KernelSupport::Unsupported {
        return Err(Error::SELinuxNotEnabled);
    }

    let c_context = os_str_to_c_string(context)?;

    OpaqueSecurityContext::from_c_str(&c_context)
        .map_err(|r| Error::from_selinux("Creating new context", r))
}

fn get_transition_context(command: &OsStr) -> Result<SecurityContext> {
    // Generate context based on process transition.
    let sec_class = SecurityClass::from_name("process")
        .map_err(|r| Error::from_selinux("Getting process security class", r))?;

    // Get context of file to be executed.
    let file_context = match SecurityContext::of_path(command, true, false) {
        Ok(Some(context)) => context,

        Ok(None) => {
            let err = io::Error::from_raw_os_error(libc::ENODATA);
            return Err(Error::from_io1("getfilecon", command, err));
        }

        Err(r) => {
            let op = "Getting security context of command file";
            return Err(Error::from_selinux(op, r));
        }
    };

    let process_context = SecurityContext::current(false)
        .map_err(|r| Error::from_selinux("Getting security context of the current process", r))?;

    // Compute result of process transition.
    process_context
        .of_labeling_decision(&file_context, sec_class, "")
        .map_err(|r| Error::from_selinux("Computing result of process transition", r))
}

fn get_initial_custom_opaque_context(
    compute_transition_context: bool,
    command: &OsStr,
) -> Result<OpaqueSecurityContext> {
    let context = if compute_transition_context {
        get_transition_context(command)?
    } else {
        SecurityContext::current(false).map_err(|r| {
            Error::from_selinux("Getting security context of the current process", r)
        })?
    };

    let c_context = context
        .to_c_string()
        .map_err(|r| Error::from_selinux("Getting security context", r))?
        .unwrap_or_else(|| Cow::Owned(CString::default()));

    OpaqueSecurityContext::from_c_str(c_context.as_ref())
        .map_err(|r| Error::from_selinux("Creating new context", r))
}

fn get_custom_context(
    compute_transition_context: bool,
    user: Option<&OsStr>,
    role: Option<&OsStr>,
    the_type: Option<&OsStr>,
    range: Option<&OsStr>,
    command: &OsStr,
) -> Result<OpaqueSecurityContext> {
    use OpaqueSecurityContext as OSC;
    type SetNewValueProc = fn(&OSC, &CStr) -> selinux::errors::Result<()>;

    if selinux::kernel_support() == selinux::KernelSupport::Unsupported {
        return Err(Error::SELinuxNotEnabled);
    }

    let osc = get_initial_custom_opaque_context(compute_transition_context, command)?;

    let list: &[(Option<&OsStr>, SetNewValueProc, &'static str)] = &[
        (user, OSC::set_user, "Setting security context user"),
        (role, OSC::set_role, "Setting security context role"),
        (the_type, OSC::set_type, "Setting security context type"),
        (range, OSC::set_range, "Setting security context range"),
    ];

    for &(new_value, method, op) in list {
        if let Some(new_value) = new_value {
            let c_new_value = os_str_to_c_string(new_value)?;
            method(&osc, &c_new_value).map_err(|r| Error::from_selinux(op, r))?;
        }
    }
    Ok(osc)
}

/// The actual return type of this function should be `Result<!, (i32, Error)>`
/// However, until the *never* type is stabilized, one way to indicate to the
/// compiler the only valid return type is to say "if this returns, it will
/// always return an error".
fn execute_command(command: &OsStr, arguments: &[OsString]) -> (i32, Error) {
    let c_command = match os_str_to_c_string(command) {
        Ok(v) => v,
        Err(r) => return (error_exit_status::ANOTHER_ERROR, r),
    };

    let argv_storage: Vec<CString> = match arguments
        .iter()
        .map(AsRef::as_ref)
        .map(os_str_to_c_string)
        .collect::<Result<_>>()
    {
        Ok(v) => v,
        Err(r) => return (error_exit_status::ANOTHER_ERROR, r),
    };

    let mut argv: Vec<*const c_char> = Vec::with_capacity(arguments.len().saturating_add(2));
    argv.push(c_command.as_ptr());
    argv.extend(argv_storage.iter().map(AsRef::as_ref).map(CStr::as_ptr));
    argv.push(ptr::null());

    unsafe { libc::execvp(c_command.as_ptr(), argv.as_ptr()) };

    let err = io::Error::last_os_error();
    let exit_status = if err.kind() == io::ErrorKind::NotFound {
        error_exit_status::NOT_FOUND
    } else {
        error_exit_status::COULD_NOT_EXECUTE
    };

    let err = Error::from_io1("Executing command", command, err);
    (exit_status, err)
}

fn os_str_to_c_string(s: &OsStr) -> Result<CString> {
    CString::new(s.as_bytes())
        .map_err(|_r| Error::from_io("CString::new()", io::ErrorKind::InvalidInput.into()))
}
