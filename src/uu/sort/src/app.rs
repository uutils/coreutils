// spell-checker:ignore (misc) Mbdfhn

use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display sorted concatenation of all FILE(s).";

const LONG_HELP_KEYS: &str = "The key format is FIELD[.CHAR][OPTIONS][,FIELD[.CHAR]][OPTIONS].

Fields by default are separated by the first whitespace after a non-whitespace character. Use -t to specify a custom separator.
In the default case, whitespace is appended at the beginning of each field. Custom separators however are not included in fields.

FIELD and CHAR both start at 1 (i.e. they are 1-indexed). If there is no end specified after a comma, the end will be the end of the line.
If CHAR is set 0, it means the end of the field. CHAR defaults to 1 for the start position and to 0 for the end position.

Valid options are: MbdfhnRrV. They override the global options for this key.";

pub mod options {
    pub mod modes {
        pub const SORT: &str = "sort";

        pub const HUMAN_NUMERIC: &str = "human-numeric-sort";
        pub const MONTH: &str = "month-sort";
        pub const NUMERIC: &str = "numeric-sort";
        pub const GENERAL_NUMERIC: &str = "general-numeric-sort";
        pub const VERSION: &str = "version-sort";
        pub const RANDOM: &str = "random-sort";

        pub const ALL_SORT_MODES: [&str; 6] = [
            GENERAL_NUMERIC,
            HUMAN_NUMERIC,
            MONTH,
            NUMERIC,
            VERSION,
            RANDOM,
        ];
    }

    pub mod check {
        pub const CHECK: &str = "check";
        pub const CHECK_SILENT: &str = "check-silent";
        pub const SILENT: &str = "silent";
        pub const QUIET: &str = "quiet";
        pub const DIAGNOSE_FIRST: &str = "diagnose-first";
    }

    pub const DICTIONARY_ORDER: &str = "dictionary-order";
    pub const MERGE: &str = "merge";
    pub const DEBUG: &str = "debug";
    pub const IGNORE_CASE: &str = "ignore-case";
    pub const IGNORE_LEADING_BLANKS: &str = "ignore-leading-blanks";
    pub const IGNORE_NONPRINTING: &str = "ignore-nonprinting";
    pub const OUTPUT: &str = "output";
    pub const REVERSE: &str = "reverse";
    pub const STABLE: &str = "stable";
    pub const UNIQUE: &str = "unique";
    pub const KEY: &str = "key";
    pub const SEPARATOR: &str = "field-separator";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const PARALLEL: &str = "parallel";
    pub const FILES0_FROM: &str = "files0-from";
    pub const BUF_SIZE: &str = "buffer-size";
    pub const TMP_DIR: &str = "temporary-directory";
    pub const COMPRESS_PROG: &str = "compress-program";
    pub const BATCH_SIZE: &str = "batch-size";

    pub const FILES: &str = "files";
}

