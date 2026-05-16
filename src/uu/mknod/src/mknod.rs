// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) parsemode makedev sysmacros perror RAII mknodat

use std::ffi::CString;

use clap::{Arg, ArgAction, Command, value_parser};
use rustix::fs::{FileType as RustixFileType, Mode};
use rustix::process::umask;
use std::ffi::OsString;
use std::io::{self, Write as _};

use uucore::display::Quotable;
use uucore::error::{ExitCode, UResult, USimpleError, UUsageError, set_exit_code};
use uucore::format_usage;
use uucore::fs::makedev;
use uucore::translate;

const MODE_RW_UGO: u32 = 0o666;

mod options {
    pub const MODE: &str = "mode";
    pub const TYPE: &str = "type";
    pub const MAJOR: &str = "major";
    pub const MINOR: &str = "minor";
    pub const SECURITY_CONTEXT: &str = "z";
    pub const CONTEXT: &str = "context";
}

#[derive(Clone, Copy, PartialEq)]
enum FileType {
    Block,
    Character,
    Fifo,
}

impl FileType {
    fn to_file_type(self) -> RustixFileType {
        match self {
            Self::Block => RustixFileType::BlockDevice,
            Self::Character => RustixFileType::CharacterDevice,
            Self::Fifo => RustixFileType::Fifo,
        }
    }
}

/// Configuration for special inode creation.
struct Config {
    /// Permission bits for the inode
    mode: Mode,

    file_type: FileType,

    /// when false, the exact mode bits will be set
    use_umask: bool,

    dev: libc::dev_t,

    /// Set security context (SELinux/SMACK).
    #[cfg(any(
        all(feature = "selinux", any(target_os = "android", target_os = "linux")),
        all(feature = "smack", target_os = "linux"),
    ))]
    set_security_context: bool,

    /// Specific security context (SELinux/SMACK).
    #[cfg(any(
        all(feature = "selinux", any(target_os = "android", target_os = "linux")),
        all(feature = "smack", target_os = "linux"),
    ))]
    context: Option<String>,
}

/// RAII guard to restore umask on drop, ensuring cleanup even on panic.
struct UmaskGuard(Mode);

impl UmaskGuard {
    fn set(new_mask: Mode) -> Self {
        let old_mask = umask(new_mask);
        Self(old_mask)
    }
}

impl Drop for UmaskGuard {
    fn drop(&mut self) {
        umask(self.0);
    }
}

/// Create a special file using `mknod(2)`.
///
/// Uses `libc::mknod` directly since `rustix::fs::mknodat` is unavailable on
/// Apple targets. Combines `file_type` (S_IF* bits) with `mode` (permission
/// bits) into the raw `mode_t` argument expected by the syscall.
fn do_mknod(path: &str, file_type: RustixFileType, mode: Mode, dev: u64) -> std::io::Result<()> {
    let raw_mode = file_type.as_raw_mode() | mode.as_raw_mode();
    let c_path = CString::new(path)?;
    let result = unsafe { libc::mknod(c_path.as_ptr(), raw_mode as _, dev as _) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn mknod(file_name: &str, config: Config) -> i32 {
    // Label the node at creation, as GNU does; relabelling after leaves a window.
    #[cfg(all(feature = "selinux", any(target_os = "android", target_os = "linux")))]
    let _selinux_guard = if config.set_security_context {
        let mode = config.file_type.to_file_type().as_raw_mode() | config.mode.as_raw_mode();
        match uucore::selinux::FsCreateContext::new(
            std::path::Path::new(file_name),
            Some(mode),
            config.context.as_ref(),
        ) {
            Ok(guard) => Some(guard),
            Err(e) => {
                let _ = writeln!(io::stderr(), "mknod: {e}");
                return 1;
            }
        }
    } else {
        None
    };

    let _guard = if config.use_umask {
        None
    } else {
        Some(UmaskGuard::set(Mode::empty()))
    };

    let mknod_err = do_mknod(
        file_name,
        config.file_type.to_file_type(),
        config.mode,
        config.dev as _,
    )
    .err();
    let errno = if mknod_err.is_some() { -1 } else { 0 };

    if let Some(err) = mknod_err {
        let _ = writeln!(io::stderr(), "{}: {err}", uucore::execution_phrase());
    }

    // Apply SMACK context if requested
    #[cfg(all(feature = "smack", target_os = "linux"))]
    if config.set_security_context
        && let Err(e) =
            uucore::smack::set_smack_label_and_cleanup(file_name, config.context.as_ref(), |p| {
                std::fs::remove_file(p)
            })
    {
        let _ = writeln!(io::stderr(), "mknod: {e}");
        return 1;
    }

    errno
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.collect();
    // Kept for the caret in mode diagnostics, which needs the mode as typed.
    let diag_args = uucore::diagnostics::operands(&args);
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let file_type = matches.get_one::<FileType>("type").unwrap();

    let mut use_umask = true;
    let mode_permissions = match matches.get_one::<String>("mode") {
        None => MODE_RW_UGO,
        Some(str_mode) => {
            use_umask = false;
            let mode =
                uucore::mode::parse_chmod(MODE_RW_UGO, str_mode, true, uucore::mode::get_umask())
                    .map_err(|err| {
                    let message =
                        translate!("mknod-error-invalid-mode", "error" => err.to_string());
                    if let Some(args) = &diag_args
                        && err.render_mode_value(args, str_mode, 0, &message)
                    {
                        // The diagnostic is already on stderr; exit quietly.
                        return ExitCode::new(1);
                    }
                    USimpleError::new(1, message)
                })?;
            if mode > 0o777 {
                return Err(USimpleError::new(
                    1,
                    translate!("mknod-error-mode-permission-bits-only"),
                ));
            }
            mode
        }
    };
    let mode = Mode::from_bits_truncate(mode_permissions as _);

    let file_name = matches
        .get_one::<String>("name")
        .expect("Missing argument 'NAME'");

    // Extract the security context related flags and options
    #[cfg(any(
        all(feature = "selinux", any(target_os = "android", target_os = "linux")),
        all(feature = "smack", target_os = "linux"),
    ))]
    let set_security_context = matches.get_flag(options::SECURITY_CONTEXT);
    #[cfg(any(
        all(feature = "selinux", any(target_os = "android", target_os = "linux")),
        all(feature = "smack", target_os = "linux"),
    ))]
    let context = matches.get_one::<String>(options::CONTEXT).cloned();

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
        file_type: *file_type,
        use_umask,
        dev,
        #[cfg(any(
            all(feature = "selinux", any(target_os = "android", target_os = "linux")),
            all(feature = "smack", target_os = "linux"),
        ))]
        set_security_context: set_security_context || context.is_some(),
        #[cfg(any(
            all(feature = "selinux", any(target_os = "android", target_os = "linux")),
            all(feature = "smack", target_os = "linux"),
        ))]
        context,
    };

    let exit_code = mknod(file_name, config);
    set_exit_code(exit_code);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("mknod")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("mknod"))
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

fn parse_type(tpe: &str) -> Result<FileType, String> {
    // Dispatch on the leading character alone, so a spelled-out type works
    // wherever its initial does: `character` is read like `c` in
    // `mknod /dev/ttyS0 character 4 64`.
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
