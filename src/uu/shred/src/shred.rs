// * This file is part of the uutils coreutils package.
// *
// * (c) Michael Rosenberg <42micro@gmail.com>
// * (c) Fort <forticulous@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE
// * file that was distributed with this source code.

// spell-checker:ignore (words) wipesync

use clap::{crate_version, Arg, ArgAction, Command};
use rand::prelude::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::cell::{Cell, RefCell};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::prelude::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::libc::S_IWUSR;
use uucore::{format_usage, help_about, help_section, help_usage, show, show_if_err, util_name};

// This block size seems to match GNU (2^16 = 65536)
const BLOCK_SIZE: usize = 1 << 16;
const NAME_CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_.";

const PATTERN_LENGTH: usize = 3;
const PATTERN_BUFFER_SIZE: usize = BLOCK_SIZE + PATTERN_LENGTH - 1;

/// Patterns that appear in order for the passes
///
/// They are all extended to 3 bytes for consistency, even though some could be
/// expressed as single bytes.
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

// Used to generate all possible filenames of a certain length using NAME_CHARSET as an alphabet
struct FilenameGenerator {
    name_len: usize,
    name_charset_indices: RefCell<Vec<usize>>, // Store the indices of the letters of our filename in NAME_CHARSET
    exhausted: Cell<bool>,
}

impl FilenameGenerator {
    fn new(name_len: usize) -> Self {
        let indices: Vec<usize> = vec![0; name_len];
        Self {
            name_len,
            name_charset_indices: RefCell::new(indices),
            exhausted: Cell::new(false),
        }
    }
}

impl Iterator for FilenameGenerator {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.exhausted.get() {
            return None;
        }

        let mut name_charset_indices = self.name_charset_indices.borrow_mut();

        // Make the return value, then increment
        let mut ret = String::new();
        for i in name_charset_indices.iter() {
            let c = char::from(NAME_CHARSET[*i]);
            ret.push(c);
        }

        if name_charset_indices[0] == NAME_CHARSET.len() - 1 {
            self.exhausted.set(true);
        }
        // Now increment the least significant index
        for i in (0..self.name_len).rev() {
            if name_charset_indices[i] == NAME_CHARSET.len() - 1 {
                name_charset_indices[i] = 0; // Carry the 1
                continue;
            } else {
                name_charset_indices[i] += 1;
                break;
            }
        }

        Some(ret)
    }
}

