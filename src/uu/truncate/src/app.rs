use clap::{crate_version, App, Arg};

const ABOUT: &str = "Shrink or extend the size of each file to the specified size.";

pub mod options {
    pub const IO_BLOCKS: &str = "io-blocks";
    pub const NO_CREATE: &str = "no-create";
    pub const REFERENCE: &str = "reference";
    pub const SIZE: &str = "size";
    pub const ARG_FILES: &str = "files";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
    .version(crate_version!())
    .about(ABOUT)
    .arg(
        Arg::with_name(options::IO_BLOCKS)
        .short("o")
        .long(options::IO_BLOCKS)
        .help("treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)")
    )
    .arg(
        Arg::with_name(options::NO_CREATE)
        .short("c")
        .long(options::NO_CREATE)
        .help("do not create files that do not exist")
    )
    .arg(
        Arg::with_name(options::REFERENCE)
        .short("r")
        .long(options::REFERENCE)
        .required_unless(options::SIZE)
        .help("base the size of each file on the size of RFILE")
        .value_name("RFILE")
    )
    .arg(
        Arg::with_name(options::SIZE)
        .short("s")
        .long(options::SIZE)
        .required_unless(options::REFERENCE)
        .help("set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified")
        .value_name("SIZE")
    )
    .arg(Arg::with_name(options::ARG_FILES)
         .value_name("FILE")
         .multiple(true)
         .takes_value(true)
         .required(true)
         .min_values(1))
}
