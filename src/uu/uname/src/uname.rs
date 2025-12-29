// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (API) nodename osname sysname (options) mnrsv mnrsvo

use std::ffi::{OsStr, OsString};

use clap::{Arg, ArgAction, Command};
use platform_info::*;
use uucore::display::println_verbatim;
use uucore::translate;
use uucore::{
    error::{UResult, USimpleError},
    format_usage,
};

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
    pub kernel_name: Option<OsString>,
    pub nodename: Option<OsString>,
    pub kernel_release: Option<OsString>,
    pub kernel_version: Option<OsString>,
    pub machine: Option<OsString>,
    pub os: Option<OsString>,
    pub processor: Option<OsString>,
    pub hardware_platform: Option<OsString>,
}

impl UNameOutput {
    fn display(&self) -> OsString {
        [
            self.kernel_name.as_ref(),
            self.nodename.as_ref(),
            self.kernel_release.as_ref(),
            self.kernel_version.as_ref(),
            self.machine.as_ref(),
            self.processor.as_ref(),
            self.hardware_platform.as_ref(),
            self.os.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|name| name.as_os_str())
        .collect::<Vec<_>>()
        .join(OsStr::new(" "))
    }

    pub fn new(opts: &Options) -> UResult<Self> {
        let uname = PlatformInfo::new()
            .map_err(|_e| USimpleError::new(1, translate!("uname-error-cannot-get-system-name")))?;
        let none = !(opts.all
            || opts.kernel_name
            || opts.nodename
            || opts.kernel_release
            || opts.kernel_version
            || opts.machine
            || opts.os
            || opts.processor
            || opts.hardware_platform);

        let kernel_name =
            (opts.kernel_name || opts.all || none).then(|| uname.sysname().to_owned());

        let nodename = (opts.nodename || opts.all).then(|| uname.nodename().to_owned());

        let kernel_release = (opts.kernel_release || opts.all).then(|| uname.release().to_owned());

        let kernel_version = (opts.kernel_version || opts.all).then(|| uname.version().to_owned());

        let machine = (opts.machine || opts.all).then(|| uname.machine().to_owned());

        let os = (opts.os || opts.all).then(|| uname.osname().to_owned());

        // This option is unsupported on modern Linux systems
        // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
        let processor = opts.processor.then(|| translate!("uname-unknown").into());

        // This option is unsupported on modern Linux systems
        // See: https://lists.gnu.org/archive/html/bug-coreutils/2005-09/msg00063.html
        let hardware_platform = opts
            .hardware_platform
            .then(|| translate!("uname-unknown").into());

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
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

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
    println_verbatim(output.display().as_os_str()).unwrap();
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("uname-about"))
        .override_usage(format_usage(&translate!("uname-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help(translate!("uname-help-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KERNEL_NAME)
                .short('s')
                .long(options::KERNEL_NAME)
                .alias("sysname") // Obsolescent option in GNU uname
                .help(translate!("uname-help-kernel-name"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NODENAME)
                .short('n')
                .long(options::NODENAME)
                .help(translate!("uname-help-nodename"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KERNEL_RELEASE)
                .short('r')
                .long(options::KERNEL_RELEASE)
                .alias("release") // Obsolescent option in GNU uname
                .help(translate!("uname-help-kernel-release"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KERNEL_VERSION)
                .short('v')
                .long(options::KERNEL_VERSION)
                .help(translate!("uname-help-kernel-version"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MACHINE)
                .short('m')
                .long(options::MACHINE)
                .help(translate!("uname-help-machine"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OS)
                .short('o')
                .long(options::OS)
                .help(translate!("uname-help-os"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PROCESSOR)
                .short('p')
                .long(options::PROCESSOR)
                .help(translate!("uname-help-processor"))
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(options::HARDWARE_PLATFORM)
                .short('i')
                .long(options::HARDWARE_PLATFORM)
                .help(translate!("uname-help-hardware-platform"))
                .action(ArgAction::SetTrue)
                .hide(true),
        )
}
