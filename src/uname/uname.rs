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
use uucore::utsname::Uname;

static SYNTAX: &'static str = "[OPTION]...";
static SUMMARY: &'static str = "Print certain system information.  With no OPTION, same as -s.";

#[cfg(target_os = "linux")]
static HOST_OS: &'static str = "GNU/Linux";
#[cfg(target_os = "windows")]
static HOST_OS: &'static str = "Windows NT";
#[cfg(target_os = "freebsd")]
static HOST_OS: &'static str = "FreeBSD";
#[cfg(target_os = "openbsd")]
static HOST_OS: &'static str = "OpenBSD";
#[cfg(target_os = "macos")]
static HOST_OS: &'static str = "Darwin";
#[cfg(target_os = "fuchsia")]
static HOST_OS: &'static str = "Fuchsia";

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = new_coreopts!(SYNTAX, SUMMARY, "");

    opts.optflag("a",
                 "all",
                 "Behave as though all of the options -mnrsv were specified.");
    opts.optflag("s", "sysname", "print the operating system name.");
    opts.optflag("n", "nodename", "print the nodename (the nodename may be a name that the system is known by to a communications network).");
    opts.optflag("r", "kernel-release", "print the operating system release.");
    opts.optflag("v", "kernel-version", "print the operating system version.");
    opts.optflag("m", "machine", "print the machine hardware name.");

    // FIXME: Unimplemented
    // opts.optflag("p", "processor", "print the machine processor architecture name.");
    // opts.optflag("i", "hardware-platform", "print the hardware platform.");

    opts.optflag("o", "operating-system", "print the operating system");
    let argc = args.len();
    let matches = opts.parse(args);
    let uname = Uname::new();
    let mut output = String::new();
    if matches.opt_present("sysname") || matches.opt_present("all") || argc == 1 {
        output.push_str(uname.sysname().as_ref());
        output.push_str(" ");
    }

    if matches.opt_present("nodename") || matches.opt_present("all") {
        output.push_str(uname.nodename().as_ref());
        output.push_str(" ");
    }
    if matches.opt_present("kernel-release") || matches.opt_present("all") {
        output.push_str(uname.release().as_ref());
        output.push_str(" ");
    }
    if matches.opt_present("kernel-version") || matches.opt_present("all") {
        output.push_str(uname.version().as_ref());
        output.push_str(" ");
    }
    if matches.opt_present("machine") || matches.opt_present("all") {
        output.push_str(uname.machine().as_ref());
        output.push_str(" ");
    }
    if matches.opt_present("operating-system") || matches.opt_present("all") {
        output.push_str(HOST_OS);
        output.push_str(" ");
    }
    println!("{}", output.trim());

    0
}
