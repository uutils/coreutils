// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) parsemode makedev sysmacros perror IFBLK IFCHR IFIFO sflag

use clap::{Arg, ArgAction, Command, value_parser};
use nix::libc::{S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR, mode_t};
use nix::sys::stat::{Mode, SFlag, mknod as nix_mknod, umask as nix_umask};

use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError, set_exit_code};
use uucore::format_usage;
use uucore::fs::makedev;
use uucore::translate;

#[allow(clippy::unnecessary_cast)]
const MODE_RW_UGO: u32 = (S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH) as u32;

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
    fn as_sflag(&self) -> SFlag {
        match self {
            Self::Block => SFlag::S_IFBLK,
            Self::Character => SFlag::S_IFCHR,
            Self::Fifo => SFlag::S_IFIFO,
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

    dev: u64,

    /// Set security context (SELinux/SMACK).
    #[cfg(any(feature = "selinux", feature = "smack"))]
    set_security_context: bool,

    /// Specific security context (SELinux/SMACK).
    #[cfg(any(feature = "selinux", feature = "smack"))]
    context: Option<String>,
}

fn mknod(file_name: &str, config: Config) -> i32 {
    // set umask to 0 and store previous umask
    let have_prev_umask = if config.use_umask {
        None
    } else {
        Some(nix_umask(Mode::empty()))
    };

    let mknod_err = nix_mknod(
        file_name,
        config.file_type.as_sflag(),
        config.mode,
        config.dev as _,
    )
    .err();
    let errno = if mknod_err.is_some() { -1 } else { 0 };

    // set umask back to original value
    if let Some(prev_umask) = have_prev_umask {
        nix_umask(prev_umask);
    }

    if let Some(err) = mknod_err {
        eprintln!(
            "{}: {}",
            uucore::execution_phrase(),
            std::io::Error::from(err)
        );
    }

    // Apply SELinux context if requested
    #[cfg(feature = "selinux")]
    if config.set_security_context {
        if let Err(e) = uucore::selinux::set_selinux_security_context(
            std::path::Path::new(file_name),
            config.context.as_ref(),
        ) {
            // if it fails, delete the file
            let _ = std::fs::remove_file(file_name);
            eprintln!("{}: {e}", uucore::util_name());
            return 1;
        }
    }

    // Apply SMACK context if requested
    #[cfg(feature = "smack")]
    if config.set_security_context {
        if let Err(e) =
            uucore::smack::set_smack_label_and_cleanup(file_name, config.context.as_ref(), |p| {
                std::fs::remove_file(p)
            })
        {
            eprintln!("{}: {e}", uucore::util_name());
            return 1;
        }
    }

    errno
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
    let mode = Mode::from_bits_truncate(mode_permissions as mode_t);

    let file_name = matches
        .get_one::<String>("name")
        .expect("Missing argument 'NAME'");

    // Extract the security context related flags and options
    #[cfg(any(feature = "selinux", feature = "smack"))]
    let set_security_context = matches.get_flag(options::SECURITY_CONTEXT);
    #[cfg(any(feature = "selinux", feature = "smack"))]
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
        (_, Some(&major), Some(&minor)) => makedev(major as _, minor as _) as u64,
        _ => {
            return Err(UUsageError::new(
                1,
                translate!("mknod-error-special-require-major-minor"),
            ));
        }
    };

    let config = Config {
        mode,
        file_type: file_type.clone(),
        use_umask,
        dev,
        #[cfg(any(feature = "selinux", feature = "smack"))]
        set_security_context: set_security_context || context.is_some(),
        #[cfg(any(feature = "selinux", feature = "smack"))]
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

fn parse_mode(str_mode: &str) -> Result<u32, String> {
    let default_mode = MODE_RW_UGO;
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
                Ok(mode)
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
