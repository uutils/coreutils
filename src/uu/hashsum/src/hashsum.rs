// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo, algoname, regexes, nread, nonames

use clap::builder::ValueParser;
use clap::crate_version;
use clap::value_parser;
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{stdin, BufReader, Read};
use std::iter;
use std::num::ParseIntError;
use std::path::Path;
use uucore::checksum::calculate_blake2b_length;
use uucore::checksum::create_sha3;
use uucore::checksum::detect_algo;
use uucore::checksum::digest_reader;
use uucore::checksum::escape_filename;
use uucore::checksum::perform_checksum_validation;
use uucore::checksum::ChecksumError;
use uucore::checksum::HashAlgorithm;
use uucore::error::{FromIo, UResult};
use uucore::sum::{Digest, Sha3_224, Sha3_256, Sha3_384, Sha3_512, Shake128, Shake256};
use uucore::{format_usage, help_about, help_usage};

const NAME: &str = "hashsum";
const ABOUT: &str = help_about!("hashsum.md");
const USAGE: &str = help_usage!("hashsum.md");

struct Options {
    algoname: &'static str,
    digest: Box<dyn Digest + 'static>,
    binary: bool,
    //check: bool,
    tag: bool,
    nonames: bool,
    //status: bool,
    //quiet: bool,
    //strict: bool,
    //warn: bool,
    output_bits: usize,
    zero: bool,
    //ignore_missing: bool,
}

/// Creates a hasher instance based on the command-line flags.
///
/// # Arguments
///
/// * `matches` - A reference to the `ArgMatches` object containing the command-line arguments.
///
/// # Returns
///
/// Returns a UResult of a tuple containing the algorithm name, the hasher instance, and
/// the output length in bits or an Err if multiple hash algorithms are specified or if a
/// required flag is missing.
#[allow(clippy::cognitive_complexity)]
fn create_algorithm_from_flags(matches: &ArgMatches) -> UResult<HashAlgorithm> {
    let mut alg: Option<HashAlgorithm> = None;

    let mut set_or_err = |new_alg: HashAlgorithm| -> UResult<()> {
        if alg.is_some() {
            return Err(ChecksumError::CombineMultipleAlgorithms.into());
        }
        alg = Some(new_alg);
        Ok(())
    };

    if matches.get_flag("md5") {
        set_or_err(detect_algo("md5sum", None)?)?;
    }
    if matches.get_flag("sha1") {
        set_or_err(detect_algo("sha1sum", None)?)?;
    }
    if matches.get_flag("sha224") {
        set_or_err(detect_algo("sha224sum", None)?)?;
    }
    if matches.get_flag("sha256") {
        set_or_err(detect_algo("sha256sum", None)?)?;
    }
    if matches.get_flag("sha384") {
        set_or_err(detect_algo("sha384sum", None)?)?;
    }
    if matches.get_flag("sha512") {
        set_or_err(detect_algo("sha512sum", None)?)?;
    }
    if matches.get_flag("b2sum") {
        set_or_err(detect_algo("b2sum", None)?)?;
    }
    if matches.get_flag("b3sum") {
        set_or_err(detect_algo("b3sum", None)?)?;
    }
    if matches.get_flag("sha3") {
        let bits = matches.get_one::<usize>("bits").cloned();
        set_or_err(create_sha3(bits)?)?;
    }
    if matches.get_flag("sha3-224") {
        set_or_err(HashAlgorithm {
            name: "SHA3-224",
            create_fn: Box::new(|| Box::new(Sha3_224::new())),
            bits: 224,
        })?;
    }
    if matches.get_flag("sha3-256") {
        set_or_err(HashAlgorithm {
            name: "SHA3-256",
            create_fn: Box::new(|| Box::new(Sha3_256::new())),
            bits: 256,
        })?;
    }
    if matches.get_flag("sha3-384") {
        set_or_err(HashAlgorithm {
            name: "SHA3-384",
            create_fn: Box::new(|| Box::new(Sha3_384::new())),
            bits: 384,
        })?;
    }
    if matches.get_flag("sha3-512") {
        set_or_err(HashAlgorithm {
            name: "SHA3-512",
            create_fn: Box::new(|| Box::new(Sha3_512::new())),
            bits: 512,
        })?;
    }
    if matches.get_flag("shake128") {
        match matches.get_one::<usize>("bits") {
            Some(bits) => set_or_err(HashAlgorithm {
                name: "SHAKE128",
                create_fn: Box::new(|| Box::new(Shake128::new())),
                bits: *bits,
            })?,
            None => return Err(ChecksumError::BitsRequiredForShake128.into()),
        };
    }
    if matches.get_flag("shake256") {
        match matches.get_one::<usize>("bits") {
            Some(bits) => set_or_err(HashAlgorithm {
                name: "SHAKE256",
                create_fn: Box::new(|| Box::new(Shake256::new())),
                bits: *bits,
            })?,
            None => return Err(ChecksumError::BitsRequiredForShake256.into()),
        };
    }

    if alg.is_none() {
        return Err(ChecksumError::NeedAlgorithmToHash.into());
    }

    Ok(alg.unwrap())
}

