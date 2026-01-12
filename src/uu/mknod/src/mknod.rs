// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) parsemode makedev sysmacros perror IFBLK IFCHR IFIFO

use clap::{Arg, ArgAction, Command, value_parser};
use libc::{S_IFBLK, S_IFCHR, S_IFIFO, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};
use libc::{dev_t, mode_t};
use std::ffi::CString;

use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError, set_exit_code};
use uucore::format_usage;
use uucore::fs::makedev;
use uucore::translate;

const MODE_RW_UGO: mode_t = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;

mod options {
    pub const MODE: &str = "mode";
    pub const TYPE: &str = "type";
    pub const MAJOR: &str = "major";
    pub const MINOR: &str = "minor";
    pub const SECURITY_CONTEXT: &str = "z";
    pub const CONTEXT: &str = "context";
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

/// Configuration for special inode creation.
pub struct Config<'a> {
    /// bitmask of inode mode (permissions and file type)
    pub mode: mode_t,

    /// when false, the exact mode bits will be set
    pub use_umask: bool,

    pub dev: dev_t,

    /// Set security context (SELinux/SMACK).
    pub set_security_context: bool,

    /// Specific security context (SELinux/SMACK).
    pub context: Option<&'a String>,
}

fn mknod(file_name: &str, config: Config) -> i32 {
    let c_str = CString::new(file_name).expect("Failed to convert to CString");

    unsafe {
        // set umask to 0 and store previous umask
        let have_prev_umask = if config.use_umask {
            None
        } else {
            Some(libc::umask(0))
        };

        let errno = libc::mknod(c_str.as_ptr(), config.mode, config.dev);

        // set umask back to original value
        if let Some(prev_umask) = have_prev_umask {
            libc::umask(prev_umask);
        }

        if errno == -1 {
            let c_str = CString::new(uucore::execution_phrase().as_bytes())
                .expect("Failed to convert to CString");
            // shows the error from the mknod syscall
            libc::perror(c_str.as_ptr());
        }

        // Apply SELinux context if requested
        #[cfg(feature = "selinux")]
        if config.set_security_context {
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

        // Apply SMACK context if requested
        #[cfg(feature = "smack")]
        if config.set_security_context {
            if let Err(e) =
                uucore::smack::set_smack_label_and_cleanup(file_name, config.context, |p| {
                    std::fs::remove_file(p)
                })
            {
                eprintln!("{}: {}", uucore::util_name(), e);
                return 1;
            }
        }

        errno
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let file_type = matches.get_one::<FileType>("type").unwrap();

    let mut use_umask = true;
    let mode_permissions = match matches.get_one::<String>("mode") {
        None => MODE_RW_UGO,
        Some(str_mode) => {
            use_umask = false;
            parse_mode(str_mode).map_err(|e| USimpleError::new(1, e))?
        }
    };
    let mode = mode_permissions | file_type.as_mode();

    let file_name = matches
        .get_one::<String>("name")
        .expect("Missing argument 'NAME'");

    // Extract the security context related flags and options
    let set_security_context = matches.get_flag(options::SECURITY_CONTEXT);
    let context = matches.get_one::<String>(options::CONTEXT);

    let dev = match (
        file_type,
        matches.get_one::<u32>(options::MAJOR),
        matches.get_one::<u32>(options::MINOR),
    ) {
        (FileType::Fifo, None, None) => 0,
        (FileType::Fifo, _, _) => {
            return Err(UUsageError::new(
                1,
                translate!("mknod-error-fifo-no-major-minor"),
            ));
        }
        (_, Some(&major), Some(&minor)) => makedev(major as _, minor as _),
        _ => {
            return Err(UUsageError::new(
                1,
                translate!("mknod-error-special-require-major-minor"),
            ));
        }
    };

    let config = Config {
        mode,
        use_umask,
        dev,
        set_security_context: set_security_context || context.is_some(),
        context,
    };

    let exit_code = mknod(file_name, config);
    set_exit_code(exit_code);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("mknod-usage")))
        .after_help(translate!("mknod-after-help"))
        .about(translate!("mknod-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help(translate!("mknod-help-mode")),
        )
        .arg(
            Arg::new("name")
                .value_name("NAME")
                .help(translate!("mknod-help-name"))
                .required(true)
                .value_hint(clap::ValueHint::AnyPath),
        )
        .arg(
            Arg::new(options::TYPE)
                .value_name("TYPE")
                .help(translate!("mknod-help-type"))
                .required(true)
                .value_parser(parse_type),
        )
        .arg(
            Arg::new(options::MAJOR)
                .value_name(options::MAJOR)
                .help(translate!("mknod-help-major"))
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new(options::MINOR)
                .value_name(options::MINOR)
                .help(translate!("mknod-help-minor"))
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new(options::SECURITY_CONTEXT)
                .short('Z')
                .help(translate!("mknod-help-selinux"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .value_parser(value_parser!(String))
                .num_args(0..=1)
                .require_equals(true)
                .help(translate!("mknod-help-context")),
        )
}

#[allow(clippy::unnecessary_cast)]
fn parse_mode(str_mode: &str) -> Result<mode_t, String> {
    let default_mode = (S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH) as u32;
    uucore::mode::parse_chmod(default_mode, str_mode, true, uucore::mode::get_umask())
        .map_err(|e| {
            translate!(
                "mknod-error-invalid-mode",
                "error" => e
            )
        })
        .and_then(|mode| {
            if mode > 0o777 {
                Err(translate!("mknod-error-mode-permission-bits-only"))
            } else {
                Ok(mode as mode_t)
            }
        })
}

fn parse_type(tpe: &str) -> Result<FileType, String> {
    // Only check the first character, to allow mnemonic usage like
    // 'mknod /dev/rst0 character 18 0'.
    tpe.chars()
        .next()
        .ok_or_else(|| translate!("mknod-error-missing-device-type"))
        .and_then(|first_char| match first_char {
            'b' => Ok(FileType::Block),
            'c' | 'u' => Ok(FileType::Character),
            'p' => Ok(FileType::Fifo),
            _ => Err(translate!("mknod-error-invalid-device-type", "type" => tpe.quote())),
        })
}