// Used to generate blocks of bytes of size <= BLOCK_SIZE based on either a give pattern
// or randomness
enum BytesWriter {
    Random {
        rng: StdRng,
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

impl BytesWriter {
    fn from_pass_type(pass: PassType) -> Self {
        match pass {
            PassType::Random => Self::Random {
                rng: StdRng::from_entropy(),
                buffer: [0; BLOCK_SIZE],
            },
            PassType::Pattern(pattern) => {
                // Copy the pattern in chunks rather than simply one byte at a time
                // We prefill the pattern so that the buffer can be reused at each
                // iteration as a small optimization.
                let buffer = match pattern {
                    Pattern::Single(byte) => [byte; PATTERN_BUFFER_SIZE],
                    Pattern::Multi(bytes) => {
                        let mut buf = [0; PATTERN_BUFFER_SIZE];
                        for chunk in buf.chunks_exact_mut(PATTERN_LENGTH) {
                            chunk.copy_from_slice(&bytes);
                        }
                        buf
                    }
                };
                Self::Pattern { offset: 0, buffer }
            }
        }
    }

    fn bytes_for_pass(&mut self, size: usize) -> &[u8] {
        match self {
            Self::Random { rng, buffer } => {
                let bytes = &mut buffer[..size];
                rng.fill(bytes);
                bytes
            }
            Self::Pattern { offset, buffer } => {
                let bytes = &buffer[*offset..size + *offset];
                *offset = (*offset + size) % PATTERN_LENGTH;
                bytes
            }
        }
    }
}

const ABOUT: &str = help_about!("shred.md");
const USAGE: &str = help_usage!("shred.md");
const AFTER_HELP: &str = help_section!("after help", "shred.md");

pub mod options {
    pub const FORCE: &str = "force";
    pub const FILE: &str = "file";
    pub const ITERATIONS: &str = "iterations";
    pub const SIZE: &str = "size";
    pub const REMOVE: &str = "remove";
    pub const VERBOSE: &str = "verbose";
    pub const EXACT: &str = "exact";
    pub const ZERO: &str = "zero";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

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
                ))
            }
        },
        None => unreachable!(),
    };

    // TODO: implement --remove HOW
    //       The optional HOW parameter indicates how to remove a directory entry:
    //         - 'unlink' => use a standard unlink call.
    //         - 'wipe' => also first obfuscate bytes in the name.
    //         - 'wipesync' => also sync each obfuscated byte to disk.
    //       The default mode is 'wipesync', but note it can be expensive.

    // TODO: implement --random-source

    let force = matches.get_flag(options::FORCE);
    let remove = matches.get_flag(options::REMOVE);
    let size_arg = matches
        .get_one::<String>(options::SIZE)
        .map(|s| s.to_string());
    let size = get_size(size_arg);
    let exact = matches.get_flag(options::EXACT) && size.is_none(); // if -s is given, ignore -x
    let zero = matches.get_flag(options::ZERO);
    let verbose = matches.get_flag(options::VERBOSE);

    for path_str in matches.get_many::<String>(options::FILE).unwrap() {
        show_if_err!(wipe_file(
            path_str, iterations, remove, size, exact, zero, verbose, force,
        ));
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
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
            Arg::new(options::REMOVE)
                .short('u')
                .long(options::REMOVE)
                .help("truncate and remove file after overwriting;  See below")
                .action(ArgAction::SetTrue),
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
        // Positional arguments
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

// TODO: Add support for all postfixes here up to and including EiB
//       http://www.gnu.org/software/coreutils/manual/coreutils.html#Block-size
fn get_size(size_str_opt: Option<String>) -> Option<u64> {
    size_str_opt.as_ref()?;

    let mut size_str = size_str_opt.as_ref().unwrap().clone();
    // Immutably look at last character of size string
    let unit = match size_str.chars().last().unwrap() {
        'K' => {
            size_str.pop();
            1024u64
        }
        'M' => {
            size_str.pop();
            (1024 * 1024) as u64
        }
        'G' => {
            size_str.pop();
            (1024 * 1024 * 1024) as u64
        }
        _ => 1u64,
    };

    let coefficient = match size_str.parse::<u64>() {
        Ok(u) => u,
        Err(_) => {
            println!(
                "{}: {}: Invalid file size",
                util_name(),
                size_str_opt.unwrap().maybe_quote()
            );
            std::process::exit(1);
        }
    };

    Some(coefficient * unit)
}

fn pass_name(pass_type: &PassType) -> String {
    match pass_type {
        PassType::Random => String::from("random"),
        PassType::Pattern(Pattern::Single(byte)) => format!("{byte:x}{byte:x}{byte:x}"),
        PassType::Pattern(Pattern::Multi([a, b, c])) => format!("{a:x}{b:x}{c:x}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn wipe_file(
    path_str: &str,
    n_passes: usize,
    remove: bool,
    size: Option<u64>,
    exact: bool,
    zero: bool,
    verbose: bool,
    force: bool,
) -> UResult<()> {
    // Get these potential errors out of the way first
    let path: &Path = Path::new(path_str);
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

    // If force is true, set file permissions to not-readonly.
    if force {
        let metadata = fs::metadata(path).map_err_context(String::new)?;
        let mut perms = metadata.permissions();
        #[cfg(unix)]
        #[allow(clippy::useless_conversion)]
        {
            // NOTE: set_readonly(false) makes the file world-writable on Unix.
            // NOTE: S_IWUSR type is u16 on macOS.
            if (perms.mode() & u32::from(S_IWUSR)) == 0 {
                perms.set_mode(u32::from(S_IWUSR));
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

    if n_passes <= 3 {
        // Only random passes if n_passes <= 3
        for _ in 0..n_passes {
            pass_sequence.push(PassType::Random);
        }
    } else {
        // First fill it with Patterns, shuffle it, then evenly distribute Random
        let n_full_arrays = n_passes / PATTERNS.len(); // How many times can we go through all the patterns?
        let remainder = n_passes % PATTERNS.len(); // How many do we get through on our last time through?

        for _ in 0..n_full_arrays {
            for p in PATTERNS {
                pass_sequence.push(PassType::Pattern(p));
            }
        }
        for pattern in PATTERNS.into_iter().take(remainder) {
            pass_sequence.push(PassType::Pattern(pattern));
        }
        let mut rng = rand::thread_rng();
        pass_sequence.shuffle(&mut rng); // randomize the order of application

        let n_random = 3 + n_passes / 10; // Minimum 3 random passes; ratio of 10 after
                                          // Evenly space random passes; ensures one at the beginning and end
        for i in 0..n_random {
            pass_sequence[i * (n_passes - 1) / (n_random - 1)] = PassType::Random;
        }
    }

    // --zero specifies whether we want one final pass of 0x00 on our file
    if zero {
        pass_sequence.push(PassType::Pattern(PATTERNS[0]));
    }

    let total_passes: usize = pass_sequence.len();
    let mut file: File = OpenOptions::new()
        .write(true)
        .truncate(false)
        .open(path)
        .map_err_context(|| format!("{}: failed to open for writing", path.maybe_quote()))?;

    let size = match size {
        Some(size) => size,
        None => get_file_size(path)?,
    };

    for (i, pass_type) in pass_sequence.into_iter().enumerate() {
        if verbose {
            let pass_name: String = pass_name(&pass_type);
            if total_passes.to_string().len() == 1 {
                println!(
                    "{}: {}: pass {}/{} ({})... ",
                    util_name(),
                    path.maybe_quote(),
                    i + 1,
                    total_passes,
                    pass_name
                );
            } else {
                println!(
                    "{}: {}: pass {:2.0}/{:2.0} ({})... ",
                    util_name(),
                    path.maybe_quote(),
                    i + 1,
                    total_passes,
                    pass_name
                );
            }
        }
        // size is an optional argument for exactly how many bytes we want to shred
        show_if_err!(do_pass(&mut file, pass_type, exact, size)
            .map_err_context(|| format!("{}: File write pass failed", path.maybe_quote())));
        // Ignore failed writes; just keep trying
    }

    if remove {
        do_remove(path, path_str, verbose)
            .map_err_context(|| format!("{}: failed to remove file", path.maybe_quote()))?;
    }
    Ok(())
}

fn do_pass(
    file: &mut File,
    pass_type: PassType,
    exact: bool,
    file_size: u64,
) -> Result<(), io::Error> {
    // We might be at the end of the file due to a previous iteration, so rewind.
    file.rewind()?;

    let mut writer = BytesWriter::from_pass_type(pass_type);

    // We start by writing BLOCK_SIZE times as many time as possible.
    for _ in 0..(file_size / BLOCK_SIZE as u64) {
        let block = writer.bytes_for_pass(BLOCK_SIZE);
        file.write_all(block)?;
    }

    // Now we might have some bytes left, so we write either that
    // many bytes if exact is true, or BLOCK_SIZE bytes if not.
    let bytes_left = (file_size % BLOCK_SIZE as u64) as usize;
    if bytes_left > 0 {
        let size = if exact { bytes_left } else { BLOCK_SIZE };
        let block = writer.bytes_for_pass(size);
        file.write_all(block)?;
    }

    file.sync_data()?;

    Ok(())
}

fn get_file_size(path: &Path) -> Result<u64, io::Error> {
    Ok(fs::metadata(path)?.len())
}

// Repeatedly renames the file with strings of decreasing length (most likely all 0s)
// Return the path of the file after its last renaming or None if error
fn wipe_name(orig_path: &Path, verbose: bool) -> Option<PathBuf> {
    let file_name_len: usize = orig_path.file_name().unwrap().to_str().unwrap().len();

    let mut last_path: PathBuf = PathBuf::from(orig_path);

    for length in (1..=file_name_len).rev() {
        for name in FilenameGenerator::new(length) {
            let new_path: PathBuf = orig_path.with_file_name(name);
            // We don't want the filename to already exist (don't overwrite)
            // If it does, find another name that doesn't
            if new_path.exists() {
                continue;
            }
            match fs::rename(&last_path, &new_path) {
                Ok(()) => {
                    if verbose {
                        println!(
                            "{}: {}: renamed to {}",
                            util_name(),
                            last_path.maybe_quote(),
                            new_path.quote()
                        );
                    }

                    // Sync every file rename
                    {
                        let new_file: File = File::open(new_path.clone())
                            .expect("Failed to open renamed file for syncing");
                        new_file.sync_all().expect("Failed to sync renamed file");
                    }

                    last_path = new_path;
                    break;
                }
                Err(e) => {
                    println!(
                        "{}: {}: Couldn't rename to {}: {}",
                        util_name(),
                        last_path.maybe_quote(),
                        new_path.quote(),
                        e
                    );
                    return None;
                }
            }
        } // If every possible filename already exists, just reduce the length and try again
    }

    Some(last_path)
}

fn do_remove(path: &Path, orig_filename: &str, verbose: bool) -> Result<(), io::Error> {
    if verbose {
        println!("{}: {}: removing", util_name(), orig_filename.maybe_quote());
    }

    let renamed_path: Option<PathBuf> = wipe_name(path, verbose);
    if let Some(rp) = renamed_path {
        fs::remove_file(rp)?;
    }

    if verbose {
        println!("{}: {}: removed", util_name(), orig_filename.maybe_quote());
    }

    Ok(())
}
