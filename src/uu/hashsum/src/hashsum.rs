//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  * (c) Vsevolod Velichko <torkvemada@sorokdva.net>
//  * (c) Gil Cottle <gcottle@redtown.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo, algoname, regexes, nread, nonames

use clap::builder::ValueParser;
use clap::crate_version;
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};
use hex::encode;
use regex::Regex;
use std::cmp::Ordering;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Read};
use std::iter;
use std::num::ParseIntError;
use std::path::Path;
use uucore::error::{FromIo, UError, UResult};
use uucore::sum::{
    Blake2b, Blake3, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha3_224, Sha3_256,
    Sha3_384, Sha3_512, Sha512, Shake128, Shake256,
};
use uucore::{crash, display::Quotable, show_warning};
use uucore::{format_usage, help_about, help_usage};

const NAME: &str = "hashsum";
const ABOUT: &str = help_about!("hashsum.md");
const USAGE: &str = help_usage!("hashsum.md");

struct Options {
    algoname: &'static str,
    digest: Box<dyn Digest + 'static>,
    binary: bool,
    check: bool,
    tag: bool,
    nonames: bool,
    status: bool,
    quiet: bool,
    strict: bool,
    warn: bool,
    output_bits: usize,
    zero: bool,
}

/// Creates a Blake2b hasher instance based on the specified length argument.
///
/// # Returns
///
/// Returns a tuple containing the algorithm name, the hasher instance, and the output length in bits.
///
/// # Panics
///
/// Panics if the length is not a multiple of 8 or if it is greater than 512.
fn create_blake2b(matches: &ArgMatches) -> (&'static str, Box<dyn Digest>, usize) {
    match matches.get_one::<usize>("length") {
        Some(0) | None => ("BLAKE2", Box::new(Blake2b::new()) as Box<dyn Digest>, 512),
        Some(length_in_bits) => {
            if *length_in_bits > 512 {
                crash!(1, "Invalid length (maximum digest length is 512 bits)")
            }

            if length_in_bits % 8 == 0 {
                let length_in_bytes = length_in_bits / 8;
                (
                    "BLAKE2",
                    Box::new(Blake2b::with_output_bytes(length_in_bytes)),
                    *length_in_bits,
                )
            } else {
                crash!(1, "Invalid length (expected a multiple of 8)")
            }
        }
    }
}

/// Creates a SHA3 hasher instance based on the specified bits argument.
///
/// # Returns
///
/// Returns a tuple containing the algorithm name, the hasher instance, and the output length in bits.
///
/// # Panics
///
/// Panics if an unsupported output size is provided, or if the `--bits` flag is missing.
fn create_sha3(matches: &ArgMatches) -> (&'static str, Box<dyn Digest>, usize) {
    match matches.get_one::<usize>("bits") {
        Some(224) => (
            "SHA3-224",
            Box::new(Sha3_224::new()) as Box<dyn Digest>,
            224,
        ),
        Some(256) => (
            "SHA3-256",
            Box::new(Sha3_256::new()) as Box<dyn Digest>,
            256,
        ),
        Some(384) => (
            "SHA3-384",
            Box::new(Sha3_384::new()) as Box<dyn Digest>,
            384,
        ),
        Some(512) => (
            "SHA3-512",
            Box::new(Sha3_512::new()) as Box<dyn Digest>,
            512,
        ),
        Some(_) => crash!(
            1,
            "Invalid output size for SHA3 (expected 224, 256, 384, or 512)"
        ),
        None => crash!(1, "--bits required for SHA3"),
    }
}

/// Creates a SHAKE-128 hasher instance based on the specified bits argument.
///
/// # Returns
///
/// Returns a tuple containing the algorithm name, the hasher instance, and the output length in bits.
///
/// # Panics
///
/// Panics if the `--bits` flag is missing.
fn create_shake128(matches: &ArgMatches) -> (&'static str, Box<dyn Digest>, usize) {
    match matches.get_one::<usize>("bits") {
        Some(bits) => (
            "SHAKE128",
            Box::new(Shake128::new()) as Box<dyn Digest>,
            *bits,
        ),
        None => crash!(1, "--bits required for SHAKE-128"),
    }
}

