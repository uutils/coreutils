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
        for name in [
            self.kernel_name.as_ref(),
            self.nodename.as_ref(),
            self.kernel_release.as_ref(),
            self.kernel_version.as_ref(),
            self.machine.as_ref(),
            self.os.as_ref(),
            self.processor.as_ref(),
            self.hardware_platform.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            output.push_str(name);
            output.push(' ');
        }
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

        let kernel_name = (opts.kernel_name || opts.all || none)
            .then(|| uname.sysname().to_string_lossy().to_string());

        let nodename =
            (opts.nodename || opts.all).then(|| uname.nodename().to_string_lossy().to_string());

        let kernel_release = (opts.kernel_release || opts.all)
            .then(|| uname.release().to_string_lossy().to_string());

        let kernel_version = (opts.kernel_version || opts.all)
            .then(|| uname.version().to_string_lossy().to_string());

        let machine =
            (opts.machine || opts.all).then(|| uname.machine().to_string_lossy().to_string());

        let os = (opts.os || opts.all).then(|| uname.osname().to_string_lossy().to_string());

        // This option is unsupported on modern Linux systems
        // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
        let processor = opts.processor.then(|| "unknown".to_string());

        // This option is unsupported on modern Linux systems
        // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
        let hardware_platform = opts.hardware_platform.then(|| "unknown".to_string());

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
        all: matches.get_flag(options::ALL),
        kernel_name: matches.get_flag(options::KERNEL_NAME),
        nodename: matches.get_flag(options::NODENAME),
        kernel_release: matches.get_flag(options::KERNEL_RELEASE),
        kernel_version: matches.get_flag(options::KERNEL_VERSION),
        machine: matches.get_flag(options::MACHINE),
        processor: matches.get_flag(options::PROCESSOR),
        hardware_platform: matches.get_flag(options::HARDWARE_PLATFORM),
        os: matches.get_flag(options::OS),
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
