use clap::{crate_version, App, Arg};
use strum_macros::{AsRefStr, EnumString};

static ABOUT: &str = "Report or omit repeated lines.";
pub mod options {
    pub static ALL_REPEATED: &str = "all-repeated";
    pub static CHECK_CHARS: &str = "check-chars";
    pub static COUNT: &str = "count";
    pub static IGNORE_CASE: &str = "ignore-case";
    pub static REPEATED: &str = "repeated";
    pub static SKIP_FIELDS: &str = "skip-fields";
    pub static SKIP_CHARS: &str = "skip-chars";
    pub static UNIQUE: &str = "unique";
    pub static ZERO_TERMINATED: &str = "zero-terminated";
    pub static GROUP: &str = "group";
}

pub const ARG_FILES: &str = "files";

#[derive(PartialEq, Clone, Copy, AsRefStr, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Delimiters {
    Append,
    Prepend,
    Separate,
    Both,
    None,
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
    .version(crate_version!())
    .about(ABOUT)
    .arg(
        Arg::with_name(options::ALL_REPEATED)
            .short("D")
            .long(options::ALL_REPEATED)
            .possible_values(&[
                Delimiters::None.as_ref(), Delimiters::Prepend.as_ref(), Delimiters::Separate.as_ref()
            ])
            .help("print all duplicate lines. Delimiting is done with blank lines. [default: none]")
            .value_name("delimit-method")
            .min_values(0)
            .max_values(1),
    )
    .arg(
        Arg::with_name(options::GROUP)
            .long(options::GROUP)
            .possible_values(&[
                Delimiters::Separate.as_ref(), Delimiters::Prepend.as_ref(),
                Delimiters::Append.as_ref(), Delimiters::Both.as_ref()
            ])
            .help("show all items, separating groups with an empty line. [default: separate]")
            .value_name("group-method")
            .min_values(0)
            .max_values(1)
            .conflicts_with_all(&[
                options::REPEATED,
                options::ALL_REPEATED,
                options::UNIQUE,
            ]),
    )
    .arg(
        Arg::with_name(options::CHECK_CHARS)
            .short("w")
            .long(options::CHECK_CHARS)
            .help("compare no more than N characters in lines")
            .value_name("N"),
    )
    .arg(
        Arg::with_name(options::COUNT)
            .short("c")
            .long(options::COUNT)
            .help("prefix lines by the number of occurrences"),
    )
    .arg(
        Arg::with_name(options::IGNORE_CASE)
            .short("i")
            .long(options::IGNORE_CASE)
            .help("ignore differences in case when comparing"),
    )
    .arg(
        Arg::with_name(options::REPEATED)
            .short("d")
            .long(options::REPEATED)
            .help("only print duplicate lines"),
    )
    .arg(
        Arg::with_name(options::SKIP_CHARS)
            .short("s")
            .long(options::SKIP_CHARS)
            .help("avoid comparing the first N characters")
            .value_name("N"),
    )
    .arg(
        Arg::with_name(options::SKIP_FIELDS)
            .short("f")
            .long(options::SKIP_FIELDS)
            .help("avoid comparing the first N fields")
            .value_name("N"),
    )
    .arg(
        Arg::with_name(options::UNIQUE)
            .short("u")
            .long(options::UNIQUE)
            .help("only print unique lines"),
    )
    .arg(
        Arg::with_name(options::ZERO_TERMINATED)
            .short("z")
            .long(options::ZERO_TERMINATED)
            .help("end lines with 0 byte, not newline"),
    )
    .arg(
        Arg::with_name(ARG_FILES)
            .multiple(true)
            .takes_value(true)
            .max_values(2),
    )
}