/// Creates a SHAKE-256 hasher instance based on the specified bits argument.
///
/// # Returns
///
/// Returns a tuple containing the algorithm name, the hasher instance, and the output length in bits.
///
/// # Panics
///
/// Panics if the `--bits` flag is missing.
fn create_shake256(matches: &ArgMatches) -> (&'static str, Box<dyn Digest>, usize) {
    match matches.get_one::<usize>("bits") {
        Some(bits) => (
            "SHAKE256",
            Box::new(Shake256::new()) as Box<dyn Digest>,
            *bits,
        ),
        None => crash!(1, "--bits required for SHAKE-256"),
    }
}

/// Detects the hash algorithm from the program name or command-line arguments.
///
/// # Arguments
///
/// * `program` - A string slice containing the program name.
/// * `matches` - A reference to the `ArgMatches` object containing the command-line arguments.
///
/// # Returns
///
/// Returns a tuple containing the algorithm name, the hasher instance, and the output length in bits.
fn detect_algo(
    program: &str,
    matches: &ArgMatches,
) -> (&'static str, Box<dyn Digest + 'static>, usize) {
    let (name, alg, output_bits) = match program {
        "md5sum" => ("MD5", Box::new(Md5::new()) as Box<dyn Digest>, 128),
        "sha1sum" => ("SHA1", Box::new(Sha1::new()) as Box<dyn Digest>, 160),
        "sha224sum" => ("SHA224", Box::new(Sha224::new()) as Box<dyn Digest>, 224),
        "sha256sum" => ("SHA256", Box::new(Sha256::new()) as Box<dyn Digest>, 256),
        "sha384sum" => ("SHA384", Box::new(Sha384::new()) as Box<dyn Digest>, 384),
        "sha512sum" => ("SHA512", Box::new(Sha512::new()) as Box<dyn Digest>, 512),
        "b2sum" => create_blake2b(matches),
        "b3sum" => ("BLAKE3", Box::new(Blake3::new()) as Box<dyn Digest>, 256),
        "sha3sum" => create_sha3(matches),
        "sha3-224sum" => (
            "SHA3-224",
            Box::new(Sha3_224::new()) as Box<dyn Digest>,
            224,
        ),
        "sha3-256sum" => (
            "SHA3-256",
            Box::new(Sha3_256::new()) as Box<dyn Digest>,
            256,
        ),
        "sha3-384sum" => (
            "SHA3-384",
            Box::new(Sha3_384::new()) as Box<dyn Digest>,
            384,
        ),
        "sha3-512sum" => (
            "SHA3-512",
            Box::new(Sha3_512::new()) as Box<dyn Digest>,
            512,
        ),
        "shake128sum" => create_shake128(matches),
        "shake256sum" => create_shake256(matches),
        _ => create_algorithm_from_flags(matches),
    };
    (name, alg, output_bits)
}

