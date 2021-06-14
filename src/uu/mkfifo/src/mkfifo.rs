//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use libc::mkfifo;
use std::ffi::CString;
use uucore::InvalidEncodingHandling;

use crate::app::{get_app, options};

mod app;

fn get_usage() -> String {
    format!("{} [OPTION]... NAME...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = get_app(executable!())
        .usage(usage.as_str())
        .get_matches_from(args);

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
