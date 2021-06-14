// spell-checker:ignore (clap) DontDelimitTrailingValues
// spell-checker:ignore (ToDO) exitcode

use clap::{App, AppSettings, Arg};

const ABOUT: &str = "dump files in octal and other formats";

const USAGE: &str = r#"
    od [OPTION]... [--] [FILENAME]...
    od [-abcdDefFhHiIlLoOsxX] [FILENAME] [[+][0x]OFFSET[.][b]]
    od --traditional [OPTION]... [FILENAME] [[+][0x]OFFSET[.][b] [[+][0x]LABEL[.][b]]]"#;

const LONG_HELP: &str = r#"
Displays data in various human-readable formats. If multiple formats are
specified, the output will contain all formats in the order they appear on the
command line. Each format will be printed on a new line. Only the line
containing the first format will be prefixed with the offset.

If no filename is specified, or it is "-", stdin will be used. After a "--", no
more options will be recognized. This allows for filenames starting with a "-".

If a filename is a valid number which can be used as an offset in the second
form, you can force it to be recognized as a filename if you include an option
like "-j0", which is only valid in the first form.

RADIX is one of o,d,x,n for octal, decimal, hexadecimal or none.

BYTES is decimal by default, octal if prefixed with a "0", or hexadecimal if
prefixed with "0x". The suffixes b, KB, K, MB, M, GB, G, will multiply the
number with 512, 1000, 1024, 1000^2, 1024^2, 1000^3, 1024^3, 1000^2, 1024^2.

OFFSET and LABEL are octal by default, hexadecimal if prefixed with "0x" or
decimal if a "." suffix is added. The "b" suffix will multiply with 512.

TYPE contains one or more format specifications consisting of:
    a       for printable 7-bits ASCII
    c       for utf-8 characters or octal for undefined characters
    d[SIZE] for signed decimal
    f[SIZE] for floating point
    o[SIZE] for octal
    u[SIZE] for unsigned decimal
    x[SIZE] for hexadecimal
SIZE is the number of bytes which can be the number 1, 2, 4, 8 or 16,
    or C, I, S, L for 1, 2, 4, 8 bytes for integer types,
    or F, D, L for 4, 8, 16 bytes for floating point.
Any type specification can have a "z" suffix, which will add a ASCII dump at
    the end of the line.

If an error occurred, a diagnostic message will be printed to stderr, and the
exitcode will be non-zero."#;

pub(crate) mod options {
    pub const ADDRESS_RADIX: &str = "address-radix";
    pub const SKIP_BYTES: &str = "skip-bytes";
    pub const READ_BYTES: &str = "read-bytes";
    pub const ENDIAN: &str = "endian";
    pub const STRINGS: &str = "strings";
    pub const FORMAT: &str = "format";
    pub const OUTPUT_DUPLICATES: &str = "output-duplicates";
    pub const TRADITIONAL: &str = "traditional";
    pub const WIDTH: &str = "width";
    pub const FILENAME: &str = "FILENAME";
}
pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(clap::crate_version!())
        .about(ABOUT)
        .usage(USAGE)
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::ADDRESS_RADIX)
                .short("A")
                .long(options::ADDRESS_RADIX)
                .help("Select the base in which file offsets are printed.")
                .value_name("RADIX"),
        )
        .arg(
            Arg::with_name(options::SKIP_BYTES)
                .short("j")
                .long(options::SKIP_BYTES)
                .help("Skip bytes input bytes before formatting and writing.")
                .value_name("BYTES"),
        )
        .arg(
            Arg::with_name(options::READ_BYTES)
                .short("N")
                .long(options::READ_BYTES)
                .help("limit dump to BYTES input bytes")
                .value_name("BYTES"),
        )
        .arg(
            Arg::with_name(options::ENDIAN)
                .long(options::ENDIAN)
                .help("byte order to use for multi-byte formats")
                .possible_values(&["big", "little"])
                .value_name("big|little"),
        )
        .arg(
            Arg::with_name(options::STRINGS)
                .short("S")
                .long(options::STRINGS)
                .help(
                    "NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when \
                     BYTES is not specified.",
                )
                .default_value("3")
                .value_name("BYTES"),
        )
        .arg(
            Arg::with_name("a")
                .short("a")
                .help("named characters, ignoring high-order bit")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("b")
                .short("b")
                .help("octal bytes")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("c")
                .short("c")
                .help("ASCII characters or backslash escapes")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("d")
                .short("d")
                .help("unsigned decimal 2-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("D")
                .short("D")
                .help("unsigned decimal 4-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("o")
                .short("o")
                .help("octal 2-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("I")
                .short("I")
                .help("decimal 8-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("L")
                .short("L")
                .help("decimal 8-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("i")
                .short("i")
                .help("decimal 4-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("l")
                .short("l")
                .help("decimal 8-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("x")
                .short("x")
                .help("hexadecimal 2-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("h")
                .short("h")
                .help("hexadecimal 2-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("O")
                .short("O")
                .help("octal 4-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("s")
                .short("s")
                .help("decimal 2-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("X")
                .short("X")
                .help("hexadecimal 4-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("H")
                .short("H")
                .help("hexadecimal 4-byte units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("e")
                .short("e")
                .help("floating point double precision (64-bit) units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("f")
                .short("f")
                .help("floating point double precision (32-bit) units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("F")
                .short("F")
                .help("floating point double precision (64-bit) units")
                .multiple(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::FORMAT)
                .short("t")
                .long(options::FORMAT)
                .help("select output format or formats")
                .multiple(true)
                .value_name("TYPE"),
        )
        .arg(
            Arg::with_name(options::OUTPUT_DUPLICATES)
                .short("v")
                .long(options::OUTPUT_DUPLICATES)
                .help("do not use * to mark line suppression")
                .takes_value(false)
                .possible_values(&["big", "little"]),
        )
        .arg(
            Arg::with_name(options::WIDTH)
                .short("w")
                .long(options::WIDTH)
                .help(
                    "output BYTES bytes per output line. 32 is implied when BYTES is not \
                     specified.",
                )
                .default_value("32")
                .value_name("BYTES"),
        )
        .arg(
            Arg::with_name(options::TRADITIONAL)
                .long(options::TRADITIONAL)
                .help("compatibility mode with one input, offset and label.")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::FILENAME)
                .hidden(true)
                .multiple(true),
        )
        .settings(&[
            AppSettings::TrailingVarArg,
            AppSettings::DontDelimitTrailingValues,
            AppSettings::DisableVersion,
            AppSettings::DeriveDisplayOrder,
        ])
}
