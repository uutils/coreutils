use std::num::ParseIntError;

use clap::{crate_version, App, Arg};

fn is_valid_bit_num(arg: String) -> Result<(), String> {
    parse_bit_num(&arg)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}
// TODO: return custom error type
fn parse_bit_num(arg: &str) -> Result<usize, ParseIntError> {
    arg.parse()
}

fn is_custom_binary(program: &str) -> bool {
    matches!(
        program,
        "md5sum"
            | "sha1sum"
            | "sha224sum"
            | "sha256sum"
            | "sha384sum"
            | "sha512sum"
            | "sha3sum"
            | "sha3-224sum"
            | "sha3-256sum"
            | "sha3-384sum"
            | "sha3-512sum"
            | "shake128sum"
            | "shake256sum"
            | "b2sum"
    )
}

pub fn get_app(app_name: &str) -> App {
    #[cfg(windows)]
    const BINARY_HELP: &str = "read in binary mode (default)";
    #[cfg(not(windows))]
    const BINARY_HELP: &str = "read in binary mode";
    #[cfg(windows)]
    const TEXT_HELP: &str = "read in text mode";
    #[cfg(not(windows))]
    const TEXT_HELP: &str = "read in text mode (default)";
    let mut app = App::new(app_name)
        .version(crate_version!())
        .about("Compute and check message digests.")
        .arg(
            Arg::with_name("binary")
                .short("b")
                .long("binary")
                .help(BINARY_HELP),
        )
        .arg(
            Arg::with_name("check")
                .short("c")
                .long("check")
                .help("read hashsums from the FILEs and check them"),
        )
        .arg(
            Arg::with_name("tag")
                .long("tag")
                .help("create a BSD-style checksum"),
        )
        .arg(
            Arg::with_name("text")
                .short("t")
                .long("text")
                .help(TEXT_HELP)
                .conflicts_with("binary"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("don't print OK for each successfully verified file"),
        )
        .arg(
            Arg::with_name("status")
                .short("s")
                .long("status")
                .help("don't output anything, status code shows success"),
        )
        .arg(
            Arg::with_name("strict")
                .long("strict")
                .help("exit non-zero for improperly formatted checksum lines"),
        )
        .arg(
            Arg::with_name("warn")
                .short("w")
                .long("warn")
                .help("warn about improperly formatted checksum lines"),
        )
        // Needed for variable-length output sums (e.g. SHAKE)
        .arg(
            Arg::with_name("bits")
                .long("bits")
                .help("set the size of the output (only for SHAKE)")
                .takes_value(true)
                .value_name("BITS")
                // XXX: should we actually use validators?  they're not particularly efficient
                .validator(is_valid_bit_num),
        )
        .arg(
            Arg::with_name("FILE")
                .index(1)
                .multiple(true)
                .value_name("FILE"),
        );
    if !is_custom_binary(app_name) {
        let algorithms = &[
            ("md5", "work with MD5"),
            ("sha1", "work with SHA1"),
            ("sha224", "work with SHA224"),
            ("sha256", "work with SHA256"),
            ("sha384", "work with SHA384"),
            ("sha512", "work with SHA512"),
            ("sha3", "work with SHA3"),
            ("sha3-224", "work with SHA3-224"),
            ("sha3-256", "work with SHA3-256"),
            ("sha3-384", "work with SHA3-384"),
            ("sha3-512", "work with SHA3-512"),
            (
                "shake128",
                "work with SHAKE128 using BITS for the output size",
            ),
            (
                "shake256",
                "work with SHAKE256 using BITS for the output size",
            ),
            ("b2sum", "work with BLAKE2"),
        ];

        for (name, desc) in algorithms {
            app = app.arg(Arg::with_name(name).long(name).help(desc));
        }
    }
    app
}
