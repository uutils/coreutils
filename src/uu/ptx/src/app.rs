// spell-checker:ignore (ToDOs) Roff

use clap::{crate_version, App, Arg};

const BRIEF: &str = "Usage: ptx [OPTION]... [INPUT]...   (without -G) or: \
ptx -G [OPTION]... [INPUT [OUTPUT]] \n Output a permuted index, \
including context, of the words in the input files. \n\n Mandatory \
arguments to long options are mandatory for short options too.\n
With no FILE, or when FILE is -, read standard input. \
Default is '-F /'.";

pub mod options {
    pub const FILE: &str = "file";
    pub const AUTO_REFERENCE: &str = "auto-reference";
    pub const TRADITIONAL: &str = "traditional";
    pub const FLAG_TRUNCATION: &str = "flag-truncation";
    pub const MACRO_NAME: &str = "macro-name";
    pub const FORMAT_ROFF: &str = "format=roff";
    pub const RIGHT_SIDE_REFS: &str = "right-side-refs";
    pub const SENTENCE_REGEXP: &str = "sentence-regexp";
    pub const FORMAT_TEX: &str = "format=tex";
    pub const WORD_REGEXP: &str = "word-regexp";
    pub const BREAK_FILE: &str = "break-file";
    pub const IGNORE_CASE: &str = "ignore-case";
    pub const GAP_SIZE: &str = "gap-size";
    pub const IGNORE_FILE: &str = "ignore-file";
    pub const ONLY_FILE: &str = "only-file";
    pub const REFERENCES: &str = "references";
    pub const WIDTH: &str = "width";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .usage(BRIEF)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
        .arg(
            Arg::with_name(options::AUTO_REFERENCE)
                .short("A")
                .long(options::AUTO_REFERENCE)
                .help("output automatically generated references")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::TRADITIONAL)
                .short("G")
                .long(options::TRADITIONAL)
                .help("behave more like System V 'ptx'"),
        )
        .arg(
            Arg::with_name(options::FLAG_TRUNCATION)
                .short("F")
                .long(options::FLAG_TRUNCATION)
                .help("use STRING for flagging line truncations")
                .value_name("STRING")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::MACRO_NAME)
                .short("M")
                .long(options::MACRO_NAME)
                .help("macro name to use instead of 'xx'")
                .value_name("STRING")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::FORMAT_ROFF)
                .short("O")
                .long(options::FORMAT_ROFF)
                .help("generate output as roff directives"),
        )
        .arg(
            Arg::with_name(options::RIGHT_SIDE_REFS)
                .short("R")
                .long(options::RIGHT_SIDE_REFS)
                .help("put references at right, not counted in -w")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::SENTENCE_REGEXP)
                .short("S")
                .long(options::SENTENCE_REGEXP)
                .help("for end of lines or end of sentences")
                .value_name("REGEXP")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::FORMAT_TEX)
                .short("T")
                .long(options::FORMAT_TEX)
                .help("generate output as TeX directives"),
        )
        .arg(
            Arg::with_name(options::WORD_REGEXP)
                .short("W")
                .long(options::WORD_REGEXP)
                .help("use REGEXP to match each keyword")
                .value_name("REGEXP")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::BREAK_FILE)
                .short("b")
                .long(options::BREAK_FILE)
                .help("word break characters in this FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::IGNORE_CASE)
                .short("f")
                .long(options::IGNORE_CASE)
                .help("fold lower case to upper case for sorting")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::GAP_SIZE)
                .short("g")
                .long(options::GAP_SIZE)
                .help("gap size in columns between output fields")
                .value_name("NUMBER")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::IGNORE_FILE)
                .short("i")
                .long(options::IGNORE_FILE)
                .help("read ignore word list from FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::ONLY_FILE)
                .short("o")
                .long(options::ONLY_FILE)
                .help("read only word list from this FILE")
                .value_name("FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::REFERENCES)
                .short("r")
                .long(options::REFERENCES)
                .help("first field of each line is a reference")
                .value_name("FILE")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::WIDTH)
                .short("w")
                .long(options::WIDTH)
                .help("output width in columns, reference excluded")
                .value_name("NUMBER")
                .takes_value(true),
        )
}
