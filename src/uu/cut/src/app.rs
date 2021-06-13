// spell-checker:ignore (ToDO) sourcefiles

use clap::{crate_version, App, Arg};

const NAME: &str = "cut";
const SYNTAX: &str =
    "[-d] [-s] [-z] [--output-delimiter] ((-f|-b|-c) {{sequence}}) {{sourcefile}}+";
const SUMMARY: &str =
    "Prints specified byte or field columns from each line of stdin or the input files";
const LONG_HELP: &str = "
 Each call must specify a mode (what to use for columns),
 a sequence (which columns to print), and provide a data source

 Specifying a mode

    Use --bytes (-b) or --characters (-c) to specify byte mode

    Use --fields (-f) to specify field mode, where each line is broken into
    fields identified by a delimiter character. For example for a typical CSV
    you could use this in combination with setting comma as the delimiter

 Specifying a sequence

    A sequence is a group of 1 or more numbers or inclusive ranges separated
    by a commas.

    cut -f 2,5-7 some_file.txt
    will display the 2nd, 5th, 6th, and 7th field for each source line

    Ranges can extend to the end of the row by excluding the the second number

    cut -f 3- some_file.txt
    will display the 3rd field and all fields after for each source line

    The first number of a range can be excluded, and this is effectively the
    same as using 1 as the first number: it causes the range to begin at the
    first column. Ranges can also display a single column

    cut -f 1,3-5 some_file.txt
    will display the 1st, 3rd, 4th, and 5th field for each source line

    The --complement option, when used, inverts the effect of the sequence

    cut --complement -f 4-6 some_file.txt
    will display the every field but the 4th, 5th, and 6th

 Specifying a data source

    If no sourcefile arguments are specified, stdin is used as the source of
    lines to print

    If sourcefile arguments are specified, stdin is ignored and all files are
    read in consecutively if a sourcefile is not successfully read, a warning
    will print to stderr, and the eventual status code will be 1, but cut
    will continue to read through proceeding sourcefiles

    To print columns from both STDIN and a file argument, use - (dash) as a
    sourcefile argument to represent stdin.

 Field Mode options

    The fields in each line are identified by a delimiter (separator)

    Set the delimiter
        Set the delimiter which separates fields in the file using the
        --delimiter (-d) option. Setting the delimiter is optional.
        If not set, a default delimiter of Tab will be used.

    Optionally Filter based on delimiter
        If the --only-delimited (-s) flag is provided, only lines which
        contain the delimiter will be printed

    Replace the delimiter
        If the --output-delimiter option is provided, the argument used for
        it will replace the delimiter character in each line printed. This is
        useful for transforming tabular data - e.g. to convert a CSV to a
        TSV (tab-separated file)

 Line endings

    When the --zero-terminated (-z) option is used, cut sees \\0 (null) as the
    'line ending' character (both for the purposes of reading lines and
    separating printed lines) instead of \\n (newline). This is useful for
    tabular data where some of the cells may contain newlines

    echo 'ab\\0cd' | cut -z -c 1
    will result in 'a\\0c\\0'
";

pub mod options {
    pub const BYTES: &str = "bytes";
    pub const CHARACTERS: &str = "characters";
    pub const DELIMITER: &str = "delimiter";
    pub const FIELDS: &str = "fields";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const ONLY_DELIMITED: &str = "only-delimited";
    pub const OUTPUT_DELIMITER: &str = "output-delimiter";
    pub const COMPLEMENT: &str = "complement";
    pub const FILE: &str = "file";
}
pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .name(NAME)
        .version(crate_version!())
        .usage(SYNTAX)
        .about(SUMMARY)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::BYTES)
                .short("b")
                .long(options::BYTES)
                .takes_value(true)
                .help("filter byte columns from the input source")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .display_order(1),
        )
        .arg(
            Arg::with_name(options::CHARACTERS)
                .short("c")
                .long(options::CHARACTERS)
                .help("alias for character mode")
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("LIST")
                .display_order(2),
        )
        .arg(
            Arg::with_name(options::DELIMITER)
                .short("d")
                .long(options::DELIMITER)
                .help("specify the delimiter character that separates fields in the input source. Defaults to Tab.")
                .takes_value(true)
                .value_name("DELIM")
                .display_order(3),
        )
        .arg(
            Arg::with_name(options::FIELDS)
                .short("f")
                .long(options::FIELDS)
                .help("filter field columns from the input source")
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("LIST")
                .display_order(4),
        )
        .arg(
            Arg::with_name(options::COMPLEMENT)
                .long(options::COMPLEMENT)
                .help("invert the filter - instead of displaying only the filtered columns, display all but those columns")
                .takes_value(false)
                .display_order(5),
        )
        .arg(
            Arg::with_name(options::ONLY_DELIMITED)
            .short("s")
                .long(options::ONLY_DELIMITED)
                .help("in field mode, only print lines which contain the delimiter")
                .takes_value(false)
                .display_order(6),
        )
        .arg(
            Arg::with_name(options::ZERO_TERMINATED)
            .short("z")
                .long(options::ZERO_TERMINATED)
                .help("instead of filtering columns based on line, filter columns based on \\0 (NULL character)")
                .takes_value(false)
                .display_order(8),
        )
        .arg(
            Arg::with_name(options::OUTPUT_DELIMITER)
            .long(options::OUTPUT_DELIMITER)
                .help("in field mode, replace the delimiter in output lines with this option's argument")
                .takes_value(true)
                .value_name("NEW_DELIM")
                .display_order(7),
        )
        .arg(
            Arg::with_name(options::FILE)
            .hidden(true)
                .multiple(true)
        )
}