/// Creates a hasher instance based on the command-line flags.
///
/// # Arguments
///
/// * `matches` - A reference to the `ArgMatches` object containing the command-line arguments.
///
/// # Returns
///
/// Returns a tuple containing the algorithm name, the hasher instance, and the output length in bits.
///
/// # Panics
///
/// Panics if multiple hash algorithms are specified or if a required flag is missing.
#[allow(clippy::cognitive_complexity)]
fn create_algorithm_from_flags(matches: &ArgMatches) -> (&'static str, Box<dyn Digest>, usize) {
    let mut alg: Option<Box<dyn Digest>> = None;
    let mut name: &'static str = "";
    let mut output_bits = 0;
    let mut set_or_crash = |n, val, bits| {
        if alg.is_some() {
            crash!(1, "You cannot combine multiple hash algorithms!");
        };
        name = n;
        alg = Some(val);
        output_bits = bits;
    };

    if matches.get_flag("md5") {
        set_or_crash("MD5", Box::new(Md5::new()), 128);
    }
    if matches.get_flag("sha1") {
        set_or_crash("SHA1", Box::new(Sha1::new()), 160);
    }
    if matches.get_flag("sha224") {
        set_or_crash("SHA224", Box::new(Sha224::new()), 224);
    }
    if matches.get_flag("sha256") {
        set_or_crash("SHA256", Box::new(Sha256::new()), 256);
    }
    if matches.get_flag("sha384") {
        set_or_crash("SHA384", Box::new(Sha384::new()), 384);
    }
    if matches.get_flag("sha512") {
        set_or_crash("SHA512", Box::new(Sha512::new()), 512);
    }
    if matches.get_flag("b2sum") {
        set_or_crash("BLAKE2", Box::new(Blake2b::new()), 512);
    }
    if matches.get_flag("b3sum") {
        set_or_crash("BLAKE3", Box::new(Blake3::new()), 256);
    }
    if matches.get_flag("sha3") {
        let (n, val, bits) = create_sha3(matches);
        set_or_crash(n, val, bits);
    }
    if matches.get_flag("sha3-224") {
        set_or_crash("SHA3-224", Box::new(Sha3_224::new()), 224);
    }
    if matches.get_flag("sha3-256") {
        set_or_crash("SHA3-256", Box::new(Sha3_256::new()), 256);
    }
    if matches.get_flag("sha3-384") {
        set_or_crash("SHA3-384", Box::new(Sha3_384::new()), 384);
    }
    if matches.get_flag("sha3-512") {
        set_or_crash("SHA3-512", Box::new(Sha3_512::new()), 512);
    }
    if matches.get_flag("shake128") {
        match matches.get_one::<usize>("bits") {
            Some(bits) => set_or_crash("SHAKE128", Box::new(Shake128::new()), *bits),
            None => crash!(1, "--bits required for SHAKE-128"),
        }
    }
    if matches.get_flag("shake256") {
        match matches.get_one::<usize>("bits") {
            Some(bits) => set_or_crash("SHAKE256", Box::new(Shake256::new()), *bits),
            None => crash!(1, "--bits required for SHAKE-256"),
        }
    }

    let alg = alg.unwrap_or_else(|| crash!(1, "You must specify hash algorithm!"));
    (name, alg, output_bits)
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
        .file_name()
        .unwrap_or_else(|| OsStr::new(NAME))
        .to_string_lossy();

    let args = iter::once(program.clone()).chain(args);

    // Default binary in Windows, text mode otherwise
    let binary_flag_default = cfg!(windows);

    let command = uu_app(&binary_name);

    // FIXME: this should use try_get_matches_from() and crash!(), but at the moment that just
    //        causes "error: " to be printed twice (once from crash!() and once from clap).  With
    //        the current setup, the name of the utility is not printed, but I think this is at
    //        least somewhat better from a user's perspective.
    let matches = command.try_get_matches_from(args)?;

    let (name, algo, bits) = detect_algo(&binary_name, &matches);

    let binary = if matches.get_flag("binary") {
        true
    } else if matches.get_flag("text") {
        false
    } else {
        binary_flag_default
    };
    let check = matches.get_flag("check");
    let tag = matches.get_flag("tag");
    let nonames = *matches
        .try_get_one("no-names")
        .unwrap_or(None)
        .unwrap_or(&false);
    let status = matches.get_flag("status");
    let quiet = matches.get_flag("quiet") || status;
    let strict = matches.get_flag("strict");
    let warn = matches.get_flag("warn") && !status;
    let zero = matches.get_flag("zero");

    let opts = Options {
        algoname: name,
        digest: algo,
        output_bits: bits,
        binary,
        check,
        tag,
        nonames,
        status,
        quiet,
        strict,
        warn,
        zero,
    };

    match matches.get_many::<OsString>("FILE") {
        Some(files) => hashsum(opts, files.map(|f| f.as_os_str())),
        None => hashsum(opts, iter::once(OsStr::new("-"))),
    }
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
        .arg(
            Arg::new("binary")
                .short('b')
                .long("binary")
                .help(BINARY_HELP)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("check")
                .short('c')
                .long("check")
                .help("read hashsums from the FILEs and check them")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tag")
                .long("tag")
                .help("create a BSD-style checksum")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("text")
                .short('t')
                .long("text")
                .help(TEXT_HELP)
                .conflicts_with("binary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("don't print OK for each successfully verified file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("status")
                .short('s')
                .long("status")
                .help("don't output anything, status code shows success")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("strict")
                .long("strict")
                .help("exit non-zero for improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("warn")
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
            Arg::new("FILE")
                .index(1)
                .action(ArgAction::Append)
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string()),
        )
}

pub fn uu_app_length() -> Command {
    uu_app_opt_length(uu_app_common())
}

fn uu_app_opt_length(command: Command) -> Command {
    command.arg(
        Arg::new("length")
            .short('l')
            .long("length")
            .help("digest length in bits; must not exceed the max for the blake2 algorithm (512) and must be a multiple of 8")
            .value_name("BITS")
            .value_parser(parse_bit_num),
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
fn uu_app(binary_name: &str) -> Command {
    match binary_name {
        // These all support the same options.
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum" => {
            uu_app_common()
        }
        // b2sum supports the md5sum options plus -l/--length.
        "b2sum" => uu_app_length(),
        // These have never been part of GNU Coreutils, but can function with the same
        // options as md5sum.
        "sha3-224sum" | "sha3-256sum" | "sha3-384sum" | "sha3-512sum" => uu_app_common(),
        // These have never been part of GNU Coreutils, and require an additional --bits
        // option to specify their output size.
        "sha3sum" | "shake128sum" | "shake256sum" => uu_app_bits(),
        // b3sum has never been part of GNU Coreutils, and has a --no-names option in
        // addition to the b2sum options.
        "b3sum" => uu_app_b3sum(),
        // We're probably just being called as `hashsum`, so give them everything.
        _ => uu_app_custom(),
    }
}

#[derive(Debug)]
enum HashsumError {
    InvalidRegex,
    InvalidFormat,
}

impl Error for HashsumError {}
impl UError for HashsumError {}

impl std::fmt::Display for HashsumError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidRegex => write!(f, "invalid regular expression"),
            Self::InvalidFormat => Ok(()),
        }
    }
}

