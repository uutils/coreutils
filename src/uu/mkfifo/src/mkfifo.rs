// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use libc::mkfifo;
use std::ffi::CString;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage, show};

static USAGE: &str = help_usage!("mkfifo.md");
static ABOUT: &str = help_about!("mkfifo.md");

mod options {
    pub static MODE: &str = "mode";
    pub static SE_LINUX_SECURITY_CONTEXT: &str = "Z";
    pub static CONTEXT: &str = "context";
    pub static FIFO: &str = "fifo";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    if matches.contains_id(options::CONTEXT) {
        return Err(USimpleError::new(1, "--context is not implemented"));
    }
    if matches.get_flag(options::SE_LINUX_SECURITY_CONTEXT) {
        return Err(USimpleError::new(1, "-Z is not implemented"));
    }

    let mode = match matches.get_one::<String>("mode") {
        None => Ok(0o666),
        Some(str_mode) => uucore::mode::parse_mode(str_mode)
            .map_err(|e| format!("invalid mode ({e})"))
            .and_then(|mode| {
                if mode > 0o777 {
                    Err("mode must specify only file permission bits".to_string())
                } else {
                    Ok(mode)
                }
            }),
    };

    let fifos: Vec<String> = match matches.get_many::<String>(options::FIFO) {
        Some(v) => v.cloned().collect(),
        None => return Err(USimpleError::new(1, "missing operand")),
    };

    for f in fifos {
        let err = unsafe {
            let name = CString::new(f.as_bytes()).unwrap();
            mkfifo(name.as_ptr(), *mode.as_ref().unwrap() as libc::mode_t)
        };
        if err == -1 {
            show!(USimpleError::new(
                1,
                format!("cannot create fifo {}: File exists", f.quote())
            ));
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long(options::MODE)
                .help("file permissions for the fifo")
                .default_value("0666")
                .value_name("MODE"),
        )
        .arg(
            Arg::new(options::SE_LINUX_SECURITY_CONTEXT)
                .short('Z')
                .help("set the SELinux security context to default type")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .help(
                    "like -Z, or if CTX is specified then set the SELinux \
                    or SMACK security context to CTX",
                ),
        )
        .arg(
            Arg::new(options::FIFO)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