/// Creates an `Arg` that conflicts with all other sort modes.
fn make_sort_mode_arg<'a, 'b>(mode: &'a str, short: &'b str, help: &'b str) -> Arg<'a, 'b> {
    let mut arg = Arg::with_name(mode).short(short).long(mode).help(help);
    for possible_mode in &options::modes::ALL_SORT_MODES {
        if *possible_mode != mode {
            arg = arg.conflicts_with(possible_mode);
        }
    }
    arg
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
    .version(crate_version!())
    .about(ABOUT)
    .arg(
        Arg::with_name(options::modes::SORT)
            .long(options::modes::SORT)
            .takes_value(true)
            .possible_values(
                &[
                    "general-numeric",
                    "human-numeric",
                    "month",
                    "numeric",
                    "version",
                    "random",
                ]
            )
            .conflicts_with_all(&options::modes::ALL_SORT_MODES)
    )
    .arg(
        make_sort_mode_arg(
            options::modes::HUMAN_NUMERIC,
            "h",
            "compare according to human readable sizes, eg 1M > 100k"
        ),
    )
    .arg(
        make_sort_mode_arg(
            options::modes::MONTH,
            "M",
            "compare according to month name abbreviation"
        ),
    )
    .arg(
        make_sort_mode_arg(
            options::modes::NUMERIC,
            "n",
            "compare according to string numerical value"
        ),
    )
    .arg(
        make_sort_mode_arg(
            options::modes::GENERAL_NUMERIC,
            "g",
            "compare according to string general numerical value"
        ),
    )
    .arg(
        make_sort_mode_arg(
            options::modes::VERSION,
            "V",
            "Sort by SemVer version number, eg 1.12.2 > 1.1.2",
        ),
    )
    .arg(
        make_sort_mode_arg(
            options::modes::RANDOM,
            "R",
            "shuffle in random order",
        ),
    )
    .arg(
        Arg::with_name(options::DICTIONARY_ORDER)
            .short("d")
            .long(options::DICTIONARY_ORDER)
            .help("consider only blanks and alphanumeric characters")
            .conflicts_with_all(
                &[
                    options::modes::NUMERIC,
                    options::modes::GENERAL_NUMERIC,
                    options::modes::HUMAN_NUMERIC,
                    options::modes::MONTH,
                ]
            ),
    )
    .arg(
        Arg::with_name(options::MERGE)
            .short("m")
            .long(options::MERGE)
            .help("merge already sorted files; do not sort"),
    )
    .arg(
        Arg::with_name(options::check::CHECK)
            .short("c")
            .long(options::check::CHECK)
            .takes_value(true)
            .require_equals(true)
            .min_values(0)
            .possible_values(&[
                options::check::SILENT,
                options::check::QUIET,
                options::check::DIAGNOSE_FIRST,
            ])
            .help("check for sorted input; do not sort"),
    )
    .arg(
        Arg::with_name(options::check::CHECK_SILENT)
            .short("C")
            .long(options::check::CHECK_SILENT)
            .help("exit successfully if the given file is already sorted, and exit with status 1 otherwise."),
    )
    .arg(
        Arg::with_name(options::IGNORE_CASE)
            .short("f")
            .long(options::IGNORE_CASE)
            .help("fold lower case to upper case characters"),
    )
    .arg(
        Arg::with_name(options::IGNORE_NONPRINTING)
            .short("i")
            .long(options::IGNORE_NONPRINTING)
            .help("ignore nonprinting characters")
            .conflicts_with_all(
                &[
                    options::modes::NUMERIC,
                    options::modes::GENERAL_NUMERIC,
                    options::modes::HUMAN_NUMERIC,
                    options::modes::MONTH
                ]
            ),
    )
    .arg(
        Arg::with_name(options::IGNORE_LEADING_BLANKS)
            .short("b")
            .long(options::IGNORE_LEADING_BLANKS)
            .help("ignore leading blanks when finding sort keys in each line"),
    )
    .arg(
        Arg::with_name(options::OUTPUT)
            .short("o")
            .long(options::OUTPUT)
            .help("write output to FILENAME instead of stdout")
            .takes_value(true)
            .value_name("FILENAME"),
    )
    .arg(
        Arg::with_name(options::REVERSE)
            .short("r")
            .long(options::REVERSE)
            .help("reverse the output"),
    )
    .arg(
        Arg::with_name(options::STABLE)
            .short("s")
            .long(options::STABLE)
            .help("stabilize sort by disabling last-resort comparison"),
    )
    .arg(
        Arg::with_name(options::UNIQUE)
            .short("u")
            .long(options::UNIQUE)
            .help("output only the first of an equal run"),
    )
    .arg(
        Arg::with_name(options::KEY)
            .short("k")
            .long(options::KEY)
            .help("sort by a key")
            .long_help(LONG_HELP_KEYS)
            .multiple(true)
            .takes_value(true),
    )
    .arg(
        Arg::with_name(options::SEPARATOR)
            .short("t")
            .long(options::SEPARATOR)
            .help("custom separator for -k")
            .takes_value(true))
    .arg(
        Arg::with_name(options::ZERO_TERMINATED)
            .short("z")
            .long(options::ZERO_TERMINATED)
            .help("line delimiter is NUL, not newline"),
    )
    .arg(
        Arg::with_name(options::PARALLEL)
            .long(options::PARALLEL)
            .help("change the number of threads running concurrently to NUM_THREADS")
            .takes_value(true)
            .value_name("NUM_THREADS"),
    )
    .arg(
        Arg::with_name(options::BUF_SIZE)
            .short("S")
            .long(options::BUF_SIZE)
            .help("sets the maximum SIZE of each segment in number of sorted items")
            .takes_value(true)
            .value_name("SIZE"),
    )
    .arg(
        Arg::with_name(options::TMP_DIR)
            .short("T")
            .long(options::TMP_DIR)
            .help("use DIR for temporaries, not $TMPDIR or /tmp")
            .takes_value(true)
            .value_name("DIR"),
    )
    .arg(
        Arg::with_name(options::COMPRESS_PROG)
            .long(options::COMPRESS_PROG)
            .help("compress temporary files with PROG, decompress with PROG -d")
            .long_help("PROG has to take input from stdin and output to stdout")
            .value_name("PROG")
    )
    .arg(
        Arg::with_name(options::BATCH_SIZE)
            .long(options::BATCH_SIZE)
            .help("Merge at most N_MERGE inputs at once.")
            .value_name("N_MERGE")
    )
    .arg(
        Arg::with_name(options::FILES0_FROM)
            .long(options::FILES0_FROM)
            .help("read input from the files specified by NUL-terminated NUL_FILES")
            .takes_value(true)
            .value_name("NUL_FILES")
            .multiple(true),
    )
    .arg(
        Arg::with_name(options::DEBUG)
            .long(options::DEBUG)
            .help("underline the parts of the line that are actually used for sorting"),
    )
    .arg(Arg::with_name(options::FILES).multiple(true).takes_value(true))
}