#[allow(clippy::cognitive_complexity)]
fn hashsum<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let mut bad_format = 0;
    let mut failed_cksum = 0;
    let mut failed_open_file = 0;
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
        if options.check {
            // Set up Regexes for line validation and parsing
            //
            // First, we compute the number of bytes we expect to be in
            // the digest string. If the algorithm has a variable number
            // of output bits, then we use the `+` modifier in the
            // regular expression, otherwise we use the `{n}` modifier,
            // where `n` is the number of bytes.
            let bytes = options.digest.output_bits() / 4;
            let modifier = if bytes > 0 {
                format!("{{{bytes}}}")
            } else {
                "+".to_string()
            };
            let gnu_re = Regex::new(&format!(
                r"^(?P<digest>[a-fA-F0-9]{modifier}) (?P<binary>[ \*])(?P<fileName>.*)",
            ))
            .map_err(|_| HashsumError::InvalidRegex)?;
            let bsd_re = Regex::new(&format!(
                r"^{algorithm} \((?P<fileName>.*)\) = (?P<digest>[a-fA-F0-9]{digest_size})",
                algorithm = options.algoname,
                digest_size = modifier,
            ))
            .map_err(|_| HashsumError::InvalidRegex)?;

            let buffer = file;
            for (i, maybe_line) in buffer.lines().enumerate() {
                let line = match maybe_line {
                    Ok(l) => l,
                    Err(e) => return Err(e.map_err_context(|| "failed to read file".to_string())),
                };
                let (ck_filename, sum, binary_check) = match gnu_re.captures(&line) {
                    Some(caps) => (
                        caps.name("fileName").unwrap().as_str(),
                        caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
                        caps.name("binary").unwrap().as_str() == "*",
                    ),
                    None => match bsd_re.captures(&line) {
                        Some(caps) => (
                            caps.name("fileName").unwrap().as_str(),
                            caps.name("digest").unwrap().as_str().to_ascii_lowercase(),
                            true,
                        ),
                        None => {
                            bad_format += 1;
                            if options.strict {
                                return Err(HashsumError::InvalidFormat.into());
                            }
                            if options.warn {
                                show_warning!(
                                    "{}: {}: improperly formatted {} checksum line",
                                    filename.maybe_quote(),
                                    i + 1,
                                    options.algoname
                                );
                            }
                            continue;
                        }
                    },
                };
                let f = match File::open(ck_filename) {
                    Err(_) => {
                        failed_open_file += 1;
                        println!(
                            "{}: {}: No such file or directory",
                            uucore::util_name(),
                            ck_filename
                        );
                        println!("{ck_filename}: FAILED open or read");
                        continue;
                    }
                    Ok(file) => file,
                };
                let mut ckf = BufReader::new(Box::new(f) as Box<dyn Read>);
                let real_sum = digest_reader(
                    &mut options.digest,
                    &mut ckf,
                    binary_check,
                    options.output_bits,
                )
                .map_err_context(|| "failed to read input".to_string())?
                .to_ascii_lowercase();
                // FIXME: Filenames with newlines should be treated specially.
                // GNU appears to replace newlines by \n and backslashes by
                // \\ and prepend a backslash (to the hash or filename) if it did
                // this escaping.
                // Different sorts of output (checking vs outputting hashes) may
                // handle this differently. Compare carefully to GNU.
                // If you can, try to preserve invalid unicode using OsStr(ing)Ext
                // and display it using uucore::display::print_verbatim(). This is
                // easier (and more important) on Unix than on Windows.
                if sum == real_sum {
                    if !options.quiet {
                        println!("{ck_filename}: OK");
                    }
                } else {
                    if !options.status {
                        println!("{ck_filename}: FAILED");
                    }
                    failed_cksum += 1;
                }
            }
        } else {
            let sum = digest_reader(
                &mut options.digest,
                &mut file,
                options.binary,
                options.output_bits,
            )
            .map_err_context(|| "failed to read input".to_string())?;
            if options.tag {
                println!("{} ({}) = {}", options.algoname, filename.display(), sum);
            } else if options.nonames {
                println!("{sum}");
            } else if options.zero {
                print!("{} {}{}\0", sum, binary_marker, filename.display());
            } else {
                println!("{} {}{}", sum, binary_marker, filename.display());
            }
        }
    }
    if !options.status {
        match bad_format.cmp(&1) {
            Ordering::Equal => show_warning!("{} line is improperly formatted", bad_format),
            Ordering::Greater => show_warning!("{} lines are improperly formatted", bad_format),
            Ordering::Less => {}
        };
        if failed_cksum > 0 {
            show_warning!("{} computed checksum did NOT match", failed_cksum);
        }
        match failed_open_file.cmp(&1) {
            Ordering::Equal => show_warning!("{} listed file could not be read", failed_open_file),
            Ordering::Greater => {
                show_warning!("{} listed files could not be read", failed_open_file);
            }
            Ordering::Less => {}
        }
    }

    Ok(())
}

fn digest_reader<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut BufReader<T>,
    binary: bool,
    output_bits: usize,
) -> io::Result<String> {
    digest.reset();

    // Read bytes from `reader` and write those bytes to `digest`.
    //
    // If `binary` is `false` and the operating system is Windows, then
    // `DigestWriter` replaces "\r\n" with "\n" before it writes the
    // bytes into `digest`. Otherwise, it just inserts the bytes as-is.
    //
    // In order to support replacing "\r\n", we must call `finalize()`
    // in order to support the possibility that the last character read
    // from the reader was "\r". (This character gets buffered by
    // `DigestWriter` and only written if the following character is
    // "\n". But when "\r" is the last character read, we need to force
    // it to be written.)
    let mut digest_writer = DigestWriter::new(digest, binary);
    std::io::copy(reader, &mut digest_writer)?;
    digest_writer.finalize();

    if digest.output_bits() > 0 {
        Ok(digest.result_str())
    } else {
        // Assume it's SHAKE.  result_str() doesn't work with shake (as of 8/30/2016)
        let mut bytes = Vec::new();
        bytes.resize((output_bits + 7) / 8, 0);
        digest.hash_finalize(&mut bytes);
        Ok(encode(bytes))
    }
}
