// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command, value_parser};
use rustix::fs::Mode;
use rustix::process::umask;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, strip_errno};
use uucore::translate;

use uucore::{format_usage, show};

mod options {
    pub static MODE: &str = "mode";
    pub static SECURITY_CONTEXT: &str = "Z";
    pub static CONTEXT: &str = "context";
    pub static FIFO: &str = "fifo";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mode = calculate_mode(matches.get_one::<String>(options::MODE))
        .map_err(|e| USimpleError::new(1, translate!("mkfifo-error-invalid-mode", "error" => e)))?;

    // Check if mode contains special bits
    let non_file_permission_bits = 0o7000; // setuid, setgid, sticky bits
    if mode & non_file_permission_bits != 0 {
        return Err(USimpleError::new(
            1,
            translate!("mkfifo-error-non-file-permission"),
        ));
    }

    #[allow(clippy::unwrap_used, reason = "set as required by clap")]
    let fifos: Vec<String> = matches
        .get_many::<String>(options::FIFO)
        .unwrap()
        .cloned()
        .collect();

    for f in fifos {
        // Clear umask around mkfifo so the kernel applies the exact
        // requested mode atomically. Skipping the path-based chmod
        // that used to follow this call closes the TOCTOU window an
        // attacker could use to swap the FIFO for a symlink between
        // mkfifo and chmod (issue #10020).
        let prev_umask = umask(Mode::empty());
        let mkfifo_result = create_fifo(f.as_str(), mode);
        umask(prev_umask);

        if let Err(e) = mkfifo_result {
            show!(USimpleError::new(
                1,
                translate!(
                    "mkfifo-error-cannot-create-fifo",
                    "path" => f.quote(),
                    "error" => strip_errno(&e)
                ),
            ));
        } else {
            // Apply SELinux context if requested
            #[cfg(all(feature = "selinux", any(target_os = "linux", target_os = "android")))]
            {
                let set_security_context = matches.get_flag(options::SECURITY_CONTEXT);
                let context = matches.get_one::<String>(options::CONTEXT);

                if set_security_context || context.is_some() {
                    use std::path::Path;
                    if let Err(e) =
                        uucore::selinux::set_selinux_security_context(Path::new(&f), context)
                    {
                        let _ = std::fs::remove_file(f);
                        return Err(USimpleError::new(1, e.to_string()));
                    }
                }
            }

            // Apply SMACK context if requested
            #[cfg(all(feature = "smack", target_os = "linux"))]
            {
                let set_security_context = matches.get_flag(options::SECURITY_CONTEXT);
                let context = matches.get_one::<String>(options::CONTEXT);
                if set_security_context || context.is_some() {
                    uucore::smack::set_smack_label_and_cleanup(&f, context, |p| {
                        std::fs::remove_file(p)
                    })?;
                }
            }
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("mkfifo")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("mkfifo"))
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
            Arg::new(options::SECURITY_CONTEXT)
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
                .required(true)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

// `rustix::fs::mkfifoat` is unavailable on Apple targets, so fall back to
// libc's path-based `mkfifo` there. Both rely on the caller having cleared
// the umask so the requested mode is applied atomically (see issue #10020).
#[cfg(not(target_vendor = "apple"))]
fn create_fifo(path: &str, mode: u32) -> Result<(), std::io::Error> {
    use rustix::fs::{CWD, mkfifoat};
    mkfifoat(CWD, path, Mode::from_bits_truncate(mode)).map_err(std::io::Error::from)
}

#[cfg(target_vendor = "apple")]
fn create_fifo(path: &str, mode: u32) -> Result<(), std::io::Error> {
    use std::ffi::CString;
    let c_path =
        CString::new(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    // SAFETY: `c_path` is a valid NUL-terminated C string and `mode` is a
    // standard mode_t bit pattern.
    let rc = unsafe { libc::mkfifo(c_path.as_ptr(), mode as libc::mode_t) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn calculate_mode(mode_option: Option<&String>) -> Result<u32, String> {
    let umask = uucore::mode::get_umask();
    let mode = 0o666; // Default mode for FIFOs

    if let Some(m) = mode_option {
        uucore::mode::parse_chmod(mode, m, false, umask)
    } else {
        Ok(mode & !umask) // Apply umask if no mode is specified
    }
}
