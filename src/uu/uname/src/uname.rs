// This file is part of the uutils coreutils package.
//
// (c) Joao Oliveira <joaoxsouls@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// last synced with: uname (GNU coreutils) 8.21

// spell-checker:ignore (ToDO) nodename kernelname kernelrelease kernelversion sysname hwplatform mnrsv

#[macro_use]
extern crate uucore;

use platform_info::*;

use crate::app::{get_app, options};

pub mod app;

#[cfg(target_os = "linux")]
const HOST_OS: &str = "GNU/Linux";
#[cfg(target_os = "windows")]
const HOST_OS: &str = "Windows NT";
#[cfg(target_os = "freebsd")]
const HOST_OS: &str = "FreeBSD";
#[cfg(target_os = "openbsd")]
const HOST_OS: &str = "OpenBSD";
#[cfg(target_vendor = "apple")]
const HOST_OS: &str = "Darwin";
#[cfg(target_os = "fuchsia")]
const HOST_OS: &str = "Fuchsia";
#[cfg(target_os = "redox")]
const HOST_OS: &str = "Redox";

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = format!("{} [OPTION]...", executable!());
    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let uname = return_if_err!(1, PlatformInfo::new());
    let mut output = String::new();

    let all = matches.is_present(options::ALL);
    let kernelname = matches.is_present(options::KERNELNAME);
    let nodename = matches.is_present(options::NODENAME);
    let kernelrelease = matches.is_present(options::KERNELRELEASE);
    let kernelversion = matches.is_present(options::KERNELVERSION);
    let machine = matches.is_present(options::MACHINE);
    let processor = matches.is_present(options::PROCESSOR);
    let hwplatform = matches.is_present(options::HWPLATFORM);
    let os = matches.is_present(options::OS);

    let none = !(all
        || kernelname
        || nodename
        || kernelrelease
        || kernelversion
        || machine
        || os
        || processor
        || hwplatform);

    if kernelname || all || none {
        output.push_str(&uname.sysname());
        output.push(' ');
    }

    if nodename || all {
        output.push_str(&uname.nodename());
        output.push(' ');
    }
    if kernelrelease || all {
        output.push_str(&uname.release());
        output.push(' ');
    }
    if kernelversion || all {
        output.push_str(&uname.version());
        output.push(' ');
    }
    if machine || all {
        output.push_str(&uname.machine());
        output.push(' ');
    }
    if processor || all {
        // According to https://stackoverflow.com/posts/394271/revisions
        // Most of the time, it returns unknown
        output.push_str("unknown");
        output.push(' ');
    }
    if hwplatform || all {
        // According to https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
        // Most of the time, it returns unknown
        output.push_str("unknown");
        output.push(' ');
    }
    if os || all {
        output.push_str(HOST_OS);
        output.push(' ');
    }
    println!("{}", output.trim_end());

    0
}
