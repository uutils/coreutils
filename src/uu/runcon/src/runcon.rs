// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (vars) RFILE

use clap::Command;
use uucore::error::{UResult, UUsageError};

use selinux::{OpaqueSecurityContext, SecurityClass, SecurityContext};

use std::borrow::Cow;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::os::raw::c_char;
use std::os::unix::ffi::OsStrExt;
use std::{io, ptr};

use crate::errors::error_exit_status;
use crate::errors::{Error, Result, RunconError};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let config = crate::uu_app();

    let options = match parse_command_line(config, args) {
        Ok(r) => r,
        Err(r) => {
            if let Error::CommandLine(ref r) = r {
                match r.kind() {
                    clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion => {
                        println!("{r}");
                        return Ok(());
                    }
                    _ => {}
                }
            }
            return Err(UUsageError::new(
                error_exit_status::ANOTHER_ERROR,
                format!("{r}"),
            ));
        }
    };

    match &options.mode {
        CommandLineMode::Print => print_current_context().map_err(|e| RunconError::new(e).into()),
        CommandLineMode::PlainContext { context, command } => {
            get_plain_context(context)
                .and_then(|ctx| set_next_exec_context(&ctx))
                .map_err(RunconError::new)?;
            // On successful execution, the following call never returns,
            // and this process image is replaced.
            execute_command(command, &options.arguments)
        }
        CommandLineMode::CustomContext {
            compute_transition_context,
            user,
            role,
            the_type,
            range,
            command,
        } => {
            match command {
                Some(command) => {
                    get_custom_context(
                        *compute_transition_context,
                        user.as_deref(),
                        role.as_deref(),
                        the_type.as_deref(),
                        range.as_deref(),
                        command,
                    )
                    .and_then(|ctx| set_next_exec_context(&ctx))
                    .map_err(RunconError::new)?;
                    // On successful execution, the following call never returns,
                    // and this process image is replaced.
                    execute_command(command, &options.arguments)
                }
                None => print_current_context().map_err(|e| RunconError::new(e).into()),
            }
        }
    }
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

fn parse_command_line(config: Command, args: impl uucore::Args) -> Result<Options> {
    let matches = config.try_get_matches_from(args)?;

    let compute_transition_context = matches.get_flag(crate::options::COMPUTE);

    let mut args = matches
        .get_many::<OsString>("ARG")
        .unwrap_or_default()
        .map(OsString::from);

    if compute_transition_context
        || matches.contains_id(crate::options::USER)
        || matches.contains_id(crate::options::ROLE)
        || matches.contains_id(crate::options::TYPE)
        || matches.contains_id(crate::options::RANGE)
    {
        // runcon [-c] [-u USER] [-r ROLE] [-t TYPE] [-l RANGE] [COMMAND [args]]

        let mode = CommandLineMode::CustomContext {
            compute_transition_context,
            user: matches
                .get_one::<OsString>(crate::options::USER)
                .map(Into::into),
            role: matches
                .get_one::<OsString>(crate::options::ROLE)
                .map(Into::into),
            the_type: matches
                .get_one::<OsString>(crate::options::TYPE)
                .map(Into::into),
            range: matches
                .get_one::<OsString>(crate::options::RANGE)
                .map(Into::into),
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
        println!("{context}");
    } else {
        println!();
    }
    Ok(())
}

fn set_next_exec_context(context: &OpaqueSecurityContext) -> Result<()> {
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

/// The actual return type of this function should be `UResult<!>`.
/// However, until the *never* type is stabilized, one way to indicate to the
/// compiler the only valid return type is to say "if this returns, it will
/// always return an error".
fn execute_command(command: &OsStr, arguments: &[OsString]) -> UResult<()> {
    let c_command = os_str_to_c_string(command).map_err(RunconError::new)?;

    let argv_storage: Vec<CString> = arguments
        .iter()
        .map(AsRef::as_ref)
        .map(os_str_to_c_string)
        .collect::<Result<_>>()
        .map_err(RunconError::new)?;

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
    Err(RunconError::with_code(exit_status, err).into())
}

fn os_str_to_c_string(s: &OsStr) -> Result<CString> {
    CString::new(s.as_bytes())
        .map_err(|_r| Error::from_io("CString::new()", io::ErrorKind::InvalidInput.into()))
}
