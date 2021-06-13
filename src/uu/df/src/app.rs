use clap::{crate_version, App, Arg};

pub const ABOUT: &str = "Show information about the file system on which each FILE resides,\n\
                      or all file systems by default.";

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERR: i32 = 1;

pub const OPT_ALL: &str = "all";
pub const OPT_BLOCKSIZE: &str = "blocksize";
pub const OPT_DIRECT: &str = "direct";
pub const OPT_TOTAL: &str = "total";
pub const OPT_HUMAN_READABLE: &str = "human-readable";
pub const OPT_HUMAN_READABLE_2: &str = "human-readable-2";
pub const OPT_INODES: &str = "inodes";
pub const OPT_KILO: &str = "kilo";
pub const OPT_LOCAL: &str = "local";
pub const OPT_NO_SYNC: &str = "no-sync";
pub const OPT_OUTPUT: &str = "output";
pub const OPT_PATHS: &str = "paths";
pub const OPT_PORTABILITY: &str = "portability";
pub const OPT_SYNC: &str = "sync";
pub const OPT_TYPE: &str = "type";
pub const OPT_PRINT_TYPE: &str = "print-type";
pub const OPT_EXCLUDE_TYPE: &str = "exclude-type";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_ALL)
                .short("a")
                .long("all")
                .help("include dummy file systems"),
        )
        .arg(
            Arg::with_name(OPT_BLOCKSIZE)
                .short("B")
                .long("block-size")
                .takes_value(true)
                .help(
                    "scale sizes by SIZE before printing them; e.g.\
                     '-BM' prints sizes in units of 1,048,576 bytes",
                ),
        )
        .arg(
            Arg::with_name(OPT_DIRECT)
                .long("direct")
                .help("show statistics for a file instead of mount point"),
        )
        .arg(
            Arg::with_name(OPT_TOTAL)
                .long("total")
                .help("produce a grand total"),
        )
        .arg(
            Arg::with_name(OPT_HUMAN_READABLE)
                .short("h")
                .long("human-readable")
                .conflicts_with(OPT_HUMAN_READABLE_2)
                .help("print sizes in human readable format (e.g., 1K 234M 2G)"),
        )
        .arg(
            Arg::with_name(OPT_HUMAN_READABLE_2)
                .short("H")
                .long("si")
                .conflicts_with(OPT_HUMAN_READABLE)
                .help("likewise, but use powers of 1000 not 1024"),
        )
        .arg(
            Arg::with_name(OPT_INODES)
                .short("i")
                .long("inodes")
                .help("list inode information instead of block usage"),
        )
        .arg(
            Arg::with_name(OPT_KILO)
                .short("k")
                .help("like --block-size=1K"),
        )
        .arg(
            Arg::with_name(OPT_LOCAL)
                .short("l")
                .long("local")
                .help("limit listing to local file systems"),
        )
        .arg(
            Arg::with_name(OPT_NO_SYNC)
                .long("no-sync")
                .conflicts_with(OPT_SYNC)
                .help("do not invoke sync before getting usage info (default)"),
        )
        .arg(
            Arg::with_name(OPT_OUTPUT)
                .long("output")
                .takes_value(true)
                .use_delimiter(true)
                .help(
                    "use the output format defined by FIELD_LIST,\
                     or print all fields if FIELD_LIST is omitted.",
                ),
        )
        .arg(
            Arg::with_name(OPT_PORTABILITY)
                .short("P")
                .long("portability")
                .help("use the POSIX output format"),
        )
        .arg(
            Arg::with_name(OPT_SYNC)
                .long("sync")
                .conflicts_with(OPT_NO_SYNC)
                .help("invoke sync before getting usage info"),
        )
        .arg(
            Arg::with_name(OPT_TYPE)
                .short("t")
                .long("type")
                .takes_value(true)
                .use_delimiter(true)
                .help("limit listing to file systems of type TYPE"),
        )
        .arg(
            Arg::with_name(OPT_PRINT_TYPE)
                .short("T")
                .long("print-type")
                .help("print file system type"),
        )
        .arg(
            Arg::with_name(OPT_EXCLUDE_TYPE)
                .short("x")
                .long("exclude-type")
                .takes_value(true)
                .use_delimiter(true)
                .help("limit listing to file systems not of type TYPE"),
        )
        .arg(Arg::with_name(OPT_PATHS).multiple(true))
        .help("Filesystem(s) to list")
}
