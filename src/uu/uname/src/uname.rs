// This file is part of the uutils coreutils package.
//
// (c) Joao Oliveira <joaoxsouls@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// last synced with: uname (GNU coreutils) 8.21

// spell-checker:ignore (ToDO) nodename kernelname kernelrelease kernelversion sysname hwplatform mnrsv

use clap::{crate_version, Arg, Command};
use platform_info::*;
use uucore::{
    error::{FromIo, UResult},
    format_usage,
};

const ABOUT: &str = "Print certain system information.  With no OPTION, same as -s.";
const USAGE: &str = "{} [OPTION]...";

pub mod options {
    pub static ALL: &str = "all";
    pub static KERNELNAME: &str = "kernel-name";
    pub static NODENAME: &str = "nodename";
    pub static KERNELVERSION: &str = "kernel-version";
    pub static KERNELRELEASE: &str = "kernel-release";
    pub static MACHINE: &str = "machine";
    pub static PROCESSOR: &str = "processor";
    pub static HWPLATFORM: &str = "hardware-platform";
    pub static OS: &str = "operating-system";
}

#[cfg(all(target_os = "linux", any(target_env = "gnu", target_env = "")))]
const HOST_OS: &str = "GNU/Linux";
#[cfg(all(target_os = "linux", not(any(target_env = "gnu", target_env = ""))))]
const HOST_OS: &str = "Linux";
#[cfg(target_os = "windows")]
const HOST_OS: &str = "Windows NT";
#[cfg(target_os = "freebsd")]
const HOST_OS: &str = "FreeBSD";
#[cfg(target_os = "netbsd")]
const HOST_OS: &str = "NetBSD";
#[cfg(target_os = "openbsd")]
const HOST_OS: &str = "OpenBSD";
#[cfg(target_vendor = "apple")]
const HOST_OS: &str = "Darwin";
#[cfg(target_os = "fuchsia")]
const HOST_OS: &str = "Fuchsia";
#[cfg(target_os = "redox")]
const HOST_OS: &str = "Redox";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let uname =
        PlatformInfo::new().map_err_context(|| "failed to create PlatformInfo".to_string())?;
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

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(Arg::new(options::ALL)
            .short('a')
            .long(options::ALL)
            .help("Behave as though all of the options -mnrsv were specified."))
        .arg(Arg::new(options::KERNELNAME)
            .short('s')
            .long(options::KERNELNAME)
            .alias("sysname") // Obsolescent option in GNU uname
            .help("print the kernel name."))
        .arg(Arg::new(options::NODENAME)
            .short('n')
            .long(options::NODENAME)
            .help("print the nodename (the nodename may be a name that the system is known by to a communications network)."))
        .arg(Arg::new(options::KERNELRELEASE)
            .short('r')
            .long(options::KERNELRELEASE)
            .alias("release") // Obsolescent option in GNU uname
            .help("print the operating system release."))
        .arg(Arg::new(options::KERNELVERSION)
            .short('v')
            .long(options::KERNELVERSION)
            .help("print the operating system version."))
        .arg(Arg::new(options::HWPLATFORM)
            .short('i')
            .long(options::HWPLATFORM)
            .help("print the hardware platform (non-portable)"))
        .arg(Arg::new(options::MACHINE)
            .short('m')
            .long(options::MACHINE)
            .help("print the machine hardware name."))
        .arg(Arg::new(options::PROCESSOR)
            .short('p')
            .long(options::PROCESSOR)
            .help("print the processor type (non-portable)"))
        .arg(Arg::new(options::OS)
            .short('o')
            .long(options::OS)
            .help("print the operating system name."))
}
