//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Colin Warren <me@zv.ms>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: unlink (GNU coreutils) 8.21 */

// spell-checker:ignore (ToDO) lstat IFLNK IFMT IFREG

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use libc::{lstat, stat, unlink};
use libc::{S_IFLNK, S_IFMT, S_IFREG};
use std::ffi::CString;
use std::io::{Error, ErrorKind};
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "Unlink the file at [FILE].";
static OPT_PATH: &str = "FILE";

fn usage() -> String {
    format!("{} [OPTION]... FILE", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let usage = usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let paths: Vec<String> = matches
        .values_of(OPT_PATH)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if paths.is_empty() {
        crash!(
            1,
            "missing operand\nTry `{0} --help` for more information.",
            executable!()
        );
    } else if paths.len() > 1 {
        crash!(
            1,
            "extra operand: '{1}'\nTry `{0} --help` for more information.",
            executable!(),
            paths[1]
        );
    }

    let c_string = CString::new(paths[0].clone()).unwrap(); // unwrap() cannot fail, the string comes from argv so it cannot contain a \0.

    let st_mode = {
        #[allow(deprecated)]
        let mut buf: stat = unsafe { std::mem::uninitialized() };
        let result = unsafe { lstat(c_string.as_ptr(), &mut buf as *mut stat) };

        if result < 0 {
            crash!(1, "Cannot stat '{}': {}", paths[0], Error::last_os_error());
        }

        buf.st_mode & S_IFMT
    };

    let result = if st_mode != S_IFREG && st_mode != S_IFLNK {
        Err(Error::new(
            ErrorKind::Other,
            "Not a regular file or symlink",
        ))
    } else {
        let result = unsafe { unlink(c_string.as_ptr()) };

        if result < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    };

    match result {
        Ok(_) => (),
        Err(e) => {
            crash!(1, "cannot unlink '{0}': {1}", paths[0], e);
        }
    }

    0
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(util_name!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(Arg::with_name(OPT_PATH).hidden(true).multiple(true))
}
