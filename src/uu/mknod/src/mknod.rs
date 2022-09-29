// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) parsemode makedev sysmacros perror IFBLK IFCHR IFIFO

use clap::{crate_version, value_parser, Arg, ArgMatches, Command};
use libc::{dev_t, mode_t};
use libc::{S_IFBLK, S_IFCHR, S_IFIFO, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};
use std::ffi::CString;

use uucore::display::Quotable;
use uucore::error::{set_exit_code, UResult, USimpleError, UUsageError};
use uucore::format_usage;

static ABOUT: &str = "Create the special file NAME of the given TYPE.";
static USAGE: &str = "{} [OPTION]... NAME TYPE [MAJOR MINOR]";
static LONG_HELP: &str = "Mandatory arguments to long options are mandatory for short options too.
-m, --mode=MODE    set file permission bits to MODE, not a=rw - umask
--help     display this help and exit
--version  output version information and exit

Both MAJOR and MINOR must be specified when TYPE is b, c, or u, and they
must be omitted when TYPE is p.  If MAJOR or MINOR begins with 0x or 0X,
it is interpreted as hexadecimal; otherwise, if it begins with 0, as octal;
otherwise, as decimal.  TYPE may be:

b      create a block (buffered) special file
c, u   create a character (unbuffered) special file
p      create a FIFO

NOTE: your shell may have its own version of mknod, which usually supersedes
the version described here.  Please refer to your shell's documentation
for details about the options it supports.
";

const MODE_RW_UGO: mode_t = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;

#[inline(always)]
fn makedev(maj: u64, min: u64) -> dev_t {
    // pick up from <sys/sysmacros.h>
    ((min & 0xff) | ((maj & 0xfff) << 8) | ((min & !0xff) << 12) | ((maj & !0xfff) << 32)) as dev_t
}

#[cfg(windows)]
fn _mknod(file_name: &str, mode: mode_t, dev: dev_t) -> i32 {
    panic!("Unsupported for windows platform")
}

#[derive(Clone, PartialEq)]
enum FileType {
    Block,
    Character,
    Fifo,
}

#[cfg(unix)]
fn _mknod(file_name: &str, mode: mode_t, dev: dev_t) -> i32 {
    let c_str = CString::new(file_name).expect("Failed to convert to CString");

    // the user supplied a mode
    let set_umask = mode & MODE_RW_UGO != MODE_RW_UGO;

    unsafe {
        // store prev umask
        let last_umask = if set_umask { libc::umask(0) } else { 0 };

        let errno = libc::mknod(c_str.as_ptr(), mode, dev);

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
        errno
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();
    // Linux-specific options, not implemented
    // opts.optflag("Z", "", "set the SELinux security context to default type");
    // opts.optopt("", "context", "like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX");

    let matches = uu_app().try_get_matches_from(args)?;

    let mode = get_mode(&matches).map_err(|e| USimpleError::new(1, e))?;

    let file_name = matches
        .get_one::<String>("name")
        .expect("Missing argument 'NAME'");

    let file_type = matches.get_one::<FileType>("type").unwrap();

    if *file_type == FileType::Fifo {
        if matches.contains_id("major") || matches.contains_id("minor") {
            Err(UUsageError::new(
                1,
                "Fifos do not have major and minor device numbers.",
            ))
        } else {
            let exit_code = _mknod(file_name, S_IFIFO | mode, 0);
            set_exit_code(exit_code);
            Ok(())
        }
    } else {
        match (
            matches.get_one::<u64>("major"),
            matches.get_one::<u64>("minor"),
        ) {
            (_, None) | (None, _) => {
                return Err(UUsageError::new(
                    1,
                    "Special files require major and minor device numbers.",
                ));
            }
            (Some(&major), Some(&minor)) => {
                let dev = makedev(major, minor);
                let exit_code = match file_type {
                    FileType::Block => _mknod(file_name, S_IFBLK | mode, dev),
                    FileType::Character => _mknod(file_name, S_IFCHR | mode, dev),
                    _ => unreachable!("file_type was validated to be only block or character"),
                };
                set_exit_code(exit_code);
                Ok(())
            }
        }
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .after_help(LONG_HELP)
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new("mode")
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
            Arg::new("type")
                .value_name("TYPE")
                .help("type of the new file (b, c, u or p)")
                .required(true)
                .value_parser(parse_type),
        )
        .arg(
            Arg::new("major")
                .value_name("MAJOR")
                .help("major file type")
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new("minor")
                .value_name("MINOR")
                .help("minor file type")
                .value_parser(value_parser!(u64)),
        )
}

fn get_mode(matches: &ArgMatches) -> Result<mode_t, String> {
    match matches.get_one::<String>("mode") {
        None => Ok(MODE_RW_UGO),
        Some(str_mode) => uucore::mode::parse_mode(str_mode)
            .map_err(|e| format!("invalid mode ({})", e))
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
