// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_version, Arg, ArgAction, Command};
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("sort.md");
const USAGE: &str = help_usage!("sort.md");
const AFTER_HELP: &str = help_section!("after help", "sort.md");

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

    pub const HELP: &str = "help";
    pub const VERSION: &str = "version";
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
fn make_sort_mode_arg(mode: &'static str, short: char, help: &'static str) -> Arg {
    let mut arg = Arg::new(mode)
        .short(short)
        .long(mode)
        .help(help)
        .action(ArgAction::SetTrue);
    for possible_mode in &options::modes::ALL_SORT_MODES {
        if *possible_mode != mode {
            arg = arg.conflicts_with(possible_mode);
        }
    }
    arg
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::VERSION)
                .long(options::VERSION)
                .help("Print version information.")
                .action(ArgAction::Version),
        )
        .arg(
            Arg::new(options::modes::SORT)
                .long(options::modes::SORT)
                .value_parser(ShortcutValueParser::new([
                    "general-numeric",
                    "human-numeric",
                    "month",
                    "numeric",
                    "version",
                    "random",
                ]))
                .conflicts_with_all(options::modes::ALL_SORT_MODES),
        )
        .arg(make_sort_mode_arg(
            options::modes::HUMAN_NUMERIC,
            'h',
            "compare according to human readable sizes, eg 1M > 100k",
        ))
        .arg(make_sort_mode_arg(
            options::modes::MONTH,
            'M',
            "compare according to month name abbreviation",
        ))
        .arg(make_sort_mode_arg(
            options::modes::NUMERIC,
            'n',
            "compare according to string numerical value",
        ))
        .arg(make_sort_mode_arg(
            options::modes::GENERAL_NUMERIC,
            'g',
            "compare according to string general numerical value",
        ))
        .arg(make_sort_mode_arg(
            options::modes::VERSION,
            'V',
            "Sort by SemVer version number, eg 1.12.2 > 1.1.2",
        ))
        .arg(make_sort_mode_arg(
            options::modes::RANDOM,
            'R',
            "shuffle in random order",
        ))
        .arg(
            Arg::new(options::DICTIONARY_ORDER)
                .short('d')
                .long(options::DICTIONARY_ORDER)
                .help("consider only blanks and alphanumeric characters")
                .conflicts_with_all([
                    options::modes::NUMERIC,
                    options::modes::GENERAL_NUMERIC,
                    options::modes::HUMAN_NUMERIC,
                    options::modes::MONTH,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MERGE)
                .short('m')
                .long(options::MERGE)
                .help("merge already sorted files; do not sort")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::check::CHECK)
                .short('c')
                .long(options::check::CHECK)
                .require_equals(true)
                .num_args(0..)
                .value_parser(ShortcutValueParser::new([
                    options::check::SILENT,
                    options::check::QUIET,
                    options::check::DIAGNOSE_FIRST,
                ]))
                .conflicts_with(options::OUTPUT)
                .help("check for sorted input; do not sort"),
        )
        .arg(
            Arg::new(options::check::CHECK_SILENT)
                .short('C')
                .long(options::check::CHECK_SILENT)
                .conflicts_with(options::OUTPUT)
                .help(
                    "exit successfully if the given file is already sorted, \
                and exit with status 1 otherwise.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .short('f')
                .long(options::IGNORE_CASE)
                .help("fold lower case to upper case characters")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_NONPRINTING)
                .short('i')
                .long(options::IGNORE_NONPRINTING)
                .help("ignore nonprinting characters")
                .conflicts_with_all([
                    options::modes::NUMERIC,
                    options::modes::GENERAL_NUMERIC,
                    options::modes::HUMAN_NUMERIC,
                    options::modes::MONTH,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_LEADING_BLANKS)
                .short('b')
                .long(options::IGNORE_LEADING_BLANKS)
                .help("ignore leading blanks when finding sort keys in each line")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT)
                .short('o')
                .long(options::OUTPUT)
                .help("write output to FILENAME instead of stdout")
                .value_name("FILENAME")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::REVERSE)
                .short('r')
                .long(options::REVERSE)
                .help("reverse the output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STABLE)
                .short('s')
                .long(options::STABLE)
                .help("stabilize sort by disabling last-resort comparison")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::UNIQUE)
                .short('u')
                .long(options::UNIQUE)
                .help("output only the first of an equal run")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KEY)
                .short('k')
                .long(options::KEY)
                .help("sort by a key")
                .action(ArgAction::Append)
                .num_args(1),
        )
        .arg(
            Arg::new(options::SEPARATOR)
                .short('t')
                .long(options::SEPARATOR)
                .help("custom separator for -k")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PARALLEL)
                .long(options::PARALLEL)
                .help("change the number of threads running concurrently to NUM_THREADS")
                .value_name("NUM_THREADS"),
        )
        .arg(
            Arg::new(options::BUF_SIZE)
                .short('S')
                .long(options::BUF_SIZE)
                .help("sets the maximum SIZE of each segment in number of sorted items")
                .value_name("SIZE"),
        )
        .arg(
            Arg::new(options::TMP_DIR)
                .short('T')
                .long(options::TMP_DIR)
                .help("use DIR for temporaries, not $TMPDIR or /tmp")
                .value_name("DIR")
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            Arg::new(options::COMPRESS_PROG)
                .long(options::COMPRESS_PROG)
                .help("compress temporary files with PROG, decompress with PROG -d; PROG has to take input from stdin and output to stdout")
                .value_name("PROG")
                .value_hint(clap::ValueHint::CommandName),
        )
        .arg(
            Arg::new(options::BATCH_SIZE)
                .long(options::BATCH_SIZE)
                .help("Merge at most N_MERGE inputs at once.")
                .value_name("N_MERGE"),
        )
        .arg(
            Arg::new(options::FILES0_FROM)
                .long(options::FILES0_FROM)
                .help("read input from the files specified by NUL-terminated NUL_FILES")
                .value_name("NUL_FILES")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::DEBUG)
                .long(options::DEBUG)
                .help("underline the parts of the line that are actually used for sorting")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
}
