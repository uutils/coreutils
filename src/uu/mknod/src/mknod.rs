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
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("mknod.md");
const USAGE: &str = help_usage!("mknod.md");
const AFTER_HELP: &str = help_section!("after help", "mknod.md");

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
                "Fifos do not have major and minor device numbers.",
            ));
        }
        (_, Some(&major), Some(&minor)) => makedev(major, minor),
        _ => {
            return Err(UUsageError::new(
                1,
                "Special files require major and minor device numbers.",
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
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help("set file permission bits to MODE, not a=rw - umask"),
        )
        .arg(
            Arg::new("name")
                .value_name("NAME")
                .help("name of the new file")
                .required(true)
                .value_hint(clap::ValueHint::AnyPath),
        )
        .arg(
            Arg::new(options::TYPE)
                .value_name("TYPE")
                .help("type of the new file (b, c, u or p)")
                .required(true)
                .value_parser(parse_type),
        )
        .arg(
            Arg::new(options::MAJOR)
                .value_name(options::MAJOR)
                .help("major file type")
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new(options::MINOR)
                .value_name(options::MINOR)
                .help("minor file type")
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new(options::SELINUX)
                .short('Z')
                .help("set SELinux security context of each created directory to the default type")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .value_parser(value_parser!(String))
                .num_args(0..=1)
                .require_equals(true)
                .help("like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX")
        )
}

fn get_mode(str_mode: Option<&String>) -> Result<mode_t, String> {
    match str_mode {
        None => Ok(MODE_RW_UGO),
        Some(str_mode) => uucore::mode::parse_mode(str_mode)
            .map_err(|e| format!("invalid mode ({e})"))
            .and_then(|mode| {
                if mode > 0o777 {
                    Err("mode must specify only file permission bits".to_string())
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
        .ok_or_else(|| "missing device type".to_string())
        .and_then(|first_char| match first_char {
            'b' => Ok(FileType::Block),
            'c' | 'u' => Ok(FileType::Character),
            'p' => Ok(FileType::Fifo),
            _ => Err(format!("invalid device type {}", tpe.quote())),
        })
}
