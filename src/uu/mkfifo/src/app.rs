use clap::{crate_version, App, Arg};

const SUMMARY: &str = "Create a FIFO with the given name.";

pub mod options {
    pub const MODE: &str = "mode";
    pub const SE_LINUX_SECURITY_CONTEXT: &str = "Z";
    pub const CONTEXT: &str = "context";
    pub const FIFO: &str = "fifo";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::MODE)
                .short("m")
                .long(options::MODE)
                .help("file permissions for the fifo")
                .default_value("0666")
                .value_name("0666"),
        )
        .arg(
            Arg::with_name(options::SE_LINUX_SECURITY_CONTEXT)
                .short(options::SE_LINUX_SECURITY_CONTEXT)
                .help("set the SELinux security context to default type"),
        )
        .arg(
            Arg::with_name(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .help(
                    "like -Z, or if CTX is specified then set the SELinux \
    or SMACK security context to CTX",
                ),
        )
        .arg(Arg::with_name(options::FIFO).hidden(true).multiple(true))
}
