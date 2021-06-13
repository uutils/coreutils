use clap::{crate_version, App, Arg};

// spell-checker:ignore (ToDO) FILENUM pairable unpairable nocheck

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(
            "For each pair of input lines with identical join fields, write a line to
standard output. The default join field is the first, delimited by blanks.

When FILE1 or FILE2 (not both) is -, read standard input.",
        )
        .help_message("display this help and exit")
        .version_message("display version and exit")
        .arg(
            Arg::with_name("a")
                .short("a")
                .takes_value(true)
                .possible_values(&["1", "2"])
                .value_name("FILENUM")
                .help(
                    "also print unpairable lines from file FILENUM, where
FILENUM is 1 or 2, corresponding to FILE1 or FILE2",
                ),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .value_name("FILENUM")
                .help("like -a FILENUM, but suppress joined output lines"),
        )
        .arg(
            Arg::with_name("e")
                .short("e")
                .takes_value(true)
                .value_name("EMPTY")
                .help("replace missing input fields with EMPTY"),
        )
        .arg(
            Arg::with_name("i")
                .short("i")
                .long("ignore-case")
                .help("ignore differences in case when comparing fields"),
        )
        .arg(
            Arg::with_name("j")
                .short("j")
                .takes_value(true)
                .value_name("FIELD")
                .help("equivalent to '-1 FIELD -2 FIELD'"),
        )
        .arg(
            Arg::with_name("o")
                .short("o")
                .takes_value(true)
                .value_name("FORMAT")
                .help("obey FORMAT while constructing output line"),
        )
        .arg(
            Arg::with_name("t")
                .short("t")
                .takes_value(true)
                .value_name("CHAR")
                .help("use CHAR as input and output field separator"),
        )
        .arg(
            Arg::with_name("1")
                .short("1")
                .takes_value(true)
                .value_name("FIELD")
                .help("join on this FIELD of file 1"),
        )
        .arg(
            Arg::with_name("2")
                .short("2")
                .takes_value(true)
                .value_name("FIELD")
                .help("join on this FIELD of file 2"),
        )
        .arg(Arg::with_name("check-order").long("check-order").help(
            "check that the input is correctly sorted, \
             even if all input lines are pairable",
        ))
        .arg(
            Arg::with_name("nocheck-order")
                .long("nocheck-order")
                .help("do not check that the input is correctly sorted"),
        )
        .arg(Arg::with_name("header").long("header").help(
            "treat the first line in each file as field headers, \
             print them without trying to pair them",
        ))
        .arg(
            Arg::with_name("file1")
                .required(true)
                .value_name("FILE1")
                .hidden(true),
        )
        .arg(
            Arg::with_name("file2")
                .required(true)
                .value_name("FILE2")
                .hidden(true),
        )
}
