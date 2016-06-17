#![crate_name = "uu_mknod"]

// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

extern crate getopts;
extern crate libc;

mod parsemode;

#[macro_use]
extern crate uucore;

use libc::{mode_t, dev_t};
use libc::{S_IRUSR, S_IWUSR, S_IRGRP, S_IWGRP, S_IROTH, S_IWOTH, S_IFIFO, S_IFBLK, S_IFCHR};

use getopts::Options;
use std::io::Write;

use std::ffi::CString;

static NAME: &'static str = "mknod";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

const MODE_RW_UGO: mode_t = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;

#[inline(always)]
fn makedev(maj: u64, min: u64) -> dev_t {
    // pick up from <sys/sysmacros.h>
    ((min & 0xff) | ((maj & 0xfff) << 8) | (((min & !0xff)) << 12) |
     (((maj & !0xfff)) << 32)) as dev_t
}

#[cfg(windows)]
fn _makenod(path: CString, mode: mode_t, dev: dev_t) -> i32 {
    panic!("Unsupported for windows platform")
}

#[cfg(unix)]
fn _makenod(path: CString, mode: mode_t, dev: dev_t) -> i32 {
    unsafe { libc::mknod(path.as_ptr(), mode, dev) }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    // Linux-specific options, not implemented
    // opts.optflag("Z", "", "set the SELinux security context to default type");
    // opts.optopt("", "context", "like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX");
    opts.optopt("m",
                "mode",
                "set file permission bits to MODE, not a=rw - umask",
                "MODE");

    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}\nTry '{} --help' for more information.", f, NAME),
    };

    if matches.opt_present("help") {
        println!(
"Usage: {0} [OPTION]... NAME TYPE [MAJOR MINOR]

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
for details about the options it supports.", NAME);
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let mut last_umask: mode_t = 0;
    let mut newmode: mode_t = MODE_RW_UGO;
    if matches.opt_present("mode") {
        match parsemode::parse_mode(matches.opt_str("mode")) {
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
    }

    let mut ret = 0i32;
    match matches.free.len() {
        0 => disp_err!("missing operand"),
        1 => disp_err!("missing operand after ‘{}’", matches.free[0]),
        _ => {
            let args = &matches.free;
            let c_str = CString::new(args[0].as_str()).expect("Failed to convert to CString");

            // Only check the first character, to allow mnemonic usage like
            // 'mknod /dev/rst0 character 18 0'.
            let ch = args[1].chars().nth(0).expect("Failed to get the first char");

            if ch == 'p' {
                if args.len() > 2 {
                    show_info!("{}: extra operand ‘{}’", NAME, args[2]);
                    if args.len() == 4 {
                        eprintln!("Fifos do not have major and minor device numbers.");
                    }
                    eprintln!("Try '{} --help' for more information.", NAME);
                    return 1;
                }

                ret = _makenod(c_str, S_IFIFO | newmode, 0);
            } else {
                if args.len() < 4 {
                    show_info!("missing operand after ‘{}’", args[args.len() - 1]);
                    if args.len() == 2 {
                        eprintln!("Special files require major and minor device numbers.");
                    }
                    eprintln!("Try '{} --help' for more information.", NAME);
                    return 1;
                } else if args.len() > 4 {
                    disp_err!("extra operand ‘{}’", args[4]);
                    return 1;
                } else if !"bcu".contains(ch) {
                    disp_err!("invalid device type ‘{}’", args[1]);
                    return 1;
                }

                let maj = args[2].parse::<u64>();
                let min = args[3].parse::<u64>();
                if maj.is_err() {
                    show_info!("invalid major device number ‘{}’", args[2]);
                    return 1;
                } else if min.is_err() {
                    show_info!("invalid minor device number ‘{}’", args[3]);
                    return 1;
                }

                let (maj, min) = (maj.unwrap(), min.unwrap());
                let dev = makedev(maj, min);
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
