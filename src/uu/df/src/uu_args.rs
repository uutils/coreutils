// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("df.md");
const USAGE: &str = help_usage!("df.md");
const AFTER_HELP: &str = help_section!("after help", "df.md");

pub mod options {
    pub static OPT_HELP: &str = "help";
    pub static OPT_ALL: &str = "all";
    pub static OPT_BLOCKSIZE: &str = "blocksize";
    pub static OPT_TOTAL: &str = "total";
    pub static OPT_HUMAN_READABLE_BINARY: &str = "human-readable-binary";
    pub static OPT_HUMAN_READABLE_DECIMAL: &str = "human-readable-decimal";
    pub static OPT_INODES: &str = "inodes";
    pub static OPT_KILO: &str = "kilo";
    pub static OPT_LOCAL: &str = "local";
    pub static OPT_NO_SYNC: &str = "no-sync";
    pub static OPT_OUTPUT: &str = "output";
    pub static OPT_PATHS: &str = "paths";
    pub static OPT_PORTABILITY: &str = "portability";
    pub static OPT_SYNC: &str = "sync";
    pub static OPT_TYPE: &str = "type";
    pub static OPT_PRINT_TYPE: &str = "print-type";
    pub static OPT_EXCLUDE_TYPE: &str = "exclude-type";
    pub static OUTPUT_FIELD_LIST: [&str; 12] = [
        "source", "fstype", "itotal", "iused", "iavail", "ipcent", "size", "used", "avail",
        "pcent", "file", "target",
    ];
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::OPT_HELP)
                .long(options::OPT_HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::OPT_ALL)
                .short('a')
                .long("all")
                .overrides_with(options::OPT_ALL)
                .help("include dummy file systems")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_BLOCKSIZE)
                .short('B')
                .long("block-size")
                .value_name("SIZE")
                .overrides_with_all([options::OPT_KILO, options::OPT_BLOCKSIZE])
                .help(
                    "scale sizes by SIZE before printing them; e.g.\
                    '-BM' prints sizes in units of 1,048,576 bytes",
                ),
        )
        .arg(
            Arg::new(options::OPT_TOTAL)
                .long("total")
                .overrides_with(options::OPT_TOTAL)
                .help("produce a grand total")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_HUMAN_READABLE_BINARY)
                .short('h')
                .long("human-readable")
                .overrides_with_all([
                    options::OPT_HUMAN_READABLE_DECIMAL,
                    options::OPT_HUMAN_READABLE_BINARY,
                ])
                .help("print sizes in human readable format (e.g., 1K 234M 2G)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_HUMAN_READABLE_DECIMAL)
                .short('H')
                .long("si")
                .overrides_with_all([
                    options::OPT_HUMAN_READABLE_BINARY,
                    options::OPT_HUMAN_READABLE_DECIMAL,
                ])
                .help("likewise, but use powers of 1000 not 1024")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_INODES)
                .short('i')
                .long("inodes")
                .overrides_with(options::OPT_INODES)
                .help("list inode information instead of block usage")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_KILO)
                .short('k')
                .help("like --block-size=1K")
                .overrides_with_all([options::OPT_BLOCKSIZE, options::OPT_KILO])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_LOCAL)
                .short('l')
                .long("local")
                .overrides_with(options::OPT_LOCAL)
                .help("limit listing to local file systems")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_NO_SYNC)
                .long("no-sync")
                .overrides_with_all([options::OPT_SYNC, options::OPT_NO_SYNC])
                .help("do not invoke sync before getting usage info (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_OUTPUT)
                .long("output")
                .value_name("FIELD_LIST")
                .action(ArgAction::Append)
                .num_args(0..)
                .require_equals(true)
                .use_value_delimiter(true)
                .value_parser(options::OUTPUT_FIELD_LIST)
                .default_missing_values(options::OUTPUT_FIELD_LIST)
                .default_values(["source", "size", "used", "avail", "pcent", "target"])
                .conflicts_with_all([
                    options::OPT_INODES,
                    options::OPT_PORTABILITY,
                    options::OPT_PRINT_TYPE,
                ])
                .help(
                    "use the output format defined by FIELD_LIST, \
                     or print all fields if FIELD_LIST is omitted.",
                ),
        )
        .arg(
            Arg::new(options::OPT_PORTABILITY)
                .short('P')
                .long("portability")
                .overrides_with(options::OPT_PORTABILITY)
                .help("use the POSIX output format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_SYNC)
                .long("sync")
                .overrides_with_all([options::OPT_NO_SYNC, options::OPT_SYNC])
                .help("invoke sync before getting usage info (non-windows only)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_TYPE)
                .short('t')
                .long("type")
                .value_parser(ValueParser::os_string())
                .value_name("TYPE")
                .action(ArgAction::Append)
                .help("limit listing to file systems of type TYPE"),
        )
        .arg(
            Arg::new(options::OPT_PRINT_TYPE)
                .short('T')
                .long("print-type")
                .overrides_with(options::OPT_PRINT_TYPE)
                .help("print file system type")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_EXCLUDE_TYPE)
                .short('x')
                .long("exclude-type")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_name("TYPE")
                .use_value_delimiter(true)
                .help("limit listing to file systems not of type TYPE"),
        )
        .arg(
            Arg::new(options::OPT_PATHS)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
