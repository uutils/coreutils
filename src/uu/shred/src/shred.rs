// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) wipesync prefill

use clap::{Arg, ArgAction, Command};
#[cfg(unix)]
use libc::S_IWUSR;
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, Write};
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::parser::parse_size::parse_size_u64;
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage, show_error, show_if_err};

const ABOUT: &str = help_about!("shred.md");
const USAGE: &str = help_usage!("shred.md");
const AFTER_HELP: &str = help_section!("after help", "shred.md");

pub mod options {
    pub const FORCE: &str = "force";
    pub const FILE: &str = "file";
    pub const ITERATIONS: &str = "iterations";
    pub const SIZE: &str = "size";
    pub const WIPESYNC: &str = "u";
    pub const REMOVE: &str = "remove";
    pub const VERBOSE: &str = "verbose";
    pub const EXACT: &str = "exact";
    pub const ZERO: &str = "zero";
    pub const RANDOM_SOURCE: &str = "random-source";

    pub mod remove {
        pub const UNLINK: &str = "unlink";
        pub const WIPE: &str = "wipe";
        pub const WIPESYNC: &str = "wipesync";
    }
}

// This block size seems to match GNU (2^16 = 65536)
const BLOCK_SIZE: usize = 1 << 16;
const NAME_CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_.";

const PATTERN_LENGTH: usize = 3;
const PATTERN_BUFFER_SIZE: usize = BLOCK_SIZE + PATTERN_LENGTH - 1;

/// Optimal block size for the filesystem. This constant is used for data size alignment, similar
/// to the behavior of GNU shred. Usually, optimal block size is a 4K block (2^12), which is why
/// it's defined as a constant. However, it's possible to get the actual size at runtime using, for
/// example, `std::os::unix::fs::MetadataExt::blksize()`.
const OPTIMAL_IO_BLOCK_SIZE: usize = 1 << 12;

/// Patterns that appear in order for the passes
///
/// A single-byte pattern is equivalent to a multi-byte pattern of that byte three times.
const PATTERNS: [Pattern; 22] = [
    Pattern::Single(b'\x00'),
    Pattern::Single(b'\xFF'),
    Pattern::Single(b'\x55'),
    Pattern::Single(b'\xAA'),
    Pattern::Multi([b'\x24', b'\x92', b'\x49']),
    Pattern::Multi([b'\x49', b'\x24', b'\x92']),
    Pattern::Multi([b'\x6D', b'\xB6', b'\xDB']),
    Pattern::Multi([b'\x92', b'\x49', b'\x24']),
    Pattern::Multi([b'\xB6', b'\xDB', b'\x6D']),
    Pattern::Multi([b'\xDB', b'\x6D', b'\xB6']),
    Pattern::Single(b'\x11'),
    Pattern::Single(b'\x22'),
    Pattern::Single(b'\x33'),
    Pattern::Single(b'\x44'),
    Pattern::Single(b'\x66'),
    Pattern::Single(b'\x77'),
    Pattern::Single(b'\x88'),
    Pattern::Single(b'\x99'),
    Pattern::Single(b'\xBB'),
    Pattern::Single(b'\xCC'),
    Pattern::Single(b'\xDD'),
    Pattern::Single(b'\xEE'),
];

#[derive(Clone, Copy)]
enum Pattern {
    Single(u8),
    Multi([u8; 3]),
}

enum PassType {
    Pattern(Pattern),
    Random,
}

#[derive(PartialEq, Clone, Copy)]
enum RemoveMethod {
    None,     // Default method. Only obfuscate the file data
    Unlink,   // The same as 'None' + unlink the file
    Wipe,     // The same as 'Unlink' + obfuscate the file name before unlink
    WipeSync, // The same as 'Wipe' sync the file name changes
}

/// Iterates over all possible filenames of a certain length using NAME_CHARSET as an alphabet
struct FilenameIter {
    // Store the indices of the letters of our filename in NAME_CHARSET
    name_charset_indices: Vec<usize>,
    exhausted: bool,
}

impl FilenameIter {
    fn new(name_len: usize) -> Self {
        Self {
            name_charset_indices: vec![0; name_len],
            exhausted: false,
        }
    }
}

impl Iterator for FilenameIter {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.exhausted {
            return None;
        }

        // First, make the return value using the current state
        let ret: String = self
            .name_charset_indices
            .iter()
            .map(|i| char::from(NAME_CHARSET[*i]))
            .collect();

