// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) wipesync prefill couldnt fillpattern genpattern genmax

use clap::{Arg, ArgAction, Command};
#[cfg(unix)]
use libc::S_IWUSR;
use rand::{RngExt as _, rngs::StdRng};
use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, Write};
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::parser::parse_size::parse_size_u64;
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::translate;
use uucore::{format_usage, show_error, show_if_err};

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
/// Sector size used when applying the "flip first bit of every sector" variants.
const SECTOR_SIZE: usize = 512;

/// Optimal block size for the filesystem. This constant is used for data size alignment, similar
/// to the behavior of GNU shred. Usually, optimal block size is a 4K block (2^12), which is why
/// it's defined as a constant. However, it's possible to get the actual size at runtime using, for
/// example, `std::os::unix::fs::MetadataExt::blksize()`.
const OPTIMAL_IO_BLOCK_SIZE: usize = 1 << 12;

/// Zero-fill pattern used for the optional final `--zero` pass.
const ZERO_PATTERN: Pattern = Pattern {
    bytes: [0, 0, 0],
    flip_sector: false,
};

/// Pass-group descriptor table for overwrite scheduling.
///
/// Layout (public Gutmann / shred design):
/// - `k > 0`: the next `k` entries are fixed pattern codes to include as a group
/// - `k < 0`: schedule `-k` random passes
/// - `k == 0`: end of table (restart from the beginning when more passes remain)
///
/// Pattern codes use the lower 12 bits as the repeating 3-byte bit pattern. Bit
/// `0x1000` marks the sector-phase variant: the first byte of every 512-byte
/// sector is XOR'd with `0x80` when the pass is written.
const PASS_GROUPS: &[i32] = &[
    -2, // 2 random passes
    2, 0x000, 0xFFF, // 1-bit
    2, 0x555, 0xAAA, // 2-bit
    -1,    // 1 random pass
    6, 0x249, 0x492, 0x6DB, 0x924, 0xB6D, 0xDB6, // 3-bit
    12, 0x111, 0x222, 0x333, 0x444, 0x666, 0x777, 0x888, 0x999, 0xBBB, 0xCCC, 0xDDD,
    0xEEE, // 4-bit
    -1,    // 1 random pass
    // First bit of each 512-byte sector flipped (phase variants)
    8, 0x1000, 0x1249, 0x1492, 0x16DB, 0x1924, 0x1B6D, 0x1DB6, 0x1FFF, 14, 0x1111, 0x1222, 0x1333,
    0x1444, 0x1555, 0x1666, 0x1777, 0x1888, 0x1999, 0x1AAA, 0x1BBB, 0x1CCC, 0x1DDD, 0x1EEE,
    -1, // 1 random pass
    0,  // end
];

/// Fixed overwrite pattern: three repeating bytes, optionally with the per-sector
/// first-byte flip used by phase-variant pattern codes (bit `0x1000`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pattern {
    bytes: [u8; 3],
    flip_sector: bool,
}

impl Pattern {
    /// Decode a pattern code into base bytes and optional sector flip.
    fn from_code(code: i32) -> Self {
        let mut bits = (code & 0xfff) as u32;
        bits |= bits << 12;
        let b0 = ((bits >> 4) & 255) as u8;
        let b1 = ((bits >> 8) & 255) as u8;
        let b2 = (bits & 255) as u8;
        Self {
            bytes: [b0, b1, b2],
            flip_sector: (code & 0x1000) != 0,
        }
    }

