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

static ALL: &str = "all";
static KERNEL_NAME: &str = "kernel-name";
static NODENAME: &str = "nodename";
static KERNEL_VERSION: &str = "kernel-version";
static KERNEL_RELEASE: &str = "kernel-release";
static MACHINE: &str = "machine";
static PROCESSOR: &str = "processor";
static HARDWARE_PLATFORM: &str = "hardware-platform";
static OS: &str = "operating-system";

pub struct UNameOutput {
    pub kernel_name: Option<String>,
    pub nodename: Option<String>,
    pub kernel_release: Option<String>,
    pub kernel_version: Option<String>,
    pub machine: Option<String>,
    pub os: Option<String>,
    pub processor: Option<String>,
    pub hardware_platform: Option<String>,
}

impl UNameOutput {
    fn display(&self) -> String {
        let mut output = String::new();
        [
            self.kernel_name.as_ref(),
            self.nodename.as_ref(),
            self.kernel_release.as_ref(),
            self.kernel_version.as_ref(),
            self.machine.as_ref(),
            self.os.as_ref(),
            self.processor.as_ref(),
            self.hardware_platform.as_ref(),
        ]
        .iter()
        .for_each(|name| {
            if let Some(name) = name {
                output.push_str(name);
                output.push(' ');
            }
        });
        output
    }
    pub fn new(opts: &Options) -> UResult<Self> {
        let uname =
            PlatformInfo::new().map_err(|_e| USimpleError::new(1, "cannot get system name"))?;
        let none = !(opts.all
            || opts.kernel_name
            || opts.nodename
            || opts.kernel_release
            || opts.kernel_version
            || opts.machine
            || opts.os
            || opts.processor
            || opts.hardware_platform);

        let kernel_name = if opts.kernel_name || opts.all || none {
            Some(uname.sysname().to_string_lossy().to_string())
        } else {
            None
        };

        let nodename = if opts.nodename || opts.all {
            Some(uname.nodename().to_string_lossy().to_string())
        } else {
            None
        };

        let kernel_release = if opts.kernel_release || opts.all {
            Some(uname.release().to_string_lossy().to_string())
        } else {
            None
        };
        let kernel_version = if opts.kernel_version || opts.all {
            Some(uname.version().to_string_lossy().to_string())
        } else {
            None
        };

        let machine = if opts.machine || opts.all {
            Some(uname.machine().to_string_lossy().to_string())
        } else {
            None
        };

        let os = if opts.os || opts.all {
            Some(uname.osname().to_string_lossy().to_string())
        } else {
            None
        };

        // This option is unsupported on modern Linux systems
        // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
        let processor = if opts.processor {
            Some("unknown".to_string())
        } else {
            None
        };

        // This option is unsupported on modern Linux systems
        // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html

        let hardware_platform = if opts.hardware_platform {
            Some("unknown".to_string())
        } else {
            None
        };

        Ok(Self {
            kernel_name,
            nodename,
            kernel_release,
            kernel_version,
            machine,
            os,
            processor,
            hardware_platform,
        })
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
    let output = UNameOutput::new(&options)?;
    println!("{}", output.display().trim_end());
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
