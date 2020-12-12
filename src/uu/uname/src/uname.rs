// This file is part of the uutils coreutils package.
//
// (c) Joao Oliveira <joaoxsouls@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// last synced with: uname (GNU coreutils) 8.21

// spell-checker:ignore (ToDO) nodename kernelname kernelrelease kernelversion sysname hwplatform mnrsv

extern crate clap;
extern crate platform_info;
#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use platform_info::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str = "Print certain system information.  With no OPTION, same as -s.";

const OPT_ALL: &str = "all";
const OPT_KERNELNAME: &str = "kernel-name";
const OPT_NODENAME: &str = "nodename";
const OPT_KERNELVERSION: &str = "kernel-version";
const OPT_KERNELRELEASE: &str = "kernel-release";
const OPT_MACHINE: &str = "machine";
const OPT_PROCESSOR: &str = "processor";
const OPT_HWPLATFORM: &str = "hardware-platform";

//FIXME: unimplemented options
//const OPT_PROCESSOR: &'static str = "processor";
//const OPT_HWPLATFORM: &'static str = "hardware-platform";
const OPT_OS: &str = "operating-system";

#[cfg(target_os = "linux")]
const HOST_OS: &str = "GNU/Linux";
#[cfg(target_os = "windows")]
const HOST_OS: &str = "Windows NT";
#[cfg(target_os = "freebsd")]
const HOST_OS: &str = "FreeBSD";
#[cfg(target_os = "openbsd")]
const HOST_OS: &str = "OpenBSD";
#[cfg(target_os = "macos")]
const HOST_OS: &str = "Darwin";
#[cfg(target_os = "fuchsia")]
const HOST_OS: &str = "Fuchsia";
#[cfg(target_os = "redox")]
const HOST_OS: &str = "Redox";

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = format!("{} [OPTION]...", executable!());
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(Arg::with_name(OPT_ALL)
            .short("a")
            .long(OPT_ALL)
            .help("Behave as though all of the options -mnrsv were specified."))
        .arg(Arg::with_name(OPT_KERNELNAME)
            .short("s")
            .long(OPT_KERNELNAME)
            .alias("sysname") // Obsolescent option in GNU uname
            .help("print the kernel name."))
        .arg(Arg::with_name(OPT_NODENAME)
            .short("n")
            .long(OPT_NODENAME)
            .help("print the nodename (the nodename may be a name that the system is known by to a communications network)."))
        .arg(Arg::with_name(OPT_KERNELRELEASE)
            .short("r")
            .long(OPT_KERNELRELEASE)
            .alias("release") // Obsolescent option in GNU uname
            .help("print the operating system release."))
        .arg(Arg::with_name(OPT_KERNELVERSION)
            .short("v")
            .long(OPT_KERNELVERSION)
            .help("print the operating system version."))
        .arg(Arg::with_name(OPT_HWPLATFORM)
            .short("i")
            .long(OPT_HWPLATFORM)
            .help("print the hardware platform (non-portable)"))
        .arg(Arg::with_name(OPT_MACHINE)
            .short("m")
            .long(OPT_MACHINE)
            .help("print the machine hardware name."))
        .arg(Arg::with_name(OPT_PROCESSOR)
            .short("p")
            .long(OPT_PROCESSOR)
            .help("print the processor type (non-portable)"))
        .arg(Arg::with_name(OPT_OS)
            .short("o")
            .long(OPT_OS)
            .help("print the operating system name."))
        .get_matches_from(args);

    let uname = return_if_err!(1, PlatformInfo::new());
    let mut output = String::new();

    let all = matches.is_present(OPT_ALL);
    let kernelname = matches.is_present(OPT_KERNELNAME);
    let nodename = matches.is_present(OPT_NODENAME);
    let kernelrelease = matches.is_present(OPT_KERNELRELEASE);
    let kernelversion = matches.is_present(OPT_KERNELVERSION);
    let machine = matches.is_present(OPT_MACHINE);
    let processor = matches.is_present(OPT_PROCESSOR);
    let hwplatform = matches.is_present(OPT_HWPLATFORM);
    let os = matches.is_present(OPT_OS);

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
