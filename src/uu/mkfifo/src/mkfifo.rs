//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use libc::mkfifo;
use std::ffi::CString;
use uucore::InvalidEncodingHandling;

static NAME: &str = "mkfifo";
static USAGE: &str = "mkfifo [OPTION]... NAME...";
static SUMMARY: &str = "Create a FIFO with the given name.";

mod options {
    pub static MODE: &str = "mode";
    pub static SE_LINUX_SECURITY_CONTEXT: &str = "Z";
    pub static CONTEXT: &str = "context";
    pub static FIFO: &str = "fifo";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    if matches.is_present(options::CONTEXT) {
        crash!(1, "--context is not implemented");
    }
    if matches.is_present(options::SE_LINUX_SECURITY_CONTEXT) {
        crash!(1, "-Z is not implemented");
    }

    let mode = match matches.value_of(options::MODE) {
        Some(m) => match usize::from_str_radix(m, 8) {
            Ok(m) => m,
            Err(e) => {
                show_error!("invalid mode: {}", e);
                return 1;
            }
        },
        None => 0o666,
    };

    let fifos: Vec<String> = match matches.values_of(options::FIFO) {
        Some(v) => v.clone().map(|s| s.to_owned()).collect(),
        None => crash!(1, "missing operand"),
    };

    let mut exit_code = 0;
    for f in fifos {
        let err = unsafe {
            let name = CString::new(f.as_bytes()).unwrap();
            mkfifo(name.as_ptr(), mode as libc::mode_t)
        };
        if err == -1 {
            show_error!("cannot create fifo '{}': File exists", f);
            exit_code = 1;
        }
    }

    exit_code
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(util_name!())
        .name(NAME)
        .version(crate_version!())
        .usage(USAGE)
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::MODE)
                .short("m")
                .long(options::MODE)
                .help("file permissions for the fifo")
                .default_value("0666")
                .value_name("0666"),
        )
        .arg(
            Arg::with_name(options::SE_LINUX_SECURITY_CONTEXT)
                .short(options::SE_LINUX_SECURITY_CONTEXT)
                .help("set the SELinux security context to default type"),
        )
        .arg(
            Arg::with_name(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .help(
                    "like -Z, or if CTX is specified then set the SELinux \
                    or SMACK security context to CTX",
                ),
        )
        .arg(Arg::with_name(options::FIFO).hidden(true).multiple(true))
}
