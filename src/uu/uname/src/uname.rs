// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (API) nodename osname sysname (options) mnrsv mnrsvo

use clap::{crate_version, Arg, ArgAction, Command};
use platform_info::*;
use uucore::{
    error::{UResult, USimpleError},
    format_usage, help_about, help_usage,
};

const ABOUT: &str = help_about!("uname.md");
const USAGE: &str = help_usage!("uname.md");

pub mod options {
    pub static ALL: &str = "all";
    pub static KERNEL_NAME: &str = "kernel-name";
    pub static NODENAME: &str = "nodename";
    pub static KERNEL_VERSION: &str = "kernel-version";
    pub static KERNEL_RELEASE: &str = "kernel-release";
    pub static MACHINE: &str = "machine";
    pub static PROCESSOR: &str = "processor";
    pub static HARDWARE_PLATFORM: &str = "hardware-platform";
    pub static OS: &str = "operating-system";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let uname = PlatformInfo::new().map_err(|_e| USimpleError::new(1, "cannot get system name"))?;

    let mut output = String::new();

    let all = matches.get_flag(options::ALL);
    let kernel_name = matches.get_flag(options::KERNEL_NAME);
    let nodename = matches.get_flag(options::NODENAME);
    let kernel_release = matches.get_flag(options::KERNEL_RELEASE);
    let kernel_version = matches.get_flag(options::KERNEL_VERSION);
    let machine = matches.get_flag(options::MACHINE);
    let processor = matches.get_flag(options::PROCESSOR);
    let hardware_platform = matches.get_flag(options::HARDWARE_PLATFORM);
    let os = matches.get_flag(options::OS);

    let none = !(all
        || kernel_name
        || nodename
        || kernel_release
        || kernel_version
        || machine
        || os
        || processor
        || hardware_platform);

    if kernel_name || all || none {
        output.push_str(&uname.sysname().to_string_lossy());
        output.push(' ');
    }

    if nodename || all {
        output.push_str(&uname.nodename().to_string_lossy());
        output.push(' ');
    }

    if kernel_release || all {
        output.push_str(&uname.release().to_string_lossy());
        output.push(' ');
    }

    if kernel_version || all {
        output.push_str(&uname.version().to_string_lossy());
        output.push(' ');
    }

    if machine || all {
        output.push_str(&uname.machine().to_string_lossy());
        output.push(' ');
    }

    if os || all {
        output.push_str(&uname.osname().to_string_lossy());
        output.push(' ');
    }

    // This option is unsupported on modern Linux systems
    // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
    if processor {
        output.push_str("unknown");
        output.push(' ');
    }

    // This option is unsupported on modern Linux systems
    // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
    if hardware_platform {
        output.push_str("unknown");
        output.push(' ');
    }

    println!("{}", output.trim_end());

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("Behave as though all of the options -mnrsvo were specified.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KERNEL_NAME)
                .short('s')
                .long(options::KERNEL_NAME)
                .alias("sysname") // Obsolescent option in GNU uname
                .help("print the kernel name.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NODENAME)
                .short('n')
                .long(options::NODENAME)
                .help(
                    "print the nodename (the nodename may be a name that the system \
                is known by to a communications network).",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KERNEL_RELEASE)
                .short('r')
                .long(options::KERNEL_RELEASE)
                .alias("release") // Obsolescent option in GNU uname
                .help("print the operating system release.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KERNEL_VERSION)
                .short('v')
                .long(options::KERNEL_VERSION)
                .help("print the operating system version.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MACHINE)
                .short('m')
                .long(options::MACHINE)
                .help("print the machine hardware name.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OS)
                .short('o')
                .long(options::OS)
                .help("print the operating system name.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PROCESSOR)
                .short('p')
                .long(options::PROCESSOR)
                .help("print the processor type (non-portable)")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(options::HARDWARE_PLATFORM)
                .short('i')
                .long(options::HARDWARE_PLATFORM)
                .help("print the hardware platform (non-portable)")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
}
