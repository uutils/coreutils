// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo, algoname, regexes, nread, nonames

use clap::ArgAction;
use clap::builder::ValueParser;
use clap::value_parser;
use clap::{Arg, ArgMatches, Command};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufReader, Read, stdin};
use std::iter;
use std::num::ParseIntError;
use std::path::Path;
use uucore::checksum::ChecksumError;
use uucore::checksum::ChecksumOptions;
use uucore::checksum::ChecksumVerbose;
use uucore::checksum::HashAlgorithm;
use uucore::checksum::calculate_blake2b_length;
use uucore::checksum::create_sha3;
use uucore::checksum::detect_algo;
use uucore::checksum::digest_reader;
use uucore::checksum::escape_filename;
use uucore::checksum::perform_checksum_validation;
use uucore::error::{UResult, strip_errno};
use uucore::format_usage;
use uucore::sum::{Digest, Sha3_224, Sha3_256, Sha3_384, Sha3_512, Shake128, Shake256};
use uucore::translate;

const NAME: &str = "hashsum";
// Using the same read buffer size as GNU
const READ_BUFFER_SIZE: usize = 32 * 1024;

struct Options<'a> {
    algoname: &'static str,
    digest: Box<dyn Digest + 'static>,
    binary: bool,
    binary_name: &'a str,
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
/// Returns a [`UResult`] of a tuple containing the algorithm name, the hasher instance, and
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
        match matches.get_one::<usize>("bits") {
            Some(bits) => set_or_err(create_sha3(*bits)?)?,
            None => return Err(ChecksumError::LengthRequired("SHA3".into()).into()),
        }
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
            None => return Err(ChecksumError::LengthRequired("SHAKE128".into()).into()),
        }
    }
    if matches.get_flag("shake256") {
        match matches.get_one::<usize>("bits") {
            Some(bits) => set_or_err(HashAlgorithm {
                name: "SHAKE256",
                create_fn: Box::new(|| Box::new(Shake256::new())),
                bits: *bits,
            })?,
            None => return Err(ChecksumError::LengthRequired("SHAKE256".into()).into()),
        }
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
    let matches = uucore::clap_localization::handle_clap_result(command, args)?;

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
        // on Windows, allow --binary/--text to be used with --check
        // and keep the behavior of defaulting to binary
        #[cfg(not(windows))]
        let binary = {
            let text_flag = matches.get_flag("text");
            let binary_flag = matches.get_flag("binary");

            if binary_flag || text_flag {
                return Err(ChecksumError::BinaryTextConflict.into());
            }

            false
        };

        // Execute the checksum validation based on the presence of files or the use of stdin
        // Determine the source of input: a list of files or stdin.
        let input = matches.get_many::<OsString>(options::FILE).map_or_else(
            || iter::once(OsStr::new("-")).collect::<Vec<_>>(),
            |files| files.map(OsStr::new).collect::<Vec<_>>(),
        );

        let verbose = ChecksumVerbose::new(status, quiet, warn);

        let opts = ChecksumOptions {
            binary,
            ignore_missing,
            strict,
            verbose,
        };

        // Execute the checksum validation
        return perform_checksum_validation(
            input.iter().copied(),
            Some(algo.name),
            Some(algo.bits),
            opts,
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
        binary_name: &binary_name,
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
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("hashsum-about"))
        .override_usage(format_usage(&translate!("hashsum-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::BINARY)
                .short('b')
                .long("binary")
                .help({
                    #[cfg(windows)]
                    {
                        translate!("hashsum-help-binary-windows")
                    }
                    #[cfg(not(windows))]
                    {
                        translate!("hashsum-help-binary-other")
                    }
                })
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long("check")
                .help(translate!("hashsum-help-check"))
                .action(ArgAction::SetTrue)
                .conflicts_with("tag"),
        )
        .arg(
            Arg::new(options::TAG)
                .long("tag")
                .help(translate!("hashsum-help-tag"))
                .action(ArgAction::SetTrue)
                .conflicts_with("text"),
        )
        .arg(
            Arg::new(options::TEXT)
                .short('t')
                .long("text")
                .help({
                    #[cfg(windows)]
                    {
                        translate!("hashsum-help-text-windows")
                    }
                    #[cfg(not(windows))]
                    {
                        translate!("hashsum-help-text-other")
                    }
                })
                .conflicts_with("binary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('q')
                .long(options::QUIET)
                .help(translate!("hashsum-help-quiet"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::STATUS, options::WARN]),
        )
        .arg(
            Arg::new(options::STATUS)
                .short('s')
                .long("status")
                .help(translate!("hashsum-help-status"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::QUIET, options::WARN]),
        )
        .arg(
            Arg::new(options::STRICT)
                .long("strict")
                .help(translate!("hashsum-help-strict"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("ignore-missing")
                .long("ignore-missing")
                .help(translate!("hashsum-help-ignore-missing"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WARN)
                .short('w')
                .long("warn")
                .help(translate!("hashsum-help-warn"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::QUIET, options::STATUS]),
        )
        .arg(
            Arg::new("zero")
                .short('z')
                .long("zero")
                .help(translate!("hashsum-help-zero"))
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
            .help(translate!("hashsum-help-length"))
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
            .help(translate!("hashsum-help-no-names"))
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
            .help(translate!("hashsum-help-bits"))
            .value_name("BITS")
            // XXX: should we actually use validators?  they're not particularly efficient
            .value_parser(parse_bit_num),
    )
}

pub fn uu_app_custom() -> Command {
    let mut command = uu_app_b3sum_opts(uu_app_opt_bits(uu_app_common()));
    let algorithms = &[
        ("md5", translate!("hashsum-help-md5")),
        ("sha1", translate!("hashsum-help-sha1")),
        ("sha224", translate!("hashsum-help-sha224")),
        ("sha256", translate!("hashsum-help-sha256")),
        ("sha384", translate!("hashsum-help-sha384")),
        ("sha512", translate!("hashsum-help-sha512")),
        ("sha3", translate!("hashsum-help-sha3")),
        ("sha3-224", translate!("hashsum-help-sha3-224")),
        ("sha3-256", translate!("hashsum-help-sha3-256")),
        ("sha3-384", translate!("hashsum-help-sha3-384")),
        ("sha3-512", translate!("hashsum-help-sha3-512")),
        ("shake128", translate!("hashsum-help-shake128")),
        ("shake256", translate!("hashsum-help-shake256")),
        ("b2sum", translate!("hashsum-help-b2sum")),
        ("b3sum", translate!("hashsum-help-b3sum")),
    ];

    for (name, desc) in algorithms {
        command = command.arg(
            Arg::new(*name)
                .long(name)
                .help(desc)
                .action(ArgAction::SetTrue),
        );
    }
    command
}

/// hashsum is handled differently in build.rs
/// therefore, this is different from other utilities.
fn uu_app(binary_name: &str) -> (Command, bool) {
    let (command, is_hashsum_bin) = match binary_name {
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
    };

    // If not called as generic hashsum, override the command name and usage
    let command = if is_hashsum_bin {
        command
    } else {
        let usage = translate!("hashsum-usage-specific", "utility_name" => binary_name);
        command
            .help_template(uucore::localized_help_template(binary_name))
            .override_usage(format_usage(&usage))
    };

    (command, is_hashsum_bin)
}

#[allow(clippy::cognitive_complexity)]
fn hashsum<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let binary_marker = if options.binary { "*" } else { " " };
    let mut err_found = None;
    for filename in files {
        let filename = Path::new(filename);

        let mut file = BufReader::with_capacity(
            READ_BUFFER_SIZE,
            if filename == OsStr::new("-") {
                Box::new(stdin()) as Box<dyn Read>
            } else {
                let file_buf = match File::open(filename) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!(
                            "{}: {}: {}",
                            options.binary_name,
                            filename.to_string_lossy(),
                            strip_errno(&e)
                        );
                        err_found = Some(ChecksumError::Io(e));
                        continue;
                    }
                };
                Box::new(file_buf) as Box<dyn Read>
            },
        );

        let sum = match digest_reader(
            &mut options.digest,
            &mut file,
            options.binary,
            options.output_bits,
        ) {
            Ok((sum, _)) => sum,
            Err(e) => {
                eprintln!(
                    "{}: {}: {}",
                    options.binary_name,
                    filename.to_string_lossy(),
                    strip_errno(&e)
                );
                err_found = Some(ChecksumError::Io(e));
                continue;
            }
        };

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
    match err_found {
        None => Ok(()),
        Some(e) => Err(Box::new(e)),
    }
}