        // Now increment the least significant index and possibly each next
        // index if necessary.
        for index in self.name_charset_indices.iter_mut().rev() {
            if *index == NAME_CHARSET.len() - 1 {
                // Carry the 1
                *index = 0;
                continue;
            } else {
                *index += 1;
                return Some(ret);
            }
        }

        // If we get here, we flipped all bits back to 0, so we exhausted all options.
        self.exhausted = true;
        Some(ret)
    }
}

enum RandomSource {
    System,
    Read(File),
}

/// Used to generate blocks of bytes of size <= BLOCK_SIZE based on either a give pattern
/// or randomness
// The lint warns about a large difference because StdRng is big, but the buffers are much
// larger anyway, so it's fine.
#[allow(clippy::large_enum_variant)]
enum BytesWriter<'a> {
    Random {
        rng: StdRng,
        buffer: [u8; BLOCK_SIZE],
    },
    RandomFile {
        rng_file: &'a File,
        buffer: [u8; BLOCK_SIZE],
    },
    // To write patterns we only write to the buffer once. To be able to do
    // this, we need to extend the buffer with 2 bytes. We can then easily
    // obtain a buffer starting with any character of the pattern that we
    // want with an offset of either 0, 1 or 2.
    //
    // For example, if we have the pattern ABC, but we want to write a block
    // of BLOCK_SIZE starting with B, we just pick the slice [1..BLOCK_SIZE+1]
    // This means that we only have to fill the buffer once and can just reuse
    // it afterwards.
    Pattern {
        offset: usize,
        buffer: [u8; PATTERN_BUFFER_SIZE],
    },
}

impl<'a> BytesWriter<'a> {
    fn from_pass_type(pass: &PassType, random_source: &'a RandomSource) -> Self {
        match pass {
            PassType::Random => match random_source {
                RandomSource::System => Self::Random {
                    rng: StdRng::from_os_rng(),
                    buffer: [0; BLOCK_SIZE],
                },
                RandomSource::Read(file) => Self::RandomFile {
                    rng_file: file,
                    buffer: [0; BLOCK_SIZE],
                },
            },
            PassType::Pattern(pattern) => {
                // Copy the pattern in chunks rather than simply one byte at a time
                // We prefill the pattern so that the buffer can be reused at each
                // iteration as a small optimization.
                let buffer = match pattern {
                    Pattern::Single(byte) => [*byte; PATTERN_BUFFER_SIZE],
                    Pattern::Multi(bytes) => {
                        let mut buf = [0; PATTERN_BUFFER_SIZE];
                        for chunk in buf.chunks_exact_mut(PATTERN_LENGTH) {
                            chunk.copy_from_slice(bytes);
                        }
                        buf
                    }
                };
                Self::Pattern { offset: 0, buffer }
            }
        }
    }

