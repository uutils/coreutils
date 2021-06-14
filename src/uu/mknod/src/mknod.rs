// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) parsemode makedev sysmacros perror IFBLK IFCHR IFIFO

#[macro_use]
extern crate uucore;

use std::ffi::CString;

use clap::ArgMatches;
use libc::{dev_t, mode_t};
use libc::{S_IFBLK, S_IFCHR, S_IFIFO, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};

use uucore::InvalidEncodingHandling;

use crate::app::get_app;

pub mod app;

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
            let c_str = CString::new(executable!()).expect("Failed to convert to CString");
            // shows the error from the mknod syscall
            libc::perror(c_str.as_ptr());
        }
        errno
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();
    // Linux-specific options, not implemented
    // opts.optflag("Z", "", "set the SELinux security context to default type");
    // opts.optopt("", "context", "like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX");

    let matches = get_app(executable!()).get_matches_from(args);

    let mode = match get_mode(&matches) {
        Ok(mode) => mode,
        Err(err) => {
            show_error!("{}", err);
            return 1;
        }
    };

    let file_name = matches.value_of("name").expect("Missing argument 'NAME'");

    // Only check the first character, to allow mnemonic usage like
    // 'mknod /dev/rst0 character 18 0'.
    let ch = matches
        .value_of("type")
        .expect("Missing argument 'TYPE'")
        .chars()
        .next()
        .expect("Failed to get the first char");

    if ch == 'p' {
        if matches.is_present("major") || matches.is_present("minor") {
            eprintln!("Fifos do not have major and minor device numbers.");
            eprintln!("Try '{} --help' for more information.", executable!());
            1
        } else {
            _mknod(file_name, S_IFIFO | mode, 0)
        }
    } else {
        match (matches.value_of("major"), matches.value_of("minor")) {
            (None, None) | (_, None) | (None, _) => {
                eprintln!("Special files require major and minor device numbers.");
                eprintln!("Try '{} --help' for more information.", executable!());
                1
            }
            (Some(major), Some(minor)) => {
                let major = major.parse::<u64>().expect("validated by clap");
                let minor = minor.parse::<u64>().expect("validated by clap");

                let dev = makedev(major, minor);
                if ch == 'b' {
                    // block special file
                    _mknod(file_name, S_IFBLK | mode, dev)
                } else if ch == 'c' || ch == 'u' {
                    // char special file
                    _mknod(file_name, S_IFCHR | mode, dev)
                } else {
                    unreachable!("{} was validated to be only b, c or u", ch);
                }
            }
        }
    }
}

fn get_mode(matches: &ArgMatches) -> Result<mode_t, String> {
    match matches.value_of("mode") {
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
