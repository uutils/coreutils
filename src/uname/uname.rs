#![crate_name = "uu_uname"]

// This file is part of the uutils coreutils package.
//
// (c) Joao Oliveira <joaoxsouls@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

// last synced with: uname (GNU coreutils) 8.21

#[macro_use]
extern crate uucore;
extern crate clap;
extern crate platform_info;

use clap::{Arg, App};
use platform_info::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const ABOUT: &'static str = "Print certain system information.  With no OPTION, same as -s.";

const OPT_ALL: &'static str = "all";
const OPT_KERNELNAME: &'static str = "kernel-name";
const OPT_NODENAME: &'static str = "nodename";
const OPT_KERNELVERSION: &'static str = "kernel-version";
const OPT_KERNELRELEASE: &'static str = "kernel-release";
const OPT_MACHINE: &'static str = "machine";

//FIXME: unimplemented options
//const OPT_PROCESSOR: &'static str = "processor";
//const OPT_HWPLATFORM: &'static str = "hardware-platform";
const OPT_OS: &'static str = "operating-system";

#[cfg(target_os = "linux")]
const HOST_OS: &'static str = "GNU/Linux";
#[cfg(target_os = "windows")]
const HOST_OS: &'static str = "Windows NT";
#[cfg(target_os = "freebsd")]
const HOST_OS: &'static str = "FreeBSD";
#[cfg(target_os = "openbsd")]
const HOST_OS: &'static str = "OpenBSD";
#[cfg(target_os = "macos")]
const HOST_OS: &'static str = "Darwin";
#[cfg(target_os = "fuchsia")]
const HOST_OS: &'static str = "Fuchsia";
#[cfg(target_os = "redox")]
const HOST_OS: &'static str = "Redox";

pub fn uumain(args: Vec<String>) -> i32 {

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
            .help("print the operating system name."))
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

        //FIXME: unimplemented options
        // .arg(Arg::with_name(OPT_PROCESSOR)
        //     .short("p")
        //     .long(OPT_PROCESSOR)
        //     .help("print the processor type (non-portable)"))
        // .arg(Arg::with_name(OPT_HWPLATFORM)
        //     .short("i")
        //     .long(OPT_HWPLATFORM)
        //     .help("print the hardware platform (non-portable)"))
        .arg(Arg::with_name(OPT_MACHINE)
            .short("m")
            .long(OPT_MACHINE)
            .help("print the machine hardware name."))
        .get_matches_from(&args);

    let argc = args.len();
    let uname = return_if_err!(1, PlatformInfo::new());
    let mut output = String::new();

    if matches.is_present(OPT_KERNELNAME) || matches.is_present(OPT_ALL) || argc == 1 {
        output.push_str(&uname.sysname());
        output.push_str(" ");
    }

    if matches.is_present(OPT_NODENAME) || matches.is_present(OPT_ALL) {
        output.push_str(&uname.nodename());
        output.push_str(" ");
    }
    if matches.is_present(OPT_KERNELRELEASE) || matches.is_present(OPT_ALL) {
        output.push_str(&uname.release());
        output.push_str(" ");
    }
    if matches.is_present(OPT_KERNELVERSION) || matches.is_present(OPT_ALL) {
        output.push_str(&uname.version());
        output.push_str(" ");
    }
    if matches.is_present(OPT_MACHINE) || matches.is_present(OPT_ALL) {
        output.push_str(&uname.machine());
        output.push_str(" ");
    }
    if matches.is_present(OPT_OS) || matches.is_present(OPT_ALL) {
        output.push_str(HOST_OS);
        output.push_str(" ");
    }
    println!("{}", output.trim_right());

    0
}
