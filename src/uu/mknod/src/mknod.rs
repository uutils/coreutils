// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) parsemode makedev sysmacros perror IFBLK IFCHR IFIFO

use clap::{Arg, ArgAction, Command, value_parser};
use libc::{S_IFBLK, S_IFCHR, S_IFIFO, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};
use libc::{dev_t, mode_t};
use std::collections::HashMap;
use std::ffi::CString;

use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError, set_exit_code};
use uucore::format_usage;
use uucore::locale::{get_message, get_message_with_args};

const MODE_RW_UGO: mode_t = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;

mod options {
    pub const MODE: &str = "mode";
    pub const TYPE: &str = "type";
    pub const MAJOR: &str = "major";
    pub const MINOR: &str = "minor";
    pub const SELINUX: &str = "z";
    pub const CONTEXT: &str = "context";
}

#[inline(always)]
fn makedev(maj: u64, min: u64) -> dev_t {
    // pick up from <sys/sysmacros.h>
    ((min & 0xff) | ((maj & 0xfff) << 8) | ((min & !0xff) << 12) | ((maj & !0xfff) << 32)) as dev_t
}

#[derive(Clone, PartialEq)]
enum FileType {
    Block,
    Character,
    Fifo,
}

impl FileType {
    fn as_mode(&self) -> mode_t {
        match self {
            Self::Block => S_IFBLK,
            Self::Character => S_IFCHR,
            Self::Fifo => S_IFIFO,
        }
    }
}

/// Configuration for directory creation.
pub struct Config<'a> {
    pub mode: mode_t,

    pub dev: dev_t,

    /// Set SELinux security context.
    pub set_selinux_context: bool,

    /// Specific SELinux context.
    pub context: Option<&'a String>,
}

fn mknod(file_name: &str, config: Config) -> i32 {
    let c_str = CString::new(file_name).expect("Failed to convert to CString");

    // the user supplied a mode
    let set_umask = config.mode & MODE_RW_UGO != MODE_RW_UGO;

    unsafe {
        // store prev umask
        let last_umask = if set_umask { libc::umask(0) } else { 0 };

        let errno = libc::mknod(c_str.as_ptr(), config.mode, config.dev);

        // set umask back to original value
        if set_umask {
            libc::umask(last_umask);
        }

        if errno == -1 {
            let c_str = CString::new(uucore::execution_phrase().as_bytes())
                .expect("Failed to convert to CString");
            // shows the error from the mknod syscall
            libc::perror(c_str.as_ptr());
        }

        // Apply SELinux context if requested
        #[cfg(feature = "selinux")]
        if config.set_selinux_context {
            if let Err(e) = uucore::selinux::set_selinux_security_context(
                std::path::Path::new(file_name),
                config.context,
            ) {
                // if it fails, delete the file
                let _ = std::fs::remove_dir(file_name);
                eprintln!("{}: {}", uucore::util_name(), e);
                return 1;
            }
        }

        errno
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let file_type = matches.get_one::<FileType>("type").unwrap();
    let mode = get_mode(matches.get_one::<String>("mode")).map_err(|e| USimpleError::new(1, e))?
        | file_type.as_mode();

    let file_name = matches
        .get_one::<String>("name")
        .expect("Missing argument 'NAME'");

    // Extract the SELinux related flags and options
    let set_selinux_context = matches.get_flag(options::SELINUX);
    let context = matches.get_one::<String>(options::CONTEXT);

    let dev = match (
        file_type,
        matches.get_one::<u64>(options::MAJOR),
        matches.get_one::<u64>(options::MINOR),
    ) {
        (FileType::Fifo, None, None) => 0,
        (FileType::Fifo, _, _) => {
            return Err(UUsageError::new(
                1,
                get_message("mknod-error-fifo-no-major-minor"),
            ));
        }
        (_, Some(&major), Some(&minor)) => makedev(major, minor),
        _ => {
            return Err(UUsageError::new(
                1,
                get_message("mknod-error-special-require-major-minor"),
            ));
        }
    };

    let config = Config {
        mode,
        dev,
        set_selinux_context: set_selinux_context || context.is_some(),
        context,
    };

    let exit_code = mknod(file_name, config);
    set_exit_code(exit_code);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .override_usage(format_usage(&get_message("mknod-usage")))
        .after_help(get_message("mknod-after-help"))
        .about(get_message("mknod-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help(get_message("mknod-help-mode")),
        )
        .arg(
            Arg::new("name")
                .value_name("NAME")
                .help(get_message("mknod-help-name"))
                .required(true)
                .value_hint(clap::ValueHint::AnyPath),
        )
        .arg(
            Arg::new(options::TYPE)
                .value_name("TYPE")
                .help(get_message("mknod-help-type"))
                .required(true)
                .value_parser(parse_type),
        )
        .arg(
            Arg::new(options::MAJOR)
                .value_name(options::MAJOR)
                .help(get_message("mknod-help-major"))
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new(options::MINOR)
                .value_name(options::MINOR)
                .help(get_message("mknod-help-minor"))
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new(options::SELINUX)
                .short('Z')
                .help(get_message("mknod-help-selinux"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .value_parser(value_parser!(String))
                .num_args(0..=1)
                .require_equals(true)
                .help(get_message("mknod-help-context")),
        )
}

fn get_mode(str_mode: Option<&String>) -> Result<mode_t, String> {
    match str_mode {
        None => Ok(MODE_RW_UGO),
        Some(str_mode) => uucore::mode::parse_mode(str_mode)
            .map_err(|e| {
                get_message_with_args(
                    "mknod-error-invalid-mode",
                    HashMap::from([("error".to_string(), e.to_string())]),
                )
            })
            .and_then(|mode| {
                if mode > 0o777 {
                    Err(get_message("mknod-error-mode-permission-bits-only"))
                } else {
                    Ok(mode)
                }
            }),
    }
}

fn parse_type(tpe: &str) -> Result<FileType, String> {
    // Only check the first character, to allow mnemonic usage like
    // 'mknod /dev/rst0 character 18 0'.
    tpe.chars()
        .next()
        .ok_or_else(|| get_message("mknod-error-missing-device-type"))
        .and_then(|first_char| match first_char {
            'b' => Ok(FileType::Block),
            'c' | 'u' => Ok(FileType::Character),
            'p' => Ok(FileType::Fifo),
            _ => Err(get_message_with_args(
                "mknod-error-invalid-device-type",
                HashMap::from([("type".to_string(), tpe.quote().to_string())]),
            )),
        })
}
