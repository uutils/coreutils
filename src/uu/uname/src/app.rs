// spell-checker:ignore (ToDO) nodename kernelname kernelrelease kernelversion sysname hwplatform mnrsv
use clap::{crate_version, App, Arg};

const ABOUT: &str = "Print certain system information.  With no OPTION, same as -s.";

pub mod options {
    pub const ALL: &str = "all";
    pub const KERNELNAME: &str = "kernel-name";
    pub const NODENAME: &str = "nodename";
    pub const KERNELVERSION: &str = "kernel-version";
    pub const KERNELRELEASE: &str = "kernel-release";
    pub const MACHINE: &str = "machine";
    pub const PROCESSOR: &str = "processor";
    pub const HWPLATFORM: &str = "hardware-platform";
    pub const OS: &str = "operating-system";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
    .version(crate_version!())
    .about(ABOUT)
    .arg(Arg::with_name(options::ALL)
        .short("a")
        .long(options::ALL)
        .help("Behave as though all of the options -mnrsv were specified."))
    .arg(Arg::with_name(options::KERNELNAME)
        .short("s")
        .long(options::KERNELNAME)
        .alias("sysname") // Obsolescent option in GNU uname
        .help("print the kernel name."))
    .arg(Arg::with_name(options::NODENAME)
        .short("n")
        .long(options::NODENAME)
        .help("print the nodename (the nodename may be a name that the system is known by to a communications network)."))
    .arg(Arg::with_name(options::KERNELRELEASE)
        .short("r")
        .long(options::KERNELRELEASE)
        .alias("release") // Obsolescent option in GNU uname
        .help("print the operating system release."))
    .arg(Arg::with_name(options::KERNELVERSION)
        .short("v")
        .long(options::KERNELVERSION)
        .help("print the operating system version."))
    .arg(Arg::with_name(options::HWPLATFORM)
        .short("i")
        .long(options::HWPLATFORM)
        .help("print the hardware platform (non-portable)"))
    .arg(Arg::with_name(options::MACHINE)
        .short("m")
        .long(options::MACHINE)
        .help("print the machine hardware name."))
    .arg(Arg::with_name(options::PROCESSOR)
        .short("p")
        .long(options::PROCESSOR)
        .help("print the processor type (non-portable)"))
    .arg(Arg::with_name(options::OS)
        .short("o")
        .long(options::OS)
        .help("print the operating system name."))
}