// TODO: return custom error type
fn parse_bit_num(arg: &str) -> Result<usize, ParseIntError> {
    arg.parse()
}

#[uucore::main]
pub fn uumain(mut args: impl uucore::Args) -> UResult<()> {
    // if there is no program name for some reason, default to "hashsum"
    let program = args.next().unwrap_or_else(|| OsString::from(NAME));
    let binary_name = Path::new(&program)
        .file_stem()
        .unwrap_or_else(|| OsStr::new(NAME))
        .to_string_lossy();

    let args = iter::once(program.clone()).chain(args);

    // Default binary in Windows, text mode otherwise
    let binary_flag_default = cfg!(windows);

    let (command, is_hashsum_bin) = uu_app(&binary_name);

    // FIXME: this should use try_get_matches_from() and crash!(), but at the moment that just
    //        causes "error: " to be printed twice (once from crash!() and once from clap).  With
    //        the current setup, the name of the utility is not printed, but I think this is at
    //        least somewhat better from a user's perspective.
    let matches = command.try_get_matches_from(args)?;

    let input_length: Option<&usize> = if binary_name == "b2sum" {
        matches.get_one::<usize>(options::LENGTH)
    } else {
        None
    };

    let length = match input_length {
        Some(length) => calculate_blake2b_length(*length)?,
        None => None,
    };

    let algo = if is_hashsum_bin {
        create_algorithm_from_flags(&matches)?
    } else {
        detect_algo(&binary_name, length)?
    };

    let binary = if matches.get_flag("binary") {
        true
    } else if matches.get_flag("text") {
        false
    } else {
        binary_flag_default
    };
    let check = matches.get_flag("check");
    let status = matches.get_flag("status");
    let quiet = matches.get_flag("quiet") || status;
    let strict = matches.get_flag("strict");
    let warn = matches.get_flag("warn") && !status;
    let ignore_missing = matches.get_flag("ignore-missing");

    if ignore_missing && !check {
        // --ignore-missing needs -c
        return Err(ChecksumError::IgnoreNotCheck.into());
    }

    if check {
        let text_flag = matches.get_flag("text");
        let binary_flag = matches.get_flag("binary");

        if binary_flag || text_flag {
            return Err(ChecksumError::BinaryTextConflict.into());
        }

        // Execute the checksum validation based on the presence of files or the use of stdin
        // Determine the source of input: a list of files or stdin.
        let input = matches.get_many::<OsString>(options::FILE).map_or_else(
            || iter::once(OsStr::new("-")).collect::<Vec<_>>(),
            |files| files.map(OsStr::new).collect::<Vec<_>>(),
        );

        // Execute the checksum validation
        return perform_checksum_validation(
            input.iter().copied(),
            strict,
            status,
            warn,
            binary_flag,
            ignore_missing,
            quiet,
            Some(algo.name),
            Some(algo.bits),
        );
    } else if quiet {
        return Err(ChecksumError::QuietNotCheck.into());
    } else if strict {
        return Err(ChecksumError::StrictNotCheck.into());
    }

    let nonames = *matches
        .try_get_one("no-names")
        .unwrap_or(None)
        .unwrap_or(&false);
    let zero = matches.get_flag("zero");

    let opts = Options {
        algoname: algo.name,
        digest: (algo.create_fn)(),
        output_bits: algo.bits,
        binary,
        tag: matches.get_flag("tag"),
        nonames,
        //status,
        //quiet,
        //warn,
        zero,
        //ignore_missing,
    };

    // Show the hashsum of the input
    match matches.get_many::<OsString>(options::FILE) {
        Some(files) => hashsum(opts, files.map(|f| f.as_os_str())),
        None => hashsum(opts, iter::once(OsStr::new("-"))),
    }
}

mod options {
    //pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    //pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    //pub const RAW: &str = "raw";
    //pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    pub const STRICT: &str = "strict";
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
    pub const STATUS: &str = "status";
    pub const WARN: &str = "warn";
    pub const QUIET: &str = "quiet";
}

