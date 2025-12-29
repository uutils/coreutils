// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (vars) RFILE
#![cfg(target_os = "linux")]

use clap::builder::ValueParser;
use uucore::error::{UError, UResult};
use uucore::translate;

use clap::{Arg, ArgAction, Command};
use selinux::{OpaqueSecurityContext, SecurityClass, SecurityContext};
use uucore::format_usage;

use std::borrow::Cow;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::process;

mod errors;

use errors::error_exit_status;
use errors::{Error, Result, RunconError};

pub mod options {
    pub const COMPUTE: &str = "compute";

    pub const USER: &str = "user";
    pub const ROLE: &str = "role";
    pub const TYPE: &str = "type";
    pub const RANGE: &str = "range";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let config = uu_app();

    let options = parse_command_line(config, args)?;

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

pub fn uu_app() -> Command {
    let cmd = Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("runcon-about"))
        .after_help(translate!("runcon-after-help"))
        .override_usage(format_usage(&translate!("runcon-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        .arg(
            Arg::new(options::COMPUTE)
                .short('c')
                .long(options::COMPUTE)
                .help(translate!("runcon-help-compute"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USER)
                .short('u')
                .long(options::USER)
                .value_name("USER")
                .help(translate!("runcon-help-user"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::ROLE)
                .short('r')
                .long(options::ROLE)
                .value_name("ROLE")
                .help(translate!("runcon-help-role"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::TYPE)
                .short('t')
                .long(options::TYPE)
                .value_name("TYPE")
                .help(translate!("runcon-help-type"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::RANGE)
                .short('l')
                .long(options::RANGE)
                .value_name("RANGE")
                .help(translate!("runcon-help-range"))
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new("ARG")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::CommandName),
        )
        // Once "ARG" is parsed, everything after that belongs to it.
        //
        // This is not how POSIX does things, but this is how the GNU implementation
        // parses its command line.
        .trailing_var_arg(true)
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

fn parse_command_line(config: Command, args: impl uucore::Args) -> UResult<Options> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(config, args, 125)?;

    let compute_transition_context = matches.get_flag(options::COMPUTE);

    let mut args = matches
        .get_many::<OsString>("ARG")
        .unwrap_or_default()
        .map(OsString::from);

    if compute_transition_context
        || matches.contains_id(options::USER)
        || matches.contains_id(options::ROLE)
        || matches.contains_id(options::TYPE)
        || matches.contains_id(options::RANGE)
    {
        // runcon [-c] [-u USER] [-r ROLE] [-t TYPE] [-l RANGE] [COMMAND [args]]

        let mode = CommandLineMode::CustomContext {
            compute_transition_context,
            user: matches.get_one::<OsString>(options::USER).map(Into::into),
            role: matches.get_one::<OsString>(options::ROLE).map(Into::into),
            the_type: matches.get_one::<OsString>(options::TYPE).map(Into::into),
            range: matches.get_one::<OsString>(options::RANGE).map(Into::into),
            command: args.next(),
        };

        Ok(Options {
            mode,
            arguments: args.collect(),
        })
    } else if let Some(context) = args.next() {
        // runcon CONTEXT COMMAND [args]

        args.next()
            .ok_or_else(|| Box::new(Error::MissingCommand) as Box<dyn UError>)
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
    let context = SecurityContext::current(false)
        .map_err(|r| Error::from_selinux("runcon-operation-getting-current-context", r))?;

    let context = context
        .to_c_string()
        .map_err(|r| Error::from_selinux("runcon-operation-getting-current-context", r))?;

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
        .map_err(|r| Error::from_selinux("runcon-operation-creating-context", r))?;

    let sc = SecurityContext::from_c_str(&c_context, false);

    if sc.check() != Some(true) {
        let ctx = OsStr::from_bytes(c_context.as_bytes());
        let err = io::ErrorKind::InvalidInput.into();
        return Err(Error::from_io1(
            "runcon-operation-checking-context",
            ctx,
            err,
        ));
    }

    sc.set_for_next_exec()
        .map_err(|r| Error::from_selinux("runcon-operation-setting-context", r))
}

fn get_plain_context(context: &OsStr) -> Result<OpaqueSecurityContext> {
    if !uucore::selinux::is_selinux_enabled() {
        return Err(Error::SELinuxNotEnabled);
    }

    let c_context = os_str_to_c_string(context)?;

    OpaqueSecurityContext::from_c_str(&c_context)
        .map_err(|r| Error::from_selinux("runcon-operation-creating-context", r))
}

fn get_transition_context(command: &OsStr) -> Result<SecurityContext<'_>> {
    // Generate context based on process transition.
    let sec_class = SecurityClass::from_name("process")
        .map_err(|r| Error::from_selinux("runcon-operation-getting-process-class", r))?;

    // Get context of file to be executed.
    let file_context = match SecurityContext::of_path(command, true, false) {
        Ok(Some(context)) => context,

        Ok(None) => {
            let err = io::Error::from_raw_os_error(libc::ENODATA);
            return Err(Error::from_io1("runcon-operation-getfilecon", command, err));
        }

        Err(r) => {
            return Err(Error::from_selinux(
                "runcon-operation-getting-file-context",
                r,
            ));
        }
    };

    let process_context = SecurityContext::current(false)
        .map_err(|r| Error::from_selinux("runcon-operation-getting-current-context", r))?;

    // Compute result of process transition.
    process_context
        .of_labeling_decision(&file_context, sec_class, "")
        .map_err(|r| Error::from_selinux("runcon-operation-computing-transition", r))
}

fn get_initial_custom_opaque_context(
    compute_transition_context: bool,
    command: &OsStr,
) -> Result<OpaqueSecurityContext> {
    let context = if compute_transition_context {
        get_transition_context(command)?
    } else {
        SecurityContext::current(false)
            .map_err(|r| Error::from_selinux("runcon-operation-getting-current-context", r))?
    };

    let c_context = context
        .to_c_string()
        .map_err(|r| Error::from_selinux("runcon-operation-getting-context", r))?
        .unwrap_or_else(|| Cow::Owned(CString::default()));

    OpaqueSecurityContext::from_c_str(c_context.as_ref())
        .map_err(|r| Error::from_selinux("runcon-operation-creating-context", r))
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

    if !uucore::selinux::is_selinux_enabled() {
        return Err(Error::SELinuxNotEnabled);
    }

    let osc = get_initial_custom_opaque_context(compute_transition_context, command)?;

    let list: &[(Option<&OsStr>, SetNewValueProc, &'static str)] = &[
        (user, OSC::set_user, "runcon-operation-setting-user"),
        (role, OSC::set_role, "runcon-operation-setting-role"),
        (the_type, OSC::set_type, "runcon-operation-setting-type"),
        (range, OSC::set_range, "runcon-operation-setting-range"),
    ];

    for &(new_value, method, op_key) in list {
        if let Some(new_value) = new_value {
            let c_new_value = os_str_to_c_string(new_value)?;
            method(&osc, &c_new_value).map_err(|r| Error::from_selinux(op_key, r))?;
        }
    }
    Ok(osc)
}

/// The actual return type of this function should be `UResult<!>`.
/// However, until the *never* type is stabilized, one way to indicate to the
/// compiler the only valid return type is to say "if this returns, it will
/// always return an error".
fn execute_command(command: &OsStr, arguments: &[OsString]) -> UResult<()> {
    let err = process::Command::new(command).args(arguments).exec();

    let exit_status = if err.kind() == io::ErrorKind::NotFound {
        error_exit_status::NOT_FOUND
    } else {
        error_exit_status::COULD_NOT_EXECUTE
    };

    let err = Error::from_io1("runcon-operation-executing-command", command, err);
    Err(RunconError::with_code(exit_status, err).into())
}

fn os_str_to_c_string(s: &OsStr) -> Result<CString> {
    CString::new(s.as_bytes()).map_err(|_r| {
        Error::from_io(
            "runcon-operation-cstring-new",
            io::ErrorKind::InvalidInput.into(),
        )
    })
}
