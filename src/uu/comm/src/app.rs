use clap::{crate_version, App, Arg};

const ABOUT: &str = "compare two sorted files line by line";

pub mod options {
    pub const COLUMN_1: &str = "1";
    pub const COLUMN_2: &str = "2";
    pub const COLUMN_3: &str = "3";
    pub const DELIMITER: &str = "output-delimiter";
    pub const DELIMITER_DEFAULT: &str = "\t";
    pub const FILE_1: &str = "FILE1";
    pub const FILE_2: &str = "FILE2";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::COLUMN_1)
                .short(options::COLUMN_1)
                .help("suppress column 1 (lines unique to FILE1)"),
        )
        .arg(
            Arg::with_name(options::COLUMN_2)
                .short(options::COLUMN_2)
                .help("suppress column 2 (lines unique to FILE2)"),
        )
        .arg(
            Arg::with_name(options::COLUMN_3)
                .short(options::COLUMN_3)
                .help("suppress column 3 (lines that appear in both files)"),
        )
        .arg(
            Arg::with_name(options::DELIMITER)
                .long(options::DELIMITER)
                .help("separate columns with STR")
                .value_name("STR")
                .default_value(options::DELIMITER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(Arg::with_name(options::FILE_1).required(true))
        .arg(Arg::with_name(options::FILE_2).required(true))
}