    /// Bytes shown in verbose pass names (after applying the sector flip to byte 0).
    fn display_bytes(self) -> [u8; 3] {
        let mut b = self.bytes;
        if self.flip_sector {
            b[0] ^= 0x80;
        }
        b
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Iterates over all possible filenames of a certain length using [`NAME_CHARSET`] as an alphabet
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

/// Used to generate blocks of bytes of size <= [`BLOCK_SIZE`] based on either a give pattern
/// or randomness
// The lint warns about a large difference because StdRng is big, but the buffers are much
// larger anyway, so it's fine.
#[allow(clippy::large_enum_variant)]
enum BytesWriter {
    Random {
        rng: StdRng,
        buffer: [u8; BLOCK_SIZE],
    },
    RandomFile {
        rng_file: File,
        buffer: [u8; BLOCK_SIZE],
    },
    // To write patterns, we only write to the buffer once. To be able to do
    // this, we need to extend the buffer with 2 bytes. We can then easily
    // obtain a buffer starting with any character of the pattern that we
    // want with an offset of either 0, 1 or 2.
    //
    // For example, if we have the pattern ABC, but we want to write a block
    // of BLOCK_SIZE starting with B, we just pick the slice [1..BLOCK_SIZE+1]
    // This means that we only have to fill the buffer once and can just reuse
    // it afterward.
    Pattern {
        offset: usize,
        buffer: [u8; PATTERN_BUFFER_SIZE],
        /// When true, keep `offset` at 0 so sector-phase flips stay aligned
        /// to the start of each write (matches the sector-relative design).
        lock_offset: bool,
    },
}

impl BytesWriter {
    fn from_pass_type(
        pass: &PassType,
        random_source: Option<&RefCell<File>>,
    ) -> Result<Self, io::Error> {
        match pass {
            PassType::Random => match random_source {
                None => Ok(Self::Random {
                    rng: rand::make_rng(),
                    buffer: [0; BLOCK_SIZE],
                }),
                Some(file_cell) => {
                    // We need to create a new file handle that shares the position
                    // For now, we'll duplicate the file descriptor to maintain position
                    let new_file = file_cell.borrow_mut().try_clone()?;
                    Ok(Self::RandomFile {
                        rng_file: new_file,
                        buffer: [0; BLOCK_SIZE],
                    })
                }
            },
            PassType::Pattern(pattern) => {
                // Prefill the pattern so the buffer can be reused each iteration.
                let mut buffer = [0_u8; PATTERN_BUFFER_SIZE];
                if pattern.bytes[0] == pattern.bytes[1] && pattern.bytes[1] == pattern.bytes[2] {
                    buffer.fill(pattern.bytes[0]);
                } else {
                    for chunk in buffer.chunks_exact_mut(PATTERN_LENGTH) {
                        chunk.copy_from_slice(&pattern.bytes);
                    }
                    let filled = PATTERN_BUFFER_SIZE - PATTERN_BUFFER_SIZE % PATTERN_LENGTH;
                    buffer[filled..]
                        .copy_from_slice(&pattern.bytes[..PATTERN_BUFFER_SIZE - filled]);
                }
                // Phase-variant patterns: invert the first bit of every 512-byte sector.
                if pattern.flip_sector {
                    let mut i = 0;
                    while i < PATTERN_BUFFER_SIZE {
                        buffer[i] ^= 0x80;
                        i += SECTOR_SIZE;
                    }
                }
                Ok(Self::Pattern {
                    offset: 0,
                    buffer,
                    lock_offset: pattern.flip_sector,
                })
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
            Self::Pattern {
                offset,
                buffer,
                lock_offset,
            } => {
                let bytes = &buffer[*offset..size + *offset];
                if !*lock_offset {
                    *offset = (*offset + size) % PATTERN_LENGTH;
                }
                Ok(bytes)
            }
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    if !matches.contains_id(options::FILE) {
        return Err(UUsageError::new(
            1,
            translate!("shred-missing-file-operand"),
        ));
    }

    let iterations = {
        let s = matches.get_one::<String>(options::ITERATIONS).unwrap(); // safe to unwrap, has default value
        s.parse::<usize>().map_err(|_| {
            USimpleError::new(
                1,
                translate!("shred-invalid-number-of-passes", "passes" => s.quote()),
            )
        })?
    };

    let random_source = match matches.get_one::<String>(options::RANDOM_SOURCE) {
        Some(filepath) => Some(RefCell::new(
            File::open(filepath).map_err_context(|| filepath.clone())?,
        )),
        None => None,
    };

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
        .map(ToOwned::to_owned);
    let size = get_size(size_arg);
    let exact = matches.get_flag(options::EXACT) || size.is_some();
    let zero = matches.get_flag(options::ZERO);
    let verbose = matches.get_flag(options::VERBOSE);

    for path_str in matches.get_many::<OsString>(options::FILE).unwrap() {
        show_if_err!(wipe_file(
            path_str,
            iterations,
            remove_method,
            size,
            exact,
            zero,
            random_source.as_ref(),
            verbose,
            force,
        ));
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("shred")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("shred"))
        .about(translate!("shred-about"))
        .after_help(translate!("shred-after-help"))
        .override_usage(format_usage(&translate!("shred-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FORCE)
                .long(options::FORCE)
                .short('f')
                .help(translate!("shred-force-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ITERATIONS)
                .long(options::ITERATIONS)
                .short('n')
                .help(translate!("shred-iterations-help"))
                .value_name("NUMBER")
                .default_value("3"),
        )
        .arg(
            Arg::new(options::SIZE)
                .long(options::SIZE)
                .short('s')
                .value_name("N")
                .help(translate!("shred-size-help")),
        )
        .arg(
            Arg::new(options::WIPESYNC)
                .short('u')
                .help(translate!("shred-deallocate-help"))
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
                .help(translate!("shred-remove-help"))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long(options::VERBOSE)
                .short('v')
                .help(translate!("shred-verbose-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::EXACT)
                .long(options::EXACT)
                .short('x')
                .help(translate!("shred-exact-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help(translate!("shred-zero-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RANDOM_SOURCE)
                .long(options::RANDOM_SOURCE)
                .help(translate!("shred-random-source-help"))
                .value_hint(clap::ValueHint::FilePath)
                .action(ArgAction::Set),
        )
        // Positional arguments
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
}

fn get_size(size_str_opt: Option<String>) -> Option<u64> {
    size_str_opt
        .as_ref()
        .and_then(|size| parse_size_u64(size.as_str()).ok())
        .or_else(|| {
            if let Some(size) = size_str_opt {
                show_error!(
                    "{}",
                    translate!("shred-invalid-file-size", "size" => size.quote())
                );
                // TODO: replace with our error management
                std::process::exit(1);
            }
            None
        })
}

fn pass_name(pass_type: &PassType) -> String {
    match pass_type {
        PassType::Random => String::from("random"),
        PassType::Pattern(pattern) => {
            let [a, b, c] = pattern.display_bytes();
            format!("{a:02x}{b:02x}{c:02x}")
        }
    }
}

/// Source of unbiased integers in `0..choices` used while scheduling passes.
trait PassRng {
    fn choose(&mut self, choices: u64) -> Result<u64, io::Error>;
}

impl PassRng for StdRng {
    fn choose(&mut self, choices: u64) -> Result<u64, io::Error> {
        debug_assert!(choices > 0);
        Ok(self.random_range(0..choices))
    }
}

/// Integer chooser that draws bytes from a file, matching the residual-entropy
/// scheme used by GNU coreutils' `randint_genmax` so `--random-source` produces
/// the same pass order as GNU shred for a given stream of bytes.
///
/// Clean-room residual behavior from the public randint design / observed
/// gshred output; not a GPL source paste.
struct FilePassRng<'a> {
    source: &'a RefCell<File>,
    randnum: u64,
    randmax: u64,
}

impl<'a> FilePassRng<'a> {
    fn new(source: &'a RefCell<File>) -> Self {
        Self {
            source,
            randnum: 0,
            randmax: 0,
        }
    }

    /// Return a uniform value in `0..=genmax`, consuming bytes from the source
    /// and retaining unused entropy for later calls.
    fn genmax(&mut self, genmax: u64) -> Result<u64, io::Error> {
        let mut randnum = self.randnum;
        let mut randmax = self.randmax;
        let choices = genmax + 1;

        loop {
            if randmax < genmax {
                // Count how many input bytes make randmax >= genmax.
                let mut rmax = randmax;
                let mut nbytes = 0_usize;
                loop {
                    rmax = (rmax << 8) | u64::from(u8::MAX);
                    nbytes += 1;
                    if rmax >= genmax {
                        break;
                    }
                }

                let mut buf = [0_u8; 8];
                // nbytes is at most 8 for any genmax that fits in u64.
                debug_assert!(nbytes <= 8);
                self.source.borrow_mut().read_exact(&mut buf[..nbytes])?;

                for &byte in &buf[..nbytes] {
                    randnum = (randnum << 8) | u64::from(byte);
                    randmax = (randmax << 8) | u64::from(u8::MAX);
                }
            }

            if randmax == genmax {
                self.randnum = 0;
                self.randmax = 0;
                return Ok(randnum);
            }

            // Reject samples outside an integral number of `choices` buckets;
            // keep the residual range for the next attempt.
            let excess_choices = randmax - genmax;
            let unusable_choices = excess_choices % choices;
            let last_usable_choice = randmax - unusable_choices;
            let reduced_randnum = randnum % choices;

            if randnum <= last_usable_choice {
                self.randnum = randnum / choices;
                self.randmax = excess_choices / choices;
                return Ok(reduced_randnum);
            }

            randnum = reduced_randnum;
            randmax = unusable_choices - 1;
        }
    }
}

impl PassRng for FilePassRng<'_> {
    fn choose(&mut self, choices: u64) -> Result<u64, io::Error> {
        debug_assert!(choices > 0);
        self.genmax(choices - 1)
    }
}

/// Schedule `num` overwrite passes: select pattern groups, then interleave random
/// passes with a Bresenham-style spacing and shuffle the fixed patterns.
///
/// Clean-room reimplementation of the publicly documented shred pass scheduling
/// design (Gutmann pattern groups + evenly spaced random passes), not a
/// translation of GNU coreutils source.
fn genpattern(num: usize, rng: &mut impl PassRng) -> Result<Vec<PassType>, io::Error> {
    if num == 0 {
        return Ok(Vec::new());
    }

    // dest holds fixed pattern codes in [0, top); random slots are filled in stage 2.
    let mut dest: Vec<i32> = vec![0; num];
    let mut p = 0_usize; // index into PASS_GROUPS
    let mut randpasses = 0_usize;
    let mut d = 0_usize; // write cursor for fixed patterns
    let mut remaining = num;

    loop {
        let k = PASS_GROUPS[p];
        p += 1;

        if k == 0 {
            // Loop the table when more passes are still needed.
            p = 0;
            continue;
        }

        if k < 0 {
            let k = (-k) as usize;
            if k >= remaining {
                randpasses += remaining;
                break;
            }
            randpasses += k;
            remaining -= k;
            continue;
        }

        let k = k as usize;
        if k <= remaining {
            // Take the whole group of fixed patterns.
            for _ in 0..k {
                dest[d] = PASS_GROUPS[p];
                p += 1;
                d += 1;
            }
            remaining -= k;
            continue;
        }

        // Partial last group: if too small a fraction, finish with random passes;
        // otherwise sample `remaining` of the `k` available patterns.
        if remaining < 2 || 3 * remaining < k {
            randpasses += remaining;
            break;
        }

        let mut k_left = k;
        while remaining > 0 {
            if remaining == k_left || rng.choose(k_left as u64)? < remaining as u64 {
                dest[d] = PASS_GROUPS[p];
                d += 1;
                remaining -= 1;
            }
            p += 1;
            k_left -= 1;
        }
        break;
    }

    let mut top = num - randpasses;
    debug_assert_eq!(d, top);

    // Stage 2: place random passes with even spacing (Bresenham / DDA) and
    // Fisher-Yates-style swaps among the fixed patterns in between.
    let randpasses_m1 = randpasses.saturating_sub(1);
    let mut accum = randpasses_m1;
    for n in 0..num {
        if accum <= randpasses_m1 {
            accum += num - 1;
            dest[top] = dest[n];
            top += 1;
            dest[n] = -1; // random
        } else {
            let span = top - n;
            let swap = n + rng.choose(span as u64)? as usize;
            dest.swap(n, swap);
        }
        accum = accum.saturating_sub(randpasses_m1);
    }
    debug_assert_eq!(top, num);

    Ok(dest
        .into_iter()
        .map(|code| {
            if code < 0 {
                PassType::Random
            } else {
                PassType::Pattern(Pattern::from_code(code))
            }
        })
        .collect())
}

/// Build the pass sequence for `num_passes`, drawing scheduling entropy from
/// `--random-source` when provided (so order matches GNU for the same stream).
fn create_pass_sequence(
    num_passes: usize,
    random_source: Option<&RefCell<File>>,
) -> UResult<Vec<PassType>> {
    if let Some(source) = random_source {
        let mut rng = FilePassRng::new(source);
        genpattern(num_passes, &mut rng).map_err_context(
            || translate!("shred-file-write-pass-failed", "file" => "random-source"),
        )
    } else {
        let mut rng: StdRng = rand::make_rng();
        genpattern(num_passes, &mut rng).map_err(|_| {
            // StdRng never fails; keep type unified with the file path.
            USimpleError::new(
                1,
                translate!("shred-file-write-pass-failed", "file" => "rng"),
            )
        })
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::cognitive_complexity)]
fn wipe_file(
    path_str: &OsString,
    n_passes: usize,
    remove_method: RemoveMethod,
    size: Option<u64>,
    exact: bool,
    zero: bool,
    random_source: Option<&RefCell<File>>,
    verbose: bool,
    force: bool,
) -> UResult<()> {
    // Get these potential errors out of the way first
    let path = Path::new(path_str);

    if path_str.as_encoded_bytes().ends_with(b"/") {
        if path.is_dir() {
            return Err(USimpleError::new(
                1,
                translate!("shred-failed-to-open-for-writing-is-a-directory", "file" => path.maybe_quote()),
            ));
        }
        if fs::metadata(path).is_err_and(|e| e.kind() == io::ErrorKind::NotADirectory) {
            return Err(USimpleError::new(
                1,
                translate!("shred-failed-to-open-for-writing-not-a-directory", "file" => path.maybe_quote()),
            ));
        }
    }

    // `Path::exists()` and `Path::is_file()` both collapse any metadata error
    // (including a permission error) into `false`, which made shred report a
    // file whose parent directory lacks search permission as "No such file or
    // directory". Inspect the metadata directly so a genuine `ENOENT` stays a
    // "no such file" error while a permission error falls through to the
    // open-for-writing below, which surfaces the real reason.
    match fs::metadata(path) {
        Ok(md) if !md.is_file() => {
            return Err(USimpleError::new(
                1,
                translate!("shred-not-a-file", "file" => path.maybe_quote()),
            ));
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(USimpleError::new(
                1,
                translate!("shred-no-such-file-or-directory", "file" => path.maybe_quote()),
            ));
        }
        _ => {}
    }

    let metadata =
        fs::metadata(path).map_err_context(|| translate!("shred-failed-to-get-metadata"))?;

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
        fs::set_permissions(path, perms)
            .map_err_context(|| translate!("shred-failed-to-set-permissions"))?;
    }

    // Fill up our pass sequence
    let mut pass_sequence = Vec::new();
    if metadata.len() != 0 {
        // Only add passes if the file is non-empty
        pass_sequence = create_pass_sequence(n_passes, random_source)?;

        // --zero specifies whether we want one final pass of 0x00 on our file
        if zero {
            pass_sequence.push(PassType::Pattern(ZERO_PATTERN));
        }
    }

    let total_passes = pass_sequence.len();
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(false)
        .open(path)
        .map_err_context(
            || translate!("shred-failed-to-open-for-writing", "file" => path.maybe_quote()),
        )?;

    let size = match size {
        Some(size) => size,
        None => metadata.len(),
    };

    for (i, pass_type) in pass_sequence.into_iter().enumerate() {
        if verbose {
            let pass_name = pass_name(&pass_type);
            let msg = translate!("shred-pass-progress", "file" => path.maybe_quote());
            show_error!(
                "{msg} {}/{total_passes} ({pass_name})...",
                (i + 1).to_string()
            );
        }
        // size is an optional argument for exactly how many bytes we want to shred
        do_pass(&mut file, &pass_type, exact, random_source, size).map_err_context(
            || translate!("shred-file-write-pass-failed", "file" => path.maybe_quote()),
        )?;
    }

    if remove_method != RemoveMethod::None {
        do_remove(path, verbose, remove_method).map_err_context(
            || translate!("shred-failed-to-remove-file", "file" => path.maybe_quote()),
        )?;
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
    random_source: Option<&RefCell<File>>,
    file_size: u64,
) -> Result<(), io::Error> {
    // We might be at the end of the file due to a previous iteration, so rewind.
    file.rewind()?;

    let mut writer = BytesWriter::from_pass_type(pass_type, random_source)?;
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

/// Repeatedly renames the file with strings of decreasing length (most likely all 0s)
/// Return the path of the file after its last renaming or None in case of an error
fn wipe_name(orig_path: &Path, verbose: bool, remove_method: RemoveMethod) -> PathBuf {
    let file_name_len = orig_path.file_name().unwrap().len();

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
                            "{}: {} {}",
                            last_path.maybe_quote().to_string(),
                            translate!("shred-renamed-to"),
                            new_path.display().to_string()
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
                    let msg = translate!("shred-couldnt-rename", "file" => last_path.maybe_quote(), "new_name" => new_path.quote(), "error" => e);
                    show_error!("{msg}");
                    // TODO: replace with our error management
                    std::process::exit(1);
                }
            }
        }
    }

    last_path
}

fn do_remove(path: &Path, verbose: bool, remove_method: RemoveMethod) -> Result<(), io::Error> {
    if verbose {
        show_error!(
            "{}",
            translate!("shred-removing", "file" => path.maybe_quote())
        );
    }

    let remove_path = if remove_method == RemoveMethod::Unlink {
        path.to_path_buf()
    } else {
        wipe_name(path, verbose, remove_method)
    };

    fs::remove_file(remove_path)?;

    if verbose {
        show_error!(
            "{}",
            translate!("shred-removed", "file" => path.maybe_quote())
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        BLOCK_SIZE, BytesWriter, OPTIMAL_IO_BLOCK_SIZE, PassRng, PassType, Pattern, SECTOR_SIZE,
        create_pass_sequence, genpattern, pass_name, split_on_blocks,
    };
    use std::cell::RefCell;
    use std::fs::{File, OpenOptions};
    use std::io::{self, Write};

    /// Deterministic chooser for structural genpattern tests (always picks 0).
    struct FakeRng;

    impl PassRng for FakeRng {
        fn choose(&mut self, choices: u64) -> Result<u64, io::Error> {
            debug_assert!(choices > 0);
            Ok(0)
        }
    }

    /// In-memory byte stream implementing the same residual-entropy choose() as FilePassRng.
    struct SlicePassRng<'a> {
        data: &'a [u8],
        pos: usize,
        randnum: u64,
        randmax: u64,
    }

    impl<'a> SlicePassRng<'a> {
        fn new(data: &'a [u8]) -> Self {
            Self {
                data,
                pos: 0,
                randnum: 0,
                randmax: 0,
            }
        }

        fn genmax(&mut self, genmax: u64) -> Result<u64, io::Error> {
            let mut randnum = self.randnum;
            let mut randmax = self.randmax;
            let choices = genmax + 1;
            loop {
                if randmax < genmax {
                    let mut rmax = randmax;
                    let mut nbytes = 0_usize;
                    loop {
                        rmax = (rmax << 8) | u64::from(u8::MAX);
                        nbytes += 1;
                        if rmax >= genmax {
                            break;
                        }
                    }
                    let mut buf = [0_u8; 8];
                    for b in buf.iter_mut().take(nbytes) {
                        // Repeat the stream if exhausted (test streams are long).
                        if self.pos >= self.data.len() {
                            self.pos = 0;
                        }
                        *b = self.data[self.pos];
                        self.pos += 1;
                    }
                    for &byte in &buf[..nbytes] {
                        randnum = (randnum << 8) | u64::from(byte);
                        randmax = (randmax << 8) | u64::from(u8::MAX);
                    }
                }
                if randmax == genmax {
                    self.randnum = 0;
                    self.randmax = 0;
                    return Ok(randnum);
                }
                let excess_choices = randmax - genmax;
                let unusable_choices = excess_choices % choices;
                let last_usable_choice = randmax - unusable_choices;
                let reduced_randnum = randnum % choices;
                if randnum <= last_usable_choice {
                    self.randnum = randnum / choices;
                    self.randmax = excess_choices / choices;
                    return Ok(reduced_randnum);
                }
                randnum = reduced_randnum;
                randmax = unusable_choices - 1;
            }
        }
    }

    impl PassRng for SlicePassRng<'_> {
        fn choose(&mut self, choices: u64) -> Result<u64, io::Error> {
            self.genmax(choices - 1)
        }
    }

    fn classify(name: &str) -> &'static str {
        if name == "random" {
            return "random";
        }
        let b = name.as_bytes();
        if b.len() == 6 && b[0] == b[2] && b[2] == b[4] && b[1] == b[3] && b[3] == b[5] {
            "single"
        } else {
            "multi"
        }
    }

    fn count_classes(seq: &[PassType]) -> (usize, usize, usize) {
        let mut random = 0;
        let mut single = 0;
        let mut multi = 0;
        for p in seq {
            match classify(&pass_name(p)) {
                "random" => random += 1,
                "single" => single += 1,
                _ => multi += 1,
            }
        }
        (random, single, multi)
    }

    #[test]
    fn test_pattern_from_code_flip() {
        let p = Pattern::from_code(0x1000);
        assert_eq!(p.bytes, [0, 0, 0]);
        assert!(p.flip_sector);
        assert_eq!(p.display_bytes(), [0x80, 0, 0]);
        assert_eq!(pass_name(&PassType::Pattern(p)), "800000");

        let p = Pattern::from_code(0x1111);
        assert_eq!(p.display_bytes(), [0x91, 0x11, 0x11]);
        assert_eq!(pass_name(&PassType::Pattern(p)), "911111");

        let p = Pattern::from_code(0x000);
        assert!(!p.flip_sector);
        assert_eq!(p.display_bytes(), [0, 0, 0]);
        assert_eq!(pass_name(&PassType::Pattern(p)), "000000");
    }

    #[test]
    fn test_sector_phase_writer_flips_each_sector() {
        let pass = PassType::Pattern(Pattern::from_code(0x1000));
        let mut writer = BytesWriter::from_pass_type(&pass, None).unwrap();
        let bytes = writer.bytes_for_pass(BLOCK_SIZE).unwrap();
        for i in (0..BLOCK_SIZE).step_by(SECTOR_SIZE) {
            assert_eq!(bytes[i], 0x80, "sector start at {i}");
            if i + 1 < BLOCK_SIZE {
                assert_eq!(bytes[i + 1], 0x00);
            }
        }
        let bytes2 = writer.bytes_for_pass(SECTOR_SIZE).unwrap();
        assert_eq!(bytes2[0], 0x80);

        let plain = PassType::Pattern(Pattern::from_code(0x111));
        let mut writer = BytesWriter::from_pass_type(&plain, None).unwrap();
        let bytes = writer.bytes_for_pass(BLOCK_SIZE).unwrap();
        assert_eq!(bytes[0], 0x11);
        assert_eq!(bytes[SECTOR_SIZE], 0x11);
    }

    #[test]
    fn test_genpattern_small_n_all_random() {
        let mut rng = FakeRng;
        assert!(genpattern(0, &mut rng).unwrap().is_empty());
        for n in 1..=3 {
            let seq = genpattern(n, &mut rng).unwrap();
            assert_eq!(seq.len(), n);
            assert!(seq.iter().all(|p| matches!(p, PassType::Random)));
        }
    }

    #[test]
    fn test_genpattern_n_passes_length() {
        let mut rng = FakeRng;
        for n in [0, 1, 3, 10, 25, 35, 50, 100] {
            let seq = genpattern(n, &mut rng).unwrap();
            assert_eq!(seq.len(), n, "length mismatch for n={n}");
        }
    }

    #[test]
    fn test_genpattern_n20_matches_gnu_with_0x55_source() {
        // Verified against GNU shred 9.11: gshred -v -n20 --random-source=<0x55...>
        let stream = vec![0x55_u8; 1_000_000];
        let mut rng = SlicePassRng::new(&stream);
        let seq = genpattern(20, &mut rng).unwrap();
        let names: Vec<String> = seq.iter().map(pass_name).collect();
        assert_eq!(
            names,
            [
                "random", "ffffff", "924924", "888888", "db6db6", "777777", "492492", "bbbbbb",
                "555555", "aaaaaa", "random", "6db6db", "249249", "999999", "111111", "000000",
                "b6db6d", "eeeeee", "333333", "random",
            ]
        );
    }

    #[test]
    fn test_genpattern_distribution_matches_gnu() {
        // Counts from GNU shred 9.11 with a 0x55-filled --random-source (issue #11611).
        let stream = vec![0x55_u8; 10_000_000];
        let expected = [
            (25, 3, 16, 6),
            (50, 6, 16, 28),
            (60, 8, 20, 32),
            (100, 12, 32, 56),
            (500, 53, 164, 283),
            (1000, 103, 331, 566),
        ];
        for (n, er, es, em) in expected {
            let mut rng = SlicePassRng::new(&stream);
            let seq = genpattern(n, &mut rng).unwrap();
            let (r, s, m) = count_classes(&seq);
            assert_eq!((r, s, m), (er, es, em), "distribution mismatch for n={n}");
        }
    }

    // `std::env::temp_dir` aborts on WASI; filesystem random-source coverage is
    // exercised by the host integration tests and the in-memory genpattern tests.
    #[test]
    #[cfg(not(target_family = "wasm"))]
    fn test_create_pass_sequence_with_file_source() {
        let path = std::env::temp_dir().join("uu_shred_11611_rng");
        {
            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            f.write_all(&vec![0x55; 1_000_000]).unwrap();
        }
        let file = File::open(&path).unwrap();
        let cell = RefCell::new(file);
        let seq = create_pass_sequence(20, Some(&cell)).unwrap();
        let names: Vec<String> = seq.iter().map(pass_name).collect();
        assert_eq!(names[0], "random");
        assert_eq!(names[19], "random");
        assert_eq!(names[1], "ffffff");
        let _ = std::fs::remove_file(&path);
    }

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
