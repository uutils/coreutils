// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::PossibleValue, crate_version, Arg, ArgAction, Command};
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("du.md");
const AFTER_HELP: &str = help_section!("after help", "du.md");
const USAGE: &str = help_usage!("du.md");

pub mod options {
    pub const HELP: &str = "help";
    pub const NULL: &str = "0";
    pub const ALL: &str = "all";
    pub const APPARENT_SIZE: &str = "apparent-size";
    pub const BLOCK_SIZE: &str = "block-size";
    pub const BYTES: &str = "b";
    pub const TOTAL: &str = "c";
    pub const MAX_DEPTH: &str = "d";
    pub const HUMAN_READABLE: &str = "h";
    pub const BLOCK_SIZE_1K: &str = "k";
    pub const COUNT_LINKS: &str = "l";
    pub const BLOCK_SIZE_1M: &str = "m";
    pub const SEPARATE_DIRS: &str = "S";
    pub const SUMMARIZE: &str = "s";
    pub const THRESHOLD: &str = "threshold";
    pub const SI: &str = "si";
    pub const TIME: &str = "time";
    pub const TIME_STYLE: &str = "time-style";
    pub const ONE_FILE_SYSTEM: &str = "one-file-system";
    pub const DEREFERENCE: &str = "dereference";
    pub const DEREFERENCE_ARGS: &str = "dereference-args";
    pub const NO_DEREFERENCE: &str = "no-dereference";
    pub const INODES: &str = "inodes";
    pub const EXCLUDE: &str = "exclude";
    pub const EXCLUDE_FROM: &str = "exclude-from";
    pub const FILES0_FROM: &str = "files0-from";
    pub const VERBOSE: &str = "verbose";
    pub const FILE: &str = "FILE";
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
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help)
        )
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("write counts for all files, not just directories")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::APPARENT_SIZE)
                .long(options::APPARENT_SIZE)
                .help(
                    "print apparent sizes, rather than disk usage \
                    although the apparent size is usually smaller, it may be larger due to holes \
                    in ('sparse') files, internal fragmentation, indirect blocks, and the like"
                )
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::BLOCK_SIZE)
                .short('B')
                .long(options::BLOCK_SIZE)
                .value_name("SIZE")
                .help(
                    "scale sizes by SIZE before printing them. \
                    E.g., '-BM' prints sizes in units of 1,048,576 bytes. See SIZE format below."
                )
        )
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long("bytes")
                .help("equivalent to '--apparent-size --block-size=1'")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::TOTAL)
                .long("total")
                .short('c')
                .help("produce a grand total")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::MAX_DEPTH)
                .short('d')
                .long("max-depth")
                .value_name("N")
                .help(
                    "print the total for a directory (or file, with --all) \
                    only if it is N or fewer levels below the command \
                    line argument;  --max-depth=0 is the same as --summarize"
                )
        )
        .arg(
            Arg::new(options::HUMAN_READABLE)
                .long("human-readable")
                .short('h')
                .help("print sizes in human readable format (e.g., 1K 234M 2G)")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::INODES)
                .long(options::INODES)
                .help(
                    "list inode usage information instead of block usage like --block-size=1K"
                )
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::BLOCK_SIZE_1K)
                .short('k')
                .help("like --block-size=1K")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::COUNT_LINKS)
                .short('l')
                .long("count-links")
                .help("count sizes many times if hard linked")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help("follow all symbolic links")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::DEREFERENCE_ARGS)
                .short('D')
                .visible_short_alias('H')
                .long(options::DEREFERENCE_ARGS)
                .help("follow only symlinks that are listed on the command line")
                .action(ArgAction::SetTrue)
        )
         .arg(
             Arg::new(options::NO_DEREFERENCE)
                 .short('P')
                 .long(options::NO_DEREFERENCE)
                 .help("don't follow any symbolic links (this is the default)")
                 .overrides_with(options::DEREFERENCE)
                 .action(ArgAction::SetTrue),
         )
        .arg(
            Arg::new(options::BLOCK_SIZE_1M)
                .short('m')
                .help("like --block-size=1M")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::NULL)
                .short('0')
                .long("null")
                .help("end each output line with 0 byte rather than newline")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::SEPARATE_DIRS)
                .short('S')
                .long("separate-dirs")
                .help("do not include size of subdirectories")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::SUMMARIZE)
                .short('s')
                .long("summarize")
                .help("display only a total for each argument")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::SI)
                .long(options::SI)
                .help("like -h, but use powers of 1000 not 1024")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::ONE_FILE_SYSTEM)
                .short('x')
                .long(options::ONE_FILE_SYSTEM)
                .help("skip directories on different file systems")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::THRESHOLD)
                .short('t')
                .long(options::THRESHOLD)
                .value_name("SIZE")
                .num_args(1)
                .allow_hyphen_values(true)
                .help("exclude entries smaller than SIZE if positive, \
                          or entries greater than SIZE if negative")
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long("verbose")
                .help("verbose mode (option not present in GNU/Coreutils)")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::EXCLUDE)
                .long(options::EXCLUDE)
                .value_name("PATTERN")
                .help("exclude files that match PATTERN")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new(options::EXCLUDE_FROM)
                .short('X')
                .long("exclude-from")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .help("exclude files that match any pattern in FILE")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new(options::FILES0_FROM)
                .long("files0-from")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .help("summarize device usage of the NUL-terminated file names specified in file F; if F is -, then read names from standard input")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .value_name("WORD")
                .require_equals(true)
                .num_args(0..)
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("atime").alias("access").alias("use"),
                    PossibleValue::new("ctime").alias("status"),
                    PossibleValue::new("creation").alias("birth"),
                ]))
                .help(
                    "show time of the last modification of any file in the \
                    directory, or any of its subdirectories. If WORD is given, show time as WORD instead \
                    of modification time: atime, access, use, ctime, status, birth or creation"
                )
        )
        .arg(
            Arg::new(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .value_name("STYLE")
                .help(
                    "show times using style STYLE: \
                    full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'"
                )
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .value_hint(clap::ValueHint::AnyPath)
                .action(ArgAction::Append)
        )
}
