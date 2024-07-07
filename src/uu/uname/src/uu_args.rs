// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

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
