use clap::{crate_version, App, Arg};

pub mod options {
    pub const ECHO: &str = "echo";
    pub const INPUT_RANGE: &str = "input-range";
    pub const HEAD_COUNT: &str = "head-count";
    pub const OUTPUT: &str = "output";
    pub const RANDOM_SOURCE: &str = "random-source";
    pub const REPEAT: &str = "repeat";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const FILE: &str = "file";
}

const USAGE: &str = r#"shuf [OPTION]... [FILE]
  or:  shuf -e [OPTION]... [ARG]...
  or:  shuf -i LO-HI [OPTION]...
Write a random permutation of the input lines to standard output.

With no FILE, or when FILE is -, read standard input.
"#;
const TEMPLATE: &str = "Usage: {usage}\nMandatory arguments to long options are mandatory for short options too.\n{unified}";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .template(TEMPLATE)
        .usage(USAGE)
        .arg(
            Arg::with_name(options::ECHO)
                .short("e")
                .long(options::ECHO)
                .takes_value(true)
                .value_name("ARG")
                .help("treat each ARG as an input line")
                .multiple(true)
                .use_delimiter(false)
                .min_values(0)
                .conflicts_with(options::INPUT_RANGE),
        )
        .arg(
            Arg::with_name(options::INPUT_RANGE)
                .short("i")
                .long(options::INPUT_RANGE)
                .takes_value(true)
                .value_name("LO-HI")
                .help("treat each number LO through HI as an input line")
                .conflicts_with(options::FILE),
        )
        .arg(
            Arg::with_name(options::HEAD_COUNT)
                .short("n")
                .long(options::HEAD_COUNT)
                .takes_value(true)
                .value_name("COUNT")
                .help("output at most COUNT lines"),
        )
        .arg(
            Arg::with_name(options::OUTPUT)
                .short("o")
                .long(options::OUTPUT)
                .takes_value(true)
                .value_name("FILE")
                .help("write result to FILE instead of standard output"),
        )
        .arg(
            Arg::with_name(options::RANDOM_SOURCE)
                .long(options::RANDOM_SOURCE)
                .takes_value(true)
                .value_name("FILE")
                .help("get random bytes from FILE"),
        )
        .arg(
            Arg::with_name(options::REPEAT)
                .short("r")
                .long(options::REPEAT)
                .help("output lines can be repeated"),
        )
        .arg(
            Arg::with_name(options::ZERO_TERMINATED)
                .short("z")
                .long(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline"),
        )
        .arg(Arg::with_name(options::FILE).takes_value(true))
}
