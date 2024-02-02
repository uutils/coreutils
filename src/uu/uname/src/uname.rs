// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (API) nodename osname sysname (options) mnrsv mnrsvo

use clap::{crate_version, Arg, ArgAction, Command};
use platform_info::*;
use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use uucore::{
    error::{UResult, USimpleError},
    format_usage, help_about, help_usage,
};

const ABOUT: &str = help_about!("uname.md");
const USAGE: &str = help_usage!("uname.md");

static ALL: &str = "all";
static KERNEL_NAME: &str = "kernel-name";
static NODENAME: &str = "nodename";
static KERNEL_VERSION: &str = "kernel-version";
static KERNEL_RELEASE: &str = "kernel-release";
static MACHINE: &str = "machine";
static PROCESSOR: &str = "processor";
static HARDWARE_PLATFORM: &str = "hardware-platform";
static OS: &str = "operating-system";

// #[derive(Debug, Clone)]
pub struct UnameOutput<'a> {
    kernel_name: String,
    nodename: String,
    kernel_release: String,
    kernel_version: String,
    machine: String,
    os: String,
    processor: String,
    hardware_platform: String,
    opts: &'a Options,
}
fn format_this(out: &UnameOutput) -> String {
    let mut output = String::new();
    let none = !(out.opts.all
        || out.opts.kernel_name
        || out.opts.nodename
        || out.opts.kernel_release
        || out.opts.kernel_version
        || out.opts.machine
        || out.opts.os
        || out.opts.processor
        || out.opts.hardware_platform);
    if out.opts.kernel_name || out.opts.all || none {
        output.push_str(&out.kernel_name);
        output.push(' ');
    }

    if out.opts.nodename || out.opts.all {
        output.push_str(&out.nodename);
        output.push(' ');
    }

    if out.opts.kernel_release || out.opts.all {
        output.push_str(&out.kernel_release);
        output.push(' ');
    }

    if out.opts.kernel_version || out.opts.all {
        output.push_str(&out.kernel_version);
        output.push(' ');
    }

    if out.opts.machine || out.opts.all {
        output.push_str(&out.machine);
        output.push(' ');
    }

    if out.opts.os || out.opts.all {
        output.push_str(&out.os);
        output.push(' ');
    }

    // This option is unsupported on modern Linux systems
    // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html

    if out.opts.processor {
        output.push_str("unknown");
        output.push(' ');
    }

    // This option is unsupported on modern Linux systems
    // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
    if out.opts.hardware_platform {
        output.push_str("unknown");
        output.push(' ');
    }
    output
}
impl<'a> Display for UnameOutput<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            format_this(self) // self.kernel_name,
                              // self.nodename,
                              // self.kernel_release,
                              // self.kernel_version,
                              // self.machine,
                              // self.os,
                              // self.processor,
                              // self.hardware_platform
        )
    }
}
pub struct Options {
    pub all: bool,
    pub kernel_name: bool,
    pub nodename: bool,
    pub kernel_version: bool,
    pub kernel_release: bool,
    pub machine: bool,
    pub processor: bool,
    pub hardware_platform: bool,
    pub os: bool,
}

pub fn uu_uname(opts: &Options) -> UResult<UnameOutput> {
    let mut output = String::new();
    let uname = PlatformInfo::new().map_err(|_e| USimpleError::new(1, "cannot get system name"))?;
    let none = !(opts.all
        || opts.kernel_name
        || opts.nodename
        || opts.kernel_release
        || opts.kernel_version
        || opts.machine
        || opts.os
        || opts.processor
        || opts.hardware_platform);

    if opts.kernel_name || opts.all || none {
        let kernel_name = &uname.sysname().to_string_lossy();
        output.push_str(kernel_name);
        output.push(' ');
    }

    if opts.nodename || opts.all {
        output.push_str(&uname.nodename().to_string_lossy());
        output.push(' ');
    }

    if opts.kernel_release || opts.all {
        output.push_str(&uname.release().to_string_lossy());
        output.push(' ');
    }

    if opts.kernel_version || opts.all {
        output.push_str(&uname.version().to_string_lossy());
        output.push(' ');
    }

    if opts.machine || opts.all {
        output.push_str(&uname.machine().to_string_lossy());
        output.push(' ');
    }

    if opts.os || opts.all {
        output.push_str(&uname.osname().to_string_lossy());
        output.push(' ');
    }

    // This option is unsupported on modern Linux systems
    // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
    if opts.processor {
        output.push_str("unknown");
        output.push(' ');
    }

    // This option is unsupported on modern Linux systems
    // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
    if opts.hardware_platform {
        output.push_str("unknown");
        output.push(' ');
    }
    let un = UnameOutput {
        kernel_name: uname.sysname().to_string_lossy().to_string(),
        nodename: uname.nodename().to_string_lossy().to_string(),
        kernel_release: uname.release().to_string_lossy().to_string(),
        kernel_version: uname.version().to_string_lossy().to_string(),
        machine: uname.machine().to_string_lossy().to_string(),
        os: uname.osname().to_string_lossy().to_string(),
        processor: "unknown".to_string(),
        hardware_platform: "unknown".to_string(),
        opts,
    };
    // Ok(output)
    Ok(un)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let options = Options {
        all: matches.get_flag(ALL),
        kernel_name: matches.get_flag(KERNEL_NAME),
        nodename: matches.get_flag(NODENAME),
        kernel_release: matches.get_flag(KERNEL_RELEASE),
        kernel_version: matches.get_flag(KERNEL_VERSION),
        machine: matches.get_flag(MACHINE),
        processor: matches.get_flag(PROCESSOR),
        hardware_platform: matches.get_flag(HARDWARE_PLATFORM),
        os: matches.get_flag(OS),
    };
    let output = uu_uname(&options)?;
    // println!("{}", output.trim_end());
    println!("{}", output);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(ALL)
                .short('a')
                .long(ALL)
                .help("Behave as though all of the options -mnrsvo were specified.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(KERNEL_NAME)
                .short('s')
                .long(KERNEL_NAME)
                .alias("sysname") // Obsolescent option in GNU uname
                .help("print the kernel name.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(NODENAME)
                .short('n')
                .long(NODENAME)
                .help(
                    "print the nodename (the nodename may be a name that the system \
                is known by to a communications network).",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(KERNEL_RELEASE)
                .short('r')
                .long(KERNEL_RELEASE)
                .alias("release") // Obsolescent option in GNU uname
                .help("print the operating system release.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(KERNEL_VERSION)
                .short('v')
                .long(KERNEL_VERSION)
                .help("print the operating system version.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(MACHINE)
                .short('m')
                .long(MACHINE)
                .help("print the machine hardware name.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OS)
                .short('o')
                .long(OS)
                .help("print the operating system name.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(PROCESSOR)
                .short('p')
                .long(PROCESSOR)
                .help("print the processor type (non-portable)")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(HARDWARE_PLATFORM)
                .short('i')
                .long(HARDWARE_PLATFORM)
                .help("print the hardware platform (non-portable)")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
}
