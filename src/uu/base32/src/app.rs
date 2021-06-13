use clap::{App, Arg};

pub mod options {
    pub static DECODE: &str = "decode";
    pub static WRAP: &str = "wrap";
    pub static IGNORE_GARBAGE: &str = "ignore-garbage";
    pub static FILE: &str = "file";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .arg(
            Arg::with_name(options::DECODE)
                .short("d")
                .long(options::DECODE)
                .help("decode data"),
        )
        .arg(
            Arg::with_name(options::IGNORE_GARBAGE)
                .short("i")
                .long(options::IGNORE_GARBAGE)
                .help("when decoding, ignore non-alphabetic characters"),
        )
        .arg(
            Arg::with_name(options::WRAP)
                .short("w")
                .long(options::WRAP)
                .takes_value(true)
                .help(
                    "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)",
                ),
        )
        // "multiple" arguments are used to check whether there is more than one
        // file passed in.
        .arg(Arg::with_name(options::FILE).index(1).multiple(true))
}