    fn bytes_for_pass(&mut self, size: usize) -> Result<&[u8], io::Error> {
        match self {
            Self::Random { rng, buffer } => {
                let bytes = &mut buffer[..size];
                rng.fill(bytes);
                Ok(bytes)
            }
            Self::RandomFile { rng_file, buffer } => {
                let bytes = &mut buffer[..size];
                rng_file.read_exact(bytes)?;
                Ok(bytes)
            }
            Self::Pattern { offset, buffer } => {
                let bytes = &buffer[*offset..size + *offset];
                *offset = (*offset + size) % PATTERN_LENGTH;
                Ok(bytes)
            }
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    if !matches.contains_id(options::FILE) {
        return Err(UUsageError::new(1, "missing file operand"));
    }

    let iterations = match matches.get_one::<String>(options::ITERATIONS) {
        Some(s) => match s.parse::<usize>() {
            Ok(u) => u,
            Err(_) => {
                return Err(USimpleError::new(
                    1,
                    format!("invalid number of passes: {}", s.quote()),
                ));
            }
        },
        None => unreachable!(),
    };

    let random_source = match matches.get_one::<String>(options::RANDOM_SOURCE) {
        Some(filepath) => RandomSource::Read(File::open(filepath).map_err(|_| {
            USimpleError::new(
                1,
                format!("cannot open random source: {}", filepath.quote()),
            )
        })?),
        None => RandomSource::System,
    };
    // TODO: implement --random-source

    let remove_method = if matches.get_flag(options::WIPESYNC) {
        RemoveMethod::WipeSync
    } else if matches.contains_id(options::REMOVE) {
        match matches
            .get_one::<String>(options::REMOVE)
            .map(AsRef::as_ref)
        {
            Some(options::remove::UNLINK) => RemoveMethod::Unlink,
            Some(options::remove::WIPE) => RemoveMethod::Wipe,
            Some(options::remove::WIPESYNC) => RemoveMethod::WipeSync,
            _ => unreachable!("should be caught by clap"),
        }
    } else {
        RemoveMethod::None
    };

    let force = matches.get_flag(options::FORCE);
    let size_arg = matches
        .get_one::<String>(options::SIZE)
        .map(|s| s.to_string());
    let size = get_size(size_arg);
    let exact = matches.get_flag(options::EXACT) || size.is_some();
    let zero = matches.get_flag(options::ZERO);
    let verbose = matches.get_flag(options::VERBOSE);

    for path_str in matches.get_many::<String>(options::FILE).unwrap() {
        show_if_err!(wipe_file(
            path_str,
            iterations,
            remove_method,
            size,
            exact,
            zero,
            &random_source,
            verbose,
            force,
        ));
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FORCE)
                .long(options::FORCE)
                .short('f')
                .help("change permissions to allow writing if necessary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ITERATIONS)
                .long(options::ITERATIONS)
                .short('n')
                .help("overwrite N times instead of the default (3)")
                .value_name("NUMBER")
                .default_value("3"),
        )
        .arg(
            Arg::new(options::SIZE)
                .long(options::SIZE)
                .short('s')
                .value_name("N")
                .help("shred this many bytes (suffixes like K, M, G accepted)"),
        )
        .arg(
            Arg::new(options::WIPESYNC)
                .short('u')
                .help("deallocate and remove file after overwriting")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REMOVE)
                .long(options::REMOVE)
                .value_name("HOW")
                .value_parser(ShortcutValueParser::new([
                    options::remove::UNLINK,
                    options::remove::WIPE,
                    options::remove::WIPESYNC,
                ]))
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value(options::remove::WIPESYNC)
                .help("like -u but give control on HOW to delete;  See below")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long(options::VERBOSE)
                .short('v')
                .help("show progress")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::EXACT)
                .long(options::EXACT)
                .short('x')
                .help(
                    "do not round file sizes up to the next full block;\n\
                     this is the default for non-regular files",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help("add a final overwrite with zeros to hide shredding")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RANDOM_SOURCE)
                .long(options::RANDOM_SOURCE)
                .help("take random bytes from FILE")
                .value_hint(clap::ValueHint::FilePath)
                .action(ArgAction::Set),
        )
        // Positional arguments
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn get_size(size_str_opt: Option<String>) -> Option<u64> {
    size_str_opt
        .as_ref()
        .and_then(|size| parse_size_u64(size.as_str()).ok())
        .or_else(|| {
            if let Some(size) = size_str_opt {
                show_error!("invalid file size: {}", size.quote());
                // TODO: replace with our error management
                std::process::exit(1);
            }
            None
        })
}

fn pass_name(pass_type: &PassType) -> String {
    match pass_type {
        PassType::Random => String::from("random"),
        PassType::Pattern(Pattern::Single(byte)) => format!("{byte:02x}{byte:02x}{byte:02x}"),
        PassType::Pattern(Pattern::Multi([a, b, c])) => format!("{a:02x}{b:02x}{c:02x}"),
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::cognitive_complexity)]
fn wipe_file(
    path_str: &str,
    n_passes: usize,
    remove_method: RemoveMethod,
    size: Option<u64>,
    exact: bool,
    zero: bool,
    random_source: &RandomSource,
    verbose: bool,
    force: bool,
) -> UResult<()> {
    // Get these potential errors out of the way first
    let path = Path::new(path_str);
    if !path.exists() {
        return Err(USimpleError::new(
            1,
            format!("{}: No such file or directory", path.maybe_quote()),
        ));
    }
    if !path.is_file() {
        return Err(USimpleError::new(
            1,
            format!("{}: Not a file", path.maybe_quote()),
        ));
    }

    let metadata = fs::metadata(path).map_err_context(String::new)?;

    // If force is true, set file permissions to not-readonly.
    if force {
        let mut perms = metadata.permissions();
        #[cfg(unix)]
        #[allow(clippy::useless_conversion, clippy::unnecessary_cast)]
        {
            // NOTE: set_readonly(false) makes the file world-writable on Unix.
            // NOTE: S_IWUSR type is u16 on macOS, i32 on Redox.
            if (perms.mode() & (S_IWUSR as u32)) == 0 {
                perms.set_mode(S_IWUSR as u32);
            }
        }
        #[cfg(not(unix))]
        // TODO: Remove the following once https://github.com/rust-lang/rust-clippy/issues/10477 is resolved.
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(path, perms).map_err_context(String::new)?;
    }

    // Fill up our pass sequence
    let mut pass_sequence = Vec::new();
    if metadata.len() != 0 {
        // Only add passes if the file is non-empty

        if n_passes <= 3 {
            // Only random passes if n_passes <= 3
            for _ in 0..n_passes {
                pass_sequence.push(PassType::Random);
            }
        } else {
            // Add initial random to avoid O(n) operation later
            pass_sequence.push(PassType::Random);
            let n_random = (n_passes / 10).max(3); // Minimum 3 random passes; ratio of 10 after
            let n_fixed = n_passes - n_random;
            // Fill it with Patterns and all but the first and last random, then shuffle it
            let n_full_arrays = n_fixed / PATTERNS.len(); // How many times can we go through all the patterns?
            let remainder = n_fixed % PATTERNS.len(); // How many do we get through on our last time through, excluding randoms?

            for _ in 0..n_full_arrays {
                for p in PATTERNS {
                    pass_sequence.push(PassType::Pattern(p));
                }
            }
            for pattern in PATTERNS.into_iter().take(remainder) {
                pass_sequence.push(PassType::Pattern(pattern));
            }
            // add random passes except one each at the beginning and end
            for _ in 0..n_random - 2 {
                pass_sequence.push(PassType::Random);
            }

            let mut rng = rand::rng();
            pass_sequence[1..].shuffle(&mut rng); // randomize the order of application
            pass_sequence.push(PassType::Random); // add the last random pass
        }

        // --zero specifies whether we want one final pass of 0x00 on our file
        if zero {
            pass_sequence.push(PassType::Pattern(PATTERNS[0]));
        }
    }

    let total_passes = pass_sequence.len();
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(false)
        .open(path)
        .map_err_context(|| format!("{}: failed to open for writing", path.maybe_quote()))?;

    let size = match size {
        Some(size) => size,
        None => metadata.len(),
    };

    for (i, pass_type) in pass_sequence.into_iter().enumerate() {
        if verbose {
            let pass_name = pass_name(&pass_type);
            show_error!(
                "{}: pass {}/{total_passes} ({pass_name})...",
                path.maybe_quote(),
                i + 1,
            );
        }
        // size is an optional argument for exactly how many bytes we want to shred
        // Ignore failed writes; just keep trying
        show_if_err!(
            do_pass(&mut file, &pass_type, exact, random_source, size)
                .map_err_context(|| format!("{}: File write pass failed", path.maybe_quote()))
        );
    }

    if remove_method != RemoveMethod::None {
        do_remove(path, path_str, verbose, remove_method)
            .map_err_context(|| format!("{}: failed to remove file", path.maybe_quote()))?;
    }
    Ok(())
}

fn split_on_blocks(file_size: u64, exact: bool) -> (u64, u64) {
    // OPTIMAL_IO_BLOCK_SIZE must not exceed BLOCK_SIZE. Violating this may cause overflows due
    // to alignment or performance issues.This kind of misconfiguration is
    // highly unlikely but would indicate a serious error.
    const _: () = assert!(OPTIMAL_IO_BLOCK_SIZE <= BLOCK_SIZE);

    let file_size = if exact {
        file_size
    } else {
        // The main idea here is to align the file size to the OPTIMAL_IO_BLOCK_SIZE, and then
        // split it into BLOCK_SIZE + remaining bytes. Since the input data is already aligned to N
        // * OPTIMAL_IO_BLOCK_SIZE, the output file size will also be aligned and correct.
        file_size.div_ceil(OPTIMAL_IO_BLOCK_SIZE as u64) * OPTIMAL_IO_BLOCK_SIZE as u64
    };
    (file_size / BLOCK_SIZE as u64, file_size % BLOCK_SIZE as u64)
}

fn do_pass(
    file: &mut File,
    pass_type: &PassType,
    exact: bool,
    random_source: &RandomSource,
    file_size: u64,
) -> Result<(), io::Error> {
    // We might be at the end of the file due to a previous iteration, so rewind.
    file.rewind()?;

    let mut writer = BytesWriter::from_pass_type(pass_type, random_source);
    let (number_of_blocks, bytes_left) = split_on_blocks(file_size, exact);

    // We start by writing BLOCK_SIZE times as many time as possible.
    for _ in 0..number_of_blocks {
        let block = writer.bytes_for_pass(BLOCK_SIZE)?;
        file.write_all(block)?;
    }

    // Then we write remaining data which is smaller than the BLOCK_SIZE
    let block = writer.bytes_for_pass(bytes_left as usize)?;
    file.write_all(block)?;

    file.sync_data()?;

    Ok(())
}

// Repeatedly renames the file with strings of decreasing length (most likely all 0s)
// Return the path of the file after its last renaming or None if error
fn wipe_name(orig_path: &Path, verbose: bool, remove_method: RemoveMethod) -> Option<PathBuf> {
    let file_name_len = orig_path.file_name().unwrap().to_str().unwrap().len();

    let mut last_path = PathBuf::from(orig_path);

    for length in (1..=file_name_len).rev() {
        // Try all filenames of a given length.
        // If every possible filename already exists, just reduce the length and try again
        for name in FilenameIter::new(length) {
            let new_path = orig_path.with_file_name(name);
            // We don't want the filename to already exist (don't overwrite)
            // If it does, find another name that doesn't
            if new_path.exists() {
                continue;
            }
            match fs::rename(&last_path, &new_path) {
                Ok(()) => {
                    if verbose {
                        show_error!(
                            "{}: renamed to {}",
                            last_path.maybe_quote(),
                            new_path.display()
                        );
                    }

                    if remove_method == RemoveMethod::WipeSync {
                        // Sync every file rename
                        let new_file = OpenOptions::new()
                            .write(true)
                            .open(new_path.clone())
                            .expect("Failed to open renamed file for syncing");
                        new_file.sync_all().expect("Failed to sync renamed file");
                    }

                    last_path = new_path;
                    break;
                }
                Err(e) => {
                    show_error!(
                        "{}: Couldn't rename to {}: {e}",
                        last_path.maybe_quote(),
                        new_path.quote(),
                    );
                    // TODO: replace with our error management
                    std::process::exit(1);
                }
            }
        }
    }

    Some(last_path)
}

fn do_remove(
    path: &Path,
    orig_filename: &str,
    verbose: bool,
    remove_method: RemoveMethod,
) -> Result<(), io::Error> {
    if verbose {
        show_error!("{}: removing", orig_filename.maybe_quote());
    }

    let remove_path = if remove_method == RemoveMethod::Unlink {
        Some(path.with_file_name(orig_filename))
    } else {
        wipe_name(path, verbose, remove_method)
    };

    if let Some(rp) = remove_path {
        fs::remove_file(rp)?;
    }

    if verbose {
        show_error!("{}: removed", orig_filename.maybe_quote());
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::{BLOCK_SIZE, OPTIMAL_IO_BLOCK_SIZE, split_on_blocks};

    #[test]
    fn test_align_non_exact_control_values() {
        // Note: This test only makes sense for the default values of BLOCK_SIZE and
        // OPTIMAL_IO_BLOCK_SIZE.
        assert_eq!(split_on_blocks(1, false), (0, 4096));
        assert_eq!(split_on_blocks(4095, false), (0, 4096));
        assert_eq!(split_on_blocks(4096, false), (0, 4096));
        assert_eq!(split_on_blocks(4097, false), (0, 8192));
        assert_eq!(split_on_blocks(65535, false), (1, 0));
        assert_eq!(split_on_blocks(65536, false), (1, 0));
        assert_eq!(split_on_blocks(65537, false), (1, 4096));
    }

    #[test]
    fn test_align_non_exact_cycle() {
        for size in 1..BLOCK_SIZE as u64 * 2 {
            let (number_of_blocks, bytes_left) = split_on_blocks(size, false);
            let test_size = number_of_blocks * BLOCK_SIZE as u64 + bytes_left;
            assert_eq!(test_size % OPTIMAL_IO_BLOCK_SIZE as u64, 0);
        }
    }

    #[test]
    fn test_align_exact_cycle() {
        for size in 1..BLOCK_SIZE as u64 * 2 {
            let (number_of_blocks, bytes_left) = split_on_blocks(size, true);
            let test_size = number_of_blocks * BLOCK_SIZE as u64 + bytes_left;
            assert_eq!(test_size, size);
        }
    }
}
