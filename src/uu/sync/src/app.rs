use clap::{crate_version, App, Arg};

const ABOUT: &str = "Synchronize cached writes to persistent storage";
pub mod options {
    pub const FILE_SYSTEM: &str = "file-system";
    pub const DATA: &str = "data";
}

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::FILE_SYSTEM)
                .short("f")
                .long(options::FILE_SYSTEM)
                .conflicts_with(options::DATA)
                .help("sync the file systems that contain the files (Linux and Windows only)"),
        )
        .arg(
            Arg::with_name(options::DATA)
                .short("d")
                .long(options::DATA)
                .conflicts_with(options::FILE_SYSTEM)
                .help("sync only file data, no unneeded metadata (Linux only)"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
}
