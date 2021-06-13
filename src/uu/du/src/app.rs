use clap::{crate_version, App, Arg};

pub mod options {
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
    pub const SI: &str = "si";
    pub const TIME: &str = "time";
    pub const TIME_STYLE: &str = "time-style";
    pub const ONE_FILE_SYSTEM: &str = "one-file-system";
    pub const FILE: &str = "FILE";
}

const SUMMARY: &str = "estimate file space usage";
const LONG_HELP: &str = "
Display values are in units of the first available SIZE from --block-size,
and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environment variables.
Otherwise, units default to 1024 bytes (or 512 if POSIXLY_CORRECT is set).

SIZE is an integer and optional unit (example: 10M is 10*1024*1024).
Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB,... (powers
of 1000).
";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(SUMMARY)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::ALL)
                .short("a")
                .long(options::ALL)
                .help("write counts for all files, not just directories"),
        )
        .arg(
            Arg::with_name(options::APPARENT_SIZE)
                .long(options::APPARENT_SIZE)
                .help(
                    "print apparent sizes,  rather  than  disk  usage \
                    although  the apparent  size is usually smaller, it may be larger due to holes \
                    in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like"
                )
                .alias("app") // The GNU test suite uses this alias
        )
        .arg(
            Arg::with_name(options::BLOCK_SIZE)
                .short("B")
                .long(options::BLOCK_SIZE)
                .value_name("SIZE")
                .help(
                    "scale sizes  by  SIZE before printing them. \
                    E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below."
                )
        )
        .arg(
            Arg::with_name(options::BYTES)
                .short("b")
                .long("bytes")
                .help("equivalent to '--apparent-size --block-size=1'")
        )
        .arg(
            Arg::with_name(options::TOTAL)
                .long("total")
                .short("c")
                .help("produce a grand total")
        )
        .arg(
            Arg::with_name(options::MAX_DEPTH)
                .short("d")
                .long("max-depth")
                .value_name("N")
                .help(
                    "print the total for a directory (or file, with --all) \
                    only if it is N or fewer levels below the command \
                    line argument;  --max-depth=0 is the same as --summarize"
                )
        )
        .arg(
            Arg::with_name(options::HUMAN_READABLE)
                .long("human-readable")
                .short("h")
                .help("print sizes in human readable format (e.g., 1K 234M 2G)")
        )
        .arg(
            Arg::with_name("inodes")
                .long("inodes")
                .help(
                    "list inode usage information instead of block usage like --block-size=1K"
                )
        )
        .arg(
            Arg::with_name(options::BLOCK_SIZE_1K)
                .short("k")
                .help("like --block-size=1K")
        )
        .arg(
            Arg::with_name(options::COUNT_LINKS)
                .short("l")
                .long("count-links")
                .help("count sizes many times if hard linked")
        )
        // .arg(
        //     Arg::with_name("dereference")
        //         .short("L")
        //         .long("dereference")
        //         .help("dereference all symbolic links")
        // )
        // .arg(
        //     Arg::with_name("no-dereference")
        //         .short("P")
        //         .long("no-dereference")
        //         .help("don't follow any symbolic links (this is the default)")
        // )
        .arg(
            Arg::with_name(options::BLOCK_SIZE_1M)
                .short("m")
                .help("like --block-size=1M")
        )
        .arg(
            Arg::with_name(options::NULL)
                .short("0")
                .long("null")
                .help("end each output line with 0 byte rather than newline")
        )
        .arg(
            Arg::with_name(options::SEPARATE_DIRS)
                .short("S")
                .long("separate-dirs")
                .help("do not include size of subdirectories")
        )
        .arg(
            Arg::with_name(options::SUMMARIZE)
                .short("s")
                .long("summarize")
                .help("display only a total for each argument")
        )
        .arg(
            Arg::with_name(options::SI)
                .long(options::SI)
                .help("like -h, but use powers of 1000 not 1024")
        )
        .arg(
            Arg::with_name(options::ONE_FILE_SYSTEM)
                .short("x")
                .long(options::ONE_FILE_SYSTEM)
                .help("skip directories on different file systems")
        )
        // .arg(
        //     Arg::with_name("")
        //         .short("x")
        //         .long("exclude-from")
        //         .value_name("FILE")
        //         .help("exclude files that match any pattern in FILE")
        // )
        // .arg(
        //     Arg::with_name("exclude")
        //         .long("exclude")
        //         .value_name("PATTERN")
        //         .help("exclude files that match PATTERN")
        // )
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .value_name("WORD")
                .require_equals(true)
                .min_values(0)
                .possible_values(&["atime", "access", "use", "ctime", "status", "birth", "creation"])
                .help(
                    "show time of the last modification of any file in the \
                    directory, or any of its subdirectories.  If WORD is given, show time as WORD instead \
                    of modification time: atime, access, use, ctime, status, birth or creation"
                )
        )
        .arg(
            Arg::with_name(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .value_name("STYLE")
                .help(
                    "show times using style STYLE: \
                    full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'"
                )
        )
        .arg(
            Arg::with_name(options::FILE)
                .hidden(true)
                .multiple(true)
        )
}
