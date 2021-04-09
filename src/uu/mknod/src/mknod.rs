// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) parsemode makedev sysmacros makenod newmode perror IFBLK IFCHR IFIFO

#[macro_use]
extern crate uucore;

use clap::{App, Arg, ArgMatches};

use libc::{dev_t, mode_t};
use libc::{S_IFBLK, S_IFCHR, S_IFIFO, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};

use getopts::Options;

use std::ffi::CString;

static NAME: &str = "mknod";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static USAGE: &str = "
Usage: {0} [OPTION]... NAME TYPE [MAJOR MINOR]

Mandatory arguments to long options are mandatory for short options too.
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
fn _makenod(path: CString, mode: mode_t, dev: dev_t) -> i32 {
    panic!("Unsupported for windows platform")
}

#[cfg(unix)]
fn _makenod(path: CString, mode: mode_t, dev: dev_t) -> i32 {
    unsafe { libc::mknod(path.as_ptr(), mode, dev) }
}

fn valid_type(tpe: String) -> Result<(), String> {
    let first_char = tpe.chars()[0];
    if vec!['b', 'c', 'u', 'p'].contains(first_char) {
        Ok(())
    } else {
        Err(format!("invalid device type ‘{}’", tpe));
    }
}

fn valid_u64(tpe: &str, num: String) -> Result<(), String> {
    num
        .parse::<u64>()
        .map(|_| ())
        .map_err(|_| format!("invalid {} device number ‘{}’", tpe, num))
}

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {

    // Linux-specific options, not implemented
    // opts.optflag("Z", "", "set the SELinux security context to default type");
    // opts.optopt("", "context", "like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX");

    let matches = App::new(executable!())
        .version(VERSION)
        // FIXME: are both needed?
        .usage(USAGE)
        .help(USAGE)
        .arg(
            Arg::with_name("mode")
                .short("m")
                .long("mode")
                .value_name("MODE")
                .help("set file permission bits to MODE, not a=rw - umask"),
        )
        .arg(
            Arg::with_name("name")
                .value_name("NAME")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::with_name("type")
                .value_name("TYPE")
                .required(true)
                .validator(valid_type)
                .index(2)
        )
        .arg(
            Arg::default()
                .value_name("MAJOR")
                .validator(|m| valid_u64("major", m))
                .index(3)
        )
        .arg(
            Arg::default()
                .value_name("MINOR")
                .validator(|m| valid_u64("minor", m))
                .index(4)
        )
        .get_matches_from(args);


    let mut last_umask: mode_t = 0;
    let newmode = if let Some(mode) = matches.value_of("mode") {
        match uucore::mode::parse_mode(mode) {
            Ok(parsed) => {
                if parsed > 0o777 {
                    show_info!("mode must specify only file permission bits");
                    return 1;
                }
                newmode = parsed;
            }
            Err(e) => {
                show_info!("{}", e);
                return 1;
            }
        }
        unsafe {
            last_umask = libc::umask(0);
        }
    } else {
        MODE_RW_UGO
    };

    let mut ret = 0i32;
    match matches.free.len() {
        0 => show_usage_error!("missing operand"),
        1 => show_usage_error!("missing operand after ‘{}’", matches.free[0]),
        _ => {
            let args = &matches.free;
            let name = matches
                .value_of("name")
                .unwrap(); // required arg
            let c_str = CString::new(&name).expect("Failed to convert to CString");

            // Only check the first character, to allow mnemonic usage like
            // 'mknod /dev/rst0 character 18 0'.
            let ch = matches.value_of("type")
                .unwrap() // required arg
                .chars()
                .next()
                .expect("Failed to get the first char");

            if ch == 'p' {
                if matches.is_present("major") || matches.is_present("minor") {
                    eprintln!("Fifos do not have major and minor device numbers.");
                    eprintln!("Try '{} --help' for more information.", NAME);
                    return 1;
                }

                ret = _makenod(c_str, S_IFIFO | newmode, 0);
            } else {
                match (matches.value_of("major"), matches.value_of("minor")) {
                    (None, None) | (_, None) | (None, _) => {
                        show_info!("missing operand after ‘{}’", args[args.len() - 1]);
                        if args.len() == 2 {
                            eprintln!("Special files require major and minor device numbers.");
                        }
                        eprintln!("Try '{} --help' for more information.", NAME);
                        return 1;
                    }
                    (Some(major), Some(minor)) => {
                        let major = major.parse::<u64>().unwrap(); // validator above
                        let minor = minor.parse::<u64>().unwrap(); // validator above

                        let dev = makedev(major, minor);
                        if ch == 'b' {
                            // block special file
                            ret = _makenod(c_str, S_IFBLK | newmode, dev);
                        } else {
                            // char special file
                            ret = _makenod(c_str, S_IFCHR | newmode, dev);
                        }
                    }
                }
            }
        }
    }

    if last_umask != 0 {
        unsafe {
            libc::umask(last_umask);
        }
    }
    if ret == -1 {
        let c_str = CString::new(format!("{}: {}", NAME, matches.free[0]).as_str())
            .expect("Failed to convert to CString");
        unsafe {
            libc::perror(c_str.as_ptr());
        }
    }

    ret
}
