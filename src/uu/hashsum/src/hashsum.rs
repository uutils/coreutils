// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo, algoname, regexes, nread, nonames

use clap::ArgMatches;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{stdin, BufReader, Read};
use std::iter;
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

pub const NAME: &str = "hashsum";

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

    let (command, is_hashsum_bin) = crate::uu_app(&binary_name);

    // FIXME: this should use try_get_matches_from() and crash!(), but at the moment that just
    //        causes "error: " to be printed twice (once from crash!() and once from clap).  With
    //        the current setup, the name of the utility is not printed, but I think this is at
    //        least somewhat better from a user's perspective.
    let matches = command.try_get_matches_from(args)?;

    let input_length: Option<&usize> = if binary_name == "b2sum" {
        matches.get_one::<usize>(crate::options::LENGTH)
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
    let warn = matches.get_flag("warn") && !status;
    let ignore_missing = matches.get_flag("ignore-missing");

    if ignore_missing && !check {
        // --ignore-missing needs -c
        return Err(ChecksumError::IgnoreNotCheck.into());
    }

    if check {
        let text_flag = matches.get_flag("text");
        let binary_flag = matches.get_flag("binary");
        let strict = matches.get_flag("strict");

        if binary_flag || text_flag {
            return Err(ChecksumError::BinaryTextConflict.into());
        }

        // Execute the checksum validation based on the presence of files or the use of stdin
        // Determine the source of input: a list of files or stdin.
        let input = matches
            .get_many::<OsString>(crate::options::FILE)
            .map_or_else(
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
    match matches.get_many::<OsString>(crate::options::FILE) {
        Some(files) => hashsum(opts, files.map(|f| f.as_os_str())),
        None => hashsum(opts, iter::once(OsStr::new("-"))),
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
