// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command, value_parser};
use libc::{mkfifo, mode_t, umask};
use std::ffi::CString;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::translate;

use uucore::{format_usage, show};

mod options {
    pub static MODE: &str = "mode";
    pub static SELINUX: &str = "Z";
    pub static CONTEXT: &str = "context";
    pub static FIFO: &str = "fifo";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mode = calculate_mode(matches.get_one::<String>(options::MODE))
        .map_err(|e| USimpleError::new(1, translate!("mkfifo-error-invalid-mode", "error" => e)))?;

    let fifos: Vec<String> = match matches.get_many::<String>(options::FIFO) {
        Some(v) => v.cloned().collect(),
        None => {
            return Err(USimpleError::new(
                1,
                translate!("mkfifo-error-missing-operand"),
            ));
        }
    };

    let has_mode = matches.contains_id(options::MODE);
    // Set umask to 0 temporarily if -m option is applied
    // mkfifo applies umask to requested mode
    let old_umask = if has_mode { unsafe { umask(0) } } else { 0 };

    for f in fifos {
        let name = CString::new(f.as_bytes()).unwrap();
        let err = unsafe { mkfifo(name.as_ptr(), mode as mode_t) };

        if err == -1 {
            show!(USimpleError::new(
                1,
                translate!("mkfifo-error-cannot-create-fifo", "path" => f.quote()),
            ));
            continue;
        }

        // Apply SELinux context if requested
        #[cfg(feature = "selinux")]
        {
            // Extract the SELinux related flags and options
            let set_selinux_context = matches.get_flag(options::SELINUX);
            let context = matches.get_one::<String>(options::CONTEXT);

            if set_selinux_context || context.is_some() {
                use std::path::Path;
                if let Err(e) =
                    uucore::selinux::set_selinux_security_context(Path::new(&f), context)
                {
                    let _ = std::fs::remove_file(f);
                    return Err(USimpleError::new(1, e.to_string()));
                }
            }
        }
    }

    if has_mode {
        unsafe { umask(old_umask) };
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("mkfifo-usage")))
        .about(translate!("mkfifo-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long(options::MODE)
                .help(translate!("mkfifo-help-mode"))
                .value_name("MODE"),
        )
        .arg(
            Arg::new(options::SELINUX)
                .short('Z')
                .help(translate!("mkfifo-help-selinux"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .value_parser(value_parser!(String))
                .num_args(0..=1)
                .require_equals(true)
                .help(translate!("mkfifo-help-context")),
        )
        .arg(
            Arg::new(options::FIFO)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

fn calculate_mode(mode_option: Option<&String>) -> Result<u32, String> {
    let umask = uucore::mode::get_umask();
    let mode = 0o666; // Default mode for FIFOs

    if let Some(m) = mode_option {
        uucore::mode::parse_chmod(mode, m, false, umask)
    } else {
        Ok(mode) // current mask will be applied automatically
    }
}
