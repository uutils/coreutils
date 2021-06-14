use clap::{crate_version, App, Arg};

const USAGE: &str = "[OPTION]... [FILE]...\nWith no FILE, or when  FILE is -, read standard input.";
const SUMMARY: &str = "Checksum and count the blocks in a file.";

pub mod options {
    pub const FILE: &str = "file";
    pub const BSD_COMPATIBLE: &str = "r";
    pub const SYSTEM_V_COMPATIBLE: &str = "sysv";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .usage(USAGE)
        .about(SUMMARY)
        .arg(Arg::with_name(options::FILE).multiple(true).hidden(true))
        .arg(
            Arg::with_name(options::BSD_COMPATIBLE)
                .short(options::BSD_COMPATIBLE)
                .help("use the BSD sum algorithm, use 1K blocks (default)"),
        )
        .arg(
            Arg::with_name(options::SYSTEM_V_COMPATIBLE)
                .short("s")
                .long(options::SYSTEM_V_COMPATIBLE)
                .help("use System V sum algorithm, use 512 bytes blocks"),
        )
}
