// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command, value_parser};
use libc::mkfifo;
use std::ffi::CString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
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
    let matches = uu_app().try_get_matches_from(args)?;

    let mode = match matches.get_one::<String>(options::MODE) {
        // if mode is passed, ignore umask
        Some(m) => match usize::from_str_radix(m, 8) {
            Ok(m) => m,
            Err(e) => {
                return Err(USimpleError::new(
                    1,
                    translate!("mkfifo-error-invalid-mode", "error" => e),
                ));
            }
        },
        // Default value + umask if present
        None => 0o666 & !(uucore::mode::get_umask() as usize),
    };

    let fifos: Vec<String> = match matches.get_many::<String>(options::FIFO) {
        Some(v) => v.cloned().collect(),
        None => {
            return Err(USimpleError::new(
                1,
                translate!("mkfifo-error-missing-operand"),
            ));
        }
    };

    for f in fifos {
        let err = unsafe {
            let name = CString::new(f.as_bytes()).unwrap();
            mkfifo(name.as_ptr(), 0o666)
        };
        if err == -1 {
            show!(USimpleError::new(
                1,
                translate!("mkfifo-error-cannot-create-fifo", "path" => f.quote()),
            ));
        }

        // Explicitly set the permissions to ignore umask
        if let Err(e) = fs::set_permissions(&f, fs::Permissions::from_mode(mode as u32)) {
            return Err(USimpleError::new(
                1,
                translate!("mkfifo-error-cannot-set-permissions", "path" => f.quote(), "error" => e),
            ));
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
                    let _ = fs::remove_file(f);
                    return Err(USimpleError::new(1, e.to_string()));
                }
            }
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
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