pub fn uu_app_common() -> Command {
    #[cfg(windows)]
    const BINARY_HELP: &str = "read in binary mode (default)";
    #[cfg(not(windows))]
    const BINARY_HELP: &str = "read in binary mode";
    #[cfg(windows)]
    const TEXT_HELP: &str = "read in text mode";
    #[cfg(not(windows))]
    const TEXT_HELP: &str = "read in text mode (default)";
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::BINARY)
                .short('b')
                .long("binary")
                .help(BINARY_HELP)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long("check")
                .help("read hashsums from the FILEs and check them")
                .action(ArgAction::SetTrue)
                .conflicts_with("tag"),
        )
        .arg(
            Arg::new(options::TAG)
                .long("tag")
                .help("create a BSD-style checksum")
                .action(ArgAction::SetTrue)
                .conflicts_with("text"),
        )
        .arg(
            Arg::new(options::TEXT)
                .short('t')
                .long("text")
                .help(TEXT_HELP)
                .conflicts_with("binary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('q')
                .long(options::QUIET)
                .help("don't print OK for each successfully verified file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STATUS)
                .short('s')
                .long("status")
                .help("don't output anything, status code shows success")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRICT)
                .long("strict")
                .help("exit non-zero for improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("ignore-missing")
                .long("ignore-missing")
                .help("don't fail or report status for missing files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WARN)
                .short('w')
                .long("warn")
                .help("warn about improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("zero")
                .short('z')
                .long("zero")
                .help("end each output line with NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .index(1)
                .action(ArgAction::Append)
                .value_name(options::FILE)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string()),
        )
}

pub fn uu_app_length() -> Command {
    uu_app_opt_length(uu_app_common())
}

fn uu_app_opt_length(command: Command) -> Command {
    command.arg(
        Arg::new(options::LENGTH)
            .long(options::LENGTH)
            .value_parser(value_parser!(usize))
            .short('l')
            .help(
                "digest length in bits; must not exceed the max for the blake2 algorithm \
                    and must be a multiple of 8",
            )
            .overrides_with(options::LENGTH)
            .action(ArgAction::Set),
    )
}

pub fn uu_app_b3sum() -> Command {
    uu_app_b3sum_opts(uu_app_common())
}

fn uu_app_b3sum_opts(command: Command) -> Command {
    command.arg(
        Arg::new("no-names")
            .long("no-names")
            .help("Omits filenames in the output (option not present in GNU/Coreutils)")
            .action(ArgAction::SetTrue),
    )
}

pub fn uu_app_bits() -> Command {
    uu_app_opt_bits(uu_app_common())
}

fn uu_app_opt_bits(command: Command) -> Command {
    // Needed for variable-length output sums (e.g. SHAKE)
    command.arg(
        Arg::new("bits")
            .long("bits")
            .help("set the size of the output (only for SHAKE)")
            .value_name("BITS")
            // XXX: should we actually use validators?  they're not particularly efficient
            .value_parser(parse_bit_num),
    )
}

pub fn uu_app_custom() -> Command {
    let mut command = uu_app_b3sum_opts(uu_app_opt_bits(uu_app_common()));
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
        ("b3sum", "work with BLAKE3"),
    ];

    for (name, desc) in algorithms {
        command = command.arg(
            Arg::new(*name)
                .long(name)
                .help(*desc)
                .action(ArgAction::SetTrue),
        );
    }
    command
}

// hashsum is handled differently in build.rs, therefore this is not the same
// as in other utilities.
fn uu_app(binary_name: &str) -> (Command, bool) {
    match binary_name {
        // These all support the same options.
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum" => {
            (uu_app_common(), false)
        }
        // b2sum supports the md5sum options plus -l/--length.
        "b2sum" => (uu_app_length(), false),
        // These have never been part of GNU Coreutils, but can function with the same
        // options as md5sum.
        "sha3-224sum" | "sha3-256sum" | "sha3-384sum" | "sha3-512sum" => (uu_app_common(), false),
        // These have never been part of GNU Coreutils, and require an additional --bits
        // option to specify their output size.
        "sha3sum" | "shake128sum" | "shake256sum" => (uu_app_bits(), false),
        // b3sum has never been part of GNU Coreutils, and has a --no-names option in
        // addition to the b2sum options.
        "b3sum" => (uu_app_b3sum(), false),
        // We're probably just being called as `hashsum`, so give them everything.
        _ => (uu_app_custom(), true),
    }
}

#[allow(clippy::cognitive_complexity)]
fn hashsum<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let binary_marker = if options.binary { "*" } else { " " };
    for filename in files {
        let filename = Path::new(filename);

        let stdin_buf;
        let file_buf;
        let mut file = BufReader::new(if filename == OsStr::new("-") {
            stdin_buf = stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else {
            file_buf =
                File::open(filename).map_err_context(|| "failed to open file".to_string())?;
            Box::new(file_buf) as Box<dyn Read>
        });

        let (sum, _) = digest_reader(
            &mut options.digest,
            &mut file,
            options.binary,
            options.output_bits,
        )
        .map_err_context(|| "failed to read input".to_string())?;
        let (escaped_filename, prefix) = escape_filename(filename);
        if options.tag {
            if options.algoname == "blake2b" {
                if options.digest.output_bits() == 512 {
                    println!("BLAKE2b ({escaped_filename}) = {sum}");
                } else {
                    // special case for BLAKE2b with non-default output length
                    println!(
                        "BLAKE2b-{} ({escaped_filename}) = {sum}",
                        options.digest.output_bits()
                    );
                }
            } else {
                println!(
                    "{prefix}{} ({escaped_filename}) = {sum}",
                    options.algoname.to_ascii_uppercase()
                );
            }
        } else if options.nonames {
            println!("{sum}");
        } else if options.zero {
            // with zero, we don't escape the filename
            print!("{sum} {binary_marker}{}\0", filename.display());
        } else {
            println!("{prefix}{sum} {binary_marker}{escaped_filename}");
        }
    }
    Ok(())
}
