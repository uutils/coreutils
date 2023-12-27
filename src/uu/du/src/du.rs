// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod physical_extents;

use chrono::{DateTime, Local};
use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use glob::Pattern;
use physical_extents::SeenPhysicalExtents;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::error::Error;
use std::fmt::Display;
#[cfg(not(windows))]
use std::fs::Metadata;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
#[cfg(not(windows))]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, UNIX_EPOCH};
use uucore::display::{print_verbatim, Quotable};
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::line_ending::LineEnding;
use uucore::parse_glob;
use uucore::parse_size::{parse_size_u64, ParseSizeError};
use uucore::{format_usage, help_about, help_section, help_usage, show, show_warning};
#[cfg(windows)]
use windows_sys::Win32::Foundation::HANDLE;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FileIdInfo, FileStandardInfo, GetFileInformationByHandleEx, FILE_ID_128, FILE_ID_INFO,
    FILE_STANDARD_INFO,
};

mod options {
    pub const HELP: &str = "help";
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
    pub const THRESHOLD: &str = "threshold";
    pub const SI: &str = "si";
    pub const TIME: &str = "time";
    pub const TIME_STYLE: &str = "time-style";
    pub const ONE_FILE_SYSTEM: &str = "one-file-system";
    pub const DEREFERENCE: &str = "dereference";
    pub const DEREFERENCE_ARGS: &str = "dereference-args";
    pub const NO_DEREFERENCE: &str = "no-dereference";
    pub const INODES: &str = "inodes";
    pub const EXCLUDE: &str = "exclude";
    pub const EXCLUDE_FROM: &str = "exclude-from";
    pub const FILES0_FROM: &str = "files0-from";
    pub const VERBOSE: &str = "verbose";
    pub const FILE: &str = "FILE";
    pub const SHARED_EXTENTS: &str = "shared-extents";
}

const ABOUT: &str = help_about!("du.md");
const AFTER_HELP: &str = help_section!("after help", "du.md");
const USAGE: &str = help_usage!("du.md");

// TODO: Support Z & Y (currently limited by size of u64)
const UNITS: [(char, u32); 6] = [('E', 6), ('P', 5), ('T', 4), ('G', 3), ('M', 2), ('K', 1)];

struct TraversalOptions {
    all: bool,
    separate_dirs: bool,
    one_file_system: bool,
    dereference: Deref,
    shared_extents: bool,
    count_links: bool,
    verbose: bool,
    excludes: Vec<Pattern>,
}

struct StatPrinter {
    total: bool,
    inodes: bool,
    max_depth: Option<usize>,
    threshold: Option<Threshold>,
    apparent_size: bool,
    size_format: SizeFormat,
    time: Option<Time>,
    time_format: String,
    line_ending: LineEnding,
    summarize: bool,
}

#[derive(PartialEq, Clone)]
enum Deref {
    All,
    Args(Vec<PathBuf>),
    None,
}

#[derive(Clone, Copy)]
enum Time {
    Accessed,
    Modified,
    Created,
}

#[derive(Clone)]
enum SizeFormat {
    Human(u64),
    BlockSize(u64),
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct FileInfo {
    file_id: u128,
    dev_id: u64,
}

struct Stat {
    path: PathBuf,
    is_dir: bool,
    is_symlink: bool,
    size: u64,
    blocks: u64,
    inodes: u64,
    inode: Option<FileInfo>,
    created: Option<u64>,
    accessed: u64,
    modified: u64,
}

impl Stat {
    fn new(path: &Path, options: &TraversalOptions) -> std::io::Result<Self> {
        // Determine whether to dereference (follow) the symbolic link
        let should_dereference = match &options.dereference {
            Deref::All => true,
            Deref::Args(paths) => paths.contains(&path.to_path_buf()),
            Deref::None => false,
        };

        let metadata = if should_dereference {
            // Get metadata, following symbolic links if necessary
            fs::metadata(path)
        } else {
            // Get metadata without following symbolic links
            fs::symlink_metadata(path)
        }?;

        #[cfg(not(windows))]
        {
            let file_info = FileInfo {
                file_id: metadata.ino() as u128,
                dev_id: metadata.dev(),
            };

            Ok(Self {
                path: path.to_path_buf(),
                is_dir: metadata.is_dir(),
                is_symlink: metadata.is_symlink(),
                size: if path.is_dir() { 0 } else { metadata.len() },
                blocks: metadata.blocks(),
                inodes: 1,
                inode: Some(file_info),
                created: birth_u64(&metadata),
                accessed: metadata.atime() as u64,
                modified: metadata.mtime() as u64,
            })
        }

        #[cfg(windows)]
        {
            let size_on_disk = get_size_on_disk(path);
            let file_info = get_file_info(path);

            Ok(Self {
                path: path.to_path_buf(),
                is_dir: metadata.is_dir(),
                size: if path.is_dir() { 0 } else { metadata.len() },
                blocks: size_on_disk / 1024 * 2,
                inodes: 1,
                inode: file_info,
                created: windows_creation_time_to_unix_time(metadata.creation_time()),
                accessed: windows_time_to_unix_time(metadata.last_access_time()),
                modified: windows_time_to_unix_time(metadata.last_write_time()),
            })
        }
    }
}

#[cfg(windows)]
// https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html#tymethod.last_access_time
// "The returned 64-bit value [...] which represents the number of 100-nanosecond intervals since January 1, 1601 (UTC)."
// "If the underlying filesystem does not support last access time, the returned value is 0."
fn windows_time_to_unix_time(win_time: u64) -> u64 {
    (win_time / 10_000_000).saturating_sub(11_644_473_600)
}

#[cfg(windows)]
fn windows_creation_time_to_unix_time(win_time: u64) -> Option<u64> {
    (win_time / 10_000_000).checked_sub(11_644_473_600)
}

#[cfg(not(windows))]
fn birth_u64(meta: &Metadata) -> Option<u64> {
    meta.created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|e| e.as_secs())
}

#[cfg(windows)]
fn get_size_on_disk(path: &Path) -> u64 {
    let mut size_on_disk = 0;

    // bind file so it stays in scope until end of function
    // if it goes out of scope the handle below becomes invalid
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return size_on_disk, // opening directories will fail
    };

    unsafe {
        let mut file_info: FILE_STANDARD_INFO = core::mem::zeroed();
        let file_info_ptr: *mut FILE_STANDARD_INFO = &mut file_info;

        let success = GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileStandardInfo,
            file_info_ptr as _,
            std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
        );

        if success != 0 {
            size_on_disk = file_info.AllocationSize as u64;
        }
    }

    size_on_disk
}

#[cfg(windows)]
fn get_file_info(path: &Path) -> Option<FileInfo> {
    let mut result = None;

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return result,
    };

    unsafe {
        let mut file_info: FILE_ID_INFO = core::mem::zeroed();
        let file_info_ptr: *mut FILE_ID_INFO = &mut file_info;

        let success = GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileIdInfo,
            file_info_ptr as _,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        );

        if success != 0 {
            result = Some(FileInfo {
                file_id: std::mem::transmute::<FILE_ID_128, u128>(file_info.FileId),
                dev_id: file_info.VolumeSerialNumber,
            });
        }
    }

    result
}

fn read_block_size(s: Option<&str>) -> UResult<u64> {
    if let Some(s) = s {
        parse_size_u64(s)
            .map_err(|e| USimpleError::new(1, format_error_message(&e, s, options::BLOCK_SIZE)))
    } else {
        for env_var in ["DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"] {
            if let Ok(env_size) = env::var(env_var) {
                if let Ok(v) = parse_size_u64(&env_size) {
                    return Ok(v);
                }
            }
        }
        if env::var("POSIXLY_CORRECT").is_ok() {
            Ok(512)
        } else {
            Ok(1024)
        }
    }
}

struct DiskUsageCalculator<'a> {
    print_tx: &'a mpsc::Sender<UResult<StatPrintInfo>>,
    options: &'a TraversalOptions,
    seen_inodes: HashSet<FileInfo>,
    seen_physical_extents: BTreeMap<u64, SeenPhysicalExtents>,
}

impl<'a> DiskUsageCalculator<'a> {
    fn new<'b>(
        print_tx: &'a mpsc::Sender<UResult<StatPrintInfo>>,
        options: &'a TraversalOptions,
    ) -> Self {
        return DiskUsageCalculator {
            print_tx,
            options,
            seen_inodes: HashSet::new(),
            seen_physical_extents: BTreeMap::new(),
        };
    }

    fn is_entry_excluded(&self, entry: &fs::DirEntry, entry_stat: &Stat) -> bool {
        for pattern in &self.options.excludes {
            // Look at all patterns with both short and long paths
            // if we have 'du foo' but search to exclude 'foo/bar'
            // we need the full path
            if pattern.matches(&entry_stat.path.to_string_lossy())
                || pattern.matches(&entry.file_name().into_string().unwrap())
            {
                // if the directory is ignored, leave early
                if self.options.verbose {
                    println!("{} ignored", &entry_stat.path.quote());
                }
                // Go to the next file
                return true;
            }
        }

        false
    }

    fn du_handle_dir_entry(
        &mut self,
        base_stat: &mut Stat,
        depth: usize,
        entry: &fs::DirEntry,
        entry_stat: Stat,
    ) -> Result<(), Box<mpsc::SendError<UResult<StatPrintInfo>>>> {
        // We have an exclude list
        if self.is_entry_excluded(entry, &entry_stat) {
            return Ok(());
        }

        if let Some(inode) = entry_stat.inode {
            if self.seen_inodes.contains(&inode) {
                if self.options.count_links {
                    base_stat.inodes += 1;
                }
                return Ok(());
            }
            self.seen_inodes.insert(inode);
        }

        let total_overlapping_by_extents = if self.options.shared_extents
            && !entry_stat.is_symlink
            && !entry_stat.is_dir
            && entry_stat.inode.is_some()
        {
            let map_by_device = self
                .seen_physical_extents
                .entry(entry_stat.inode.unwrap().dev_id)
                .or_insert(SeenPhysicalExtents::default());

            let (total_overlapping, errors) =
                map_by_device.get_total_overlap_and_insert(entry.path());

            for error in errors {
                self.print_tx.send(Err(error))?;
            }

            total_overlapping
        } else {
            0
        };

        let is_ignored_by_extents: bool =
            total_overlapping_by_extents > 0 && total_overlapping_by_extents >= entry_stat.size;
        if is_ignored_by_extents {
            return Ok(());
        }

        if entry_stat.is_dir {
            if self.options.one_file_system {
                if let (Some(this_inode), Some(my_inode)) = (entry_stat.inode, base_stat.inode) {
                    if this_inode.dev_id != my_inode.dev_id {
                        return Ok(());
                    }
                }
            }

            let this_stat = self.run(entry_stat, depth + 1)?;

            if !self.options.separate_dirs {
                base_stat.size += this_stat.size;
                base_stat.blocks += this_stat.blocks;
                base_stat.inodes += this_stat.inodes;
            }
            self.print_tx.send(Ok(StatPrintInfo {
                stat: this_stat,
                depth: depth + 1,
            }))?;
        } else {
            let mut adapted: Stat = entry_stat;
            adapted.size -= total_overlapping_by_extents;
            adapted.blocks -= total_overlapping_by_extents / 512;

            base_stat.size += adapted.size;
            base_stat.blocks += adapted.blocks;
            base_stat.inodes += 1;

            if self.options.all {
                self.print_tx.send(Ok(StatPrintInfo {
                    stat: adapted,
                    depth: depth + 1,
                }))?;
            }
        }

        Ok(())
    }

    // this takes `my_stat` to avoid having to stat files multiple times.
    #[allow(clippy::cognitive_complexity)]
    fn run(
        &mut self,
        mut my_stat: Stat,
        depth: usize,
    ) -> Result<Stat, Box<mpsc::SendError<UResult<StatPrintInfo>>>> {
        if !my_stat.is_dir {
            return Ok(my_stat);
        }

        let dir_iterator = match fs::read_dir(&my_stat.path) {
            Ok(dir_iterator) => dir_iterator,
            Err(e) => {
                self.print_tx.send(Err(e.map_err_context(|| {
                    format!("cannot read directory {}", my_stat.path.quote())
                })))?;
                return Ok(my_stat);
            }
        };

        for f in dir_iterator {
            match f {
                Ok(directory_entry) => match Stat::new(&directory_entry.path(), self.options) {
                    Ok(this_stat) => {
                        self.du_handle_dir_entry(&mut my_stat, depth, &directory_entry, this_stat)?;
                    }
                    Err(e) => self.print_tx.send(Err(e.map_err_context(|| {
                        format!("cannot access {}", directory_entry.path().quote())
                    })))?,
                },
                Err(error) => self.print_tx.send(Err(error.into()))?,
            }
        }

        Ok(my_stat)
    }
}

#[derive(Debug)]
enum DuError {
    InvalidMaxDepthArg(String),
    SummarizeDepthConflict(String),
    InvalidTimeStyleArg(String),
    InvalidTimeArg,
    InvalidGlob(String),
}

impl Display for DuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMaxDepthArg(s) => write!(f, "invalid maximum depth {}", s.quote()),
            Self::SummarizeDepthConflict(s) => {
                write!(
                    f,
                    "summarizing conflicts with --max-depth={}",
                    s.maybe_quote()
                )
            }
            Self::InvalidTimeStyleArg(s) => write!(
                f,
                "invalid argument {} for 'time style'
Valid arguments are:
- 'full-iso'
- 'long-iso'
- 'iso'
Try '{} --help' for more information.",
                s.quote(),
                uucore::execution_phrase()
            ),
            Self::InvalidTimeArg => write!(
                f,
                "'birth' and 'creation' arguments for --time are not supported on this platform.",
            ),
            Self::InvalidGlob(s) => write!(f, "Invalid exclude syntax: {s}"),
        }
    }
}

impl Error for DuError {}

impl UError for DuError {
    fn code(&self) -> i32 {
        match self {
            Self::InvalidMaxDepthArg(_)
            | Self::SummarizeDepthConflict(_)
            | Self::InvalidTimeStyleArg(_)
            | Self::InvalidTimeArg
            | Self::InvalidGlob(_) => 1,
        }
    }
}

// Read a file and return each line in a vector of String
fn file_as_vec(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);

    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

// Given the --exclude-from and/or --exclude arguments, returns the globset lists
// to ignore the files
fn build_exclude_patterns(matches: &ArgMatches) -> UResult<Vec<Pattern>> {
    let exclude_from_iterator = matches
        .get_many::<String>(options::EXCLUDE_FROM)
        .unwrap_or_default()
        .flat_map(file_as_vec);

    let excludes_iterator = matches
        .get_many::<String>(options::EXCLUDE)
        .unwrap_or_default()
        .cloned();

    let mut exclude_patterns = Vec::new();
    for f in excludes_iterator.chain(exclude_from_iterator) {
        if matches.get_flag(options::VERBOSE) {
            println!("adding {:?} to the exclude list ", &f);
        }
        match parse_glob::from_str(&f) {
            Ok(glob) => exclude_patterns.push(glob),
            Err(err) => return Err(DuError::InvalidGlob(err.to_string()).into()),
        }
    }
    Ok(exclude_patterns)
}

struct StatPrintInfo {
    stat: Stat,
    depth: usize,
}

impl StatPrinter {
    fn choose_size(&self, stat: &Stat) -> u64 {
        if self.inodes {
            stat.inodes
        } else if self.apparent_size {
            stat.size
        } else {
            // The st_blocks field indicates the number of blocks allocated to the file, 512-byte units.
            // See: http://linux.die.net/man/2/stat
            stat.blocks * 512
        }
    }

    fn print_stats(&self, rx: &mpsc::Receiver<UResult<StatPrintInfo>>) -> UResult<()> {
        let mut grand_total = 0;
        loop {
            let received = rx.recv();

            match received {
                Ok(message) => match message {
                    Ok(stat_info) => {
                        let size = self.choose_size(&stat_info.stat);

                        if stat_info.depth == 0 {
                            grand_total += size;
                        }

                        if !self
                            .threshold
                            .map_or(false, |threshold| threshold.should_exclude(size))
                            && self
                                .max_depth
                                .map_or(true, |max_depth| stat_info.depth <= max_depth)
                            && (!self.summarize || stat_info.depth == 0)
                        {
                            self.print_stat(&stat_info.stat, size)?;
                        }
                    }
                    Err(e) => show!(e),
                },
                Err(_) => break,
            }
        }

        if self.total {
            print!("{}\ttotal", self.convert_size(grand_total));
            print!("{}", self.line_ending);
        }

        Ok(())
    }

    fn convert_size(&self, size: u64) -> String {
        if self.inodes {
            return size.to_string();
        }
        match self.size_format {
            SizeFormat::Human(multiplier) => {
                if size == 0 {
                    return "0".to_string();
                }
                for &(unit, power) in &UNITS {
                    let limit = multiplier.pow(power);
                    if size >= limit {
                        return format!("{:.1}{}", (size as f64) / (limit as f64), unit);
                    }
                }
                format!("{size}B")
            }
            SizeFormat::BlockSize(block_size) => div_ceil(size, block_size).to_string(),
        }
    }

    fn print_stat(&self, stat: &Stat, size: u64) -> UResult<()> {
        if let Some(time) = self.time {
            let secs = get_time_secs(time, stat)?;
            let tm = DateTime::<Local>::from(UNIX_EPOCH + Duration::from_secs(secs));
            let time_str = tm.format(&self.time_format).to_string();
            print!("{}\t{}\t", self.convert_size(size), time_str);
        } else {
            print!("{}\t", self.convert_size(size));
        }

        print_verbatim(&stat.path).unwrap();
        print!("{}", self.line_ending);

        Ok(())
    }
}

// This can be replaced with u64::div_ceil once it is stabilized.
// This implementation approach is optimized for when `b` is a constant,
// particularly a power of two.
pub fn div_ceil(a: u64, b: u64) -> u64 {
    (a + b - 1) / b
}

// Read file paths from the specified file, separated by null characters
fn read_files_from(file_name: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    let reader: Box<dyn BufRead> = if file_name == "-" {
        // Read from standard input
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        // First, check if the file_name is a directory
        let path = PathBuf::from(file_name);
        if path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{}: read error: Is a directory", file_name),
            ));
        }

        // Attempt to open the file and handle the error if it does not exist
        match File::open(file_name) {
            Ok(file) => Box::new(BufReader::new(file)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "cannot open '{}' for reading: No such file or directory",
                        file_name
                    ),
                ))
            }
            Err(e) => return Err(e),
        }
    };

    let mut paths = Vec::new();

    for line in reader.split(b'\0') {
        let path = line?;
        if !path.is_empty() {
            paths.push(PathBuf::from(String::from_utf8_lossy(&path).to_string()));
        }
    }

    Ok(paths)
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let summarize = matches.get_flag(options::SUMMARIZE);

    let max_depth = parse_depth(
        matches
            .get_one::<String>(options::MAX_DEPTH)
            .map(|s| s.as_str()),
        summarize,
    )?;

    let files = if let Some(file_from) = matches.get_one::<String>(options::FILES0_FROM) {
        if file_from == "-" && matches.get_one::<String>(options::FILE).is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "extra operand {}\nfile operands cannot be combined with --files0-from",
                    matches.get_one::<String>(options::FILE).unwrap().quote()
                ),
            )
            .into());
        }

        read_files_from(file_from)?
    } else {
        match matches.get_one::<String>(options::FILE) {
            Some(_) => matches
                .get_many::<String>(options::FILE)
                .unwrap()
                .map(PathBuf::from)
                .collect(),
            None => vec![PathBuf::from(".")],
        }
    };

    let time = matches.contains_id(options::TIME).then(|| {
        match matches.get_one::<String>(options::TIME).map(AsRef::as_ref) {
            None | Some("ctime" | "status") => Time::Modified,
            Some("access" | "atime" | "use") => Time::Accessed,
            Some("birth" | "creation") => Time::Created,
            _ => unreachable!("should be caught by clap"),
        }
    });

    let size_format = if matches.get_flag(options::HUMAN_READABLE) {
        SizeFormat::Human(1024)
    } else if matches.get_flag(options::SI) {
        SizeFormat::Human(1000)
    } else if matches.get_flag(options::BYTES) {
        SizeFormat::BlockSize(1)
    } else if matches.get_flag(options::BLOCK_SIZE_1K) {
        SizeFormat::BlockSize(1024)
    } else if matches.get_flag(options::BLOCK_SIZE_1M) {
        SizeFormat::BlockSize(1024 * 1024)
    } else {
        SizeFormat::BlockSize(read_block_size(
            matches
                .get_one::<String>(options::BLOCK_SIZE)
                .map(AsRef::as_ref),
        )?)
    };

    let traversal_options = TraversalOptions {
        all: matches.get_flag(options::ALL),
        separate_dirs: matches.get_flag(options::SEPARATE_DIRS),
        one_file_system: matches.get_flag(options::ONE_FILE_SYSTEM),
        dereference: if matches.get_flag(options::DEREFERENCE) {
            Deref::All
        } else if matches.get_flag(options::DEREFERENCE_ARGS) {
            // We don't care about the cost of cloning as it is rarely used
            Deref::Args(files.clone())
        } else {
            Deref::None
        },
        shared_extents: matches.get_flag(options::SHARED_EXTENTS),
        count_links: matches.get_flag(options::COUNT_LINKS),
        verbose: matches.get_flag(options::VERBOSE),
        excludes: build_exclude_patterns(&matches)?,
    };

    let stat_printer = StatPrinter {
        max_depth,
        size_format,
        summarize,
        total: matches.get_flag(options::TOTAL),
        inodes: matches.get_flag(options::INODES),
        threshold: matches
            .get_one::<String>(options::THRESHOLD)
            .map(|s| {
                Threshold::from_str(s).map_err(|e| {
                    USimpleError::new(1, format_error_message(&e, s, options::THRESHOLD))
                })
            })
            .transpose()?,
        apparent_size: matches.get_flag(options::APPARENT_SIZE) || matches.get_flag(options::BYTES),
        time,
        time_format: parse_time_style(matches.get_one::<String>("time-style").map(|s| s.as_str()))?
            .to_string(),
        line_ending: LineEnding::from_zero_flag(matches.get_flag(options::NULL)),
    };

    if stat_printer.inodes
        && (matches.get_flag(options::APPARENT_SIZE) || matches.get_flag(options::BYTES))
    {
        show_warning!("options --apparent-size and -b are ineffective with --inodes");
    }

    // Use separate thread to print output, so we can print finished results while computation is still running
    let (print_tx, rx) = mpsc::channel::<UResult<StatPrintInfo>>();
    let printing_thread = thread::spawn(move || stat_printer.print_stats(&rx));

    'loop_file: for path in files {
        // Skip if we don't want to ignore anything
        if !&traversal_options.excludes.is_empty() {
            let path_string = path.to_string_lossy();
            for pattern in &traversal_options.excludes {
                if pattern.matches(&path_string) {
                    // if the directory is ignored, leave early
                    if traversal_options.verbose {
                        println!("{} ignored", path_string.quote());
                    }
                    continue 'loop_file;
                }
            }
        }

        // Check existence of path provided in argument
        if let Ok(stat) = Stat::new(&path, &traversal_options) {
            // Kick off the computation of disk usage from the initial path
            let mut du_calc = DiskUsageCalculator::new(&print_tx, &traversal_options);
            if let Some(inode) = stat.inode {
                du_calc.seen_inodes.insert(inode);
            }
            let stat = du_calc
                .run(stat, 0)
                .map_err(|e| USimpleError::new(1, e.to_string()))?;

            print_tx
                .send(Ok(StatPrintInfo { stat, depth: 0 }))
                .map_err(|e| USimpleError::new(1, e.to_string()))?;
        } else {
            print_tx
                .send(Err(USimpleError::new(
                    1,
                    format!(
                        "{}: No such file or directory",
                        path.to_string_lossy().maybe_quote()
                    ),
                )))
                .map_err(|e| USimpleError::new(1, e.to_string()))?;
        }
    }

    drop(print_tx);

    printing_thread
        .join()
        .map_err(|_| USimpleError::new(1, "Printing thread panicked."))??;

    Ok(())
}

fn get_time_secs(time: Time, stat: &Stat) -> Result<u64, DuError> {
    match time {
        Time::Modified => Ok(stat.modified),
        Time::Accessed => Ok(stat.accessed),
        Time::Created => stat.created.ok_or(DuError::InvalidTimeArg),
    }
}

fn parse_time_style(s: Option<&str>) -> UResult<&str> {
    match s {
        Some(s) => match s {
            "full-iso" => Ok("%Y-%m-%d %H:%M:%S.%f %z"),
            "long-iso" => Ok("%Y-%m-%d %H:%M"),
            "iso" => Ok("%Y-%m-%d"),
            _ => Err(DuError::InvalidTimeStyleArg(s.into()).into()),
        },
        None => Ok("%Y-%m-%d %H:%M"),
    }
}

fn parse_depth(max_depth_str: Option<&str>, summarize: bool) -> UResult<Option<usize>> {
    let max_depth = max_depth_str.as_ref().and_then(|s| s.parse::<usize>().ok());
    match (max_depth_str, max_depth) {
        (Some(s), _) if summarize => Err(DuError::SummarizeDepthConflict(s.into()).into()),
        (Some(s), None) => Err(DuError::InvalidMaxDepthArg(s.into()).into()),
        (Some(_), Some(_)) | (None, _) => Ok(max_depth),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help)
        )
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("write counts for all files, not just directories")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::APPARENT_SIZE)
                .long(options::APPARENT_SIZE)
                .help(
                    "print apparent sizes, rather than disk usage \
                    although the apparent size is usually smaller, it may be larger due to holes \
                    in ('sparse') files, internal fragmentation, indirect blocks, and the like"
                )
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::BLOCK_SIZE)
                .short('B')
                .long(options::BLOCK_SIZE)
                .value_name("SIZE")
                .help(
                    "scale sizes by SIZE before printing them. \
                    E.g., '-BM' prints sizes in units of 1,048,576 bytes. See SIZE format below."
                )
        )
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long("bytes")
                .help("equivalent to '--apparent-size --block-size=1'")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::TOTAL)
                .long("total")
                .short('c')
                .help("produce a grand total")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::MAX_DEPTH)
                .short('d')
                .long("max-depth")
                .value_name("N")
                .help(
                    "print the total for a directory (or file, with --all) \
                    only if it is N or fewer levels below the command \
                    line argument;  --max-depth=0 is the same as --summarize"
                )
        )
        .arg(
            Arg::new(options::HUMAN_READABLE)
                .long("human-readable")
                .short('h')
                .help("print sizes in human readable format (e.g., 1K 234M 2G)")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::INODES)
                .long(options::INODES)
                .help(
                    "list inode usage information instead of block usage like --block-size=1K"
                )
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::BLOCK_SIZE_1K)
                .short('k')
                .help("like --block-size=1K")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::COUNT_LINKS)
                .short('l')
                .long("count-links")
                .help("count sizes many times if hard linked")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help("follow all symbolic links")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::DEREFERENCE_ARGS)
                .short('D')
                .visible_short_alias('H')
                .long(options::DEREFERENCE_ARGS)
                .help("follow only symlinks that are listed on the command line")
                .action(ArgAction::SetTrue)
         )
         .arg(
             Arg::new(options::NO_DEREFERENCE)
                 .short('P')
                 .long(options::NO_DEREFERENCE)
                 .help("don't follow any symbolic links (this is the default)")
                 .overrides_with(options::DEREFERENCE)
                 .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHARED_EXTENTS)
                .short('e')
                .long(options::SHARED_EXTENTS)
                .help("search for shared file extents (e.g. on CoW filesystems)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BLOCK_SIZE_1M)
                .short('m')
                .help("like --block-size=1M")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::NULL)
                .short('0')
                .long("null")
                .help("end each output line with 0 byte rather than newline")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::SEPARATE_DIRS)
                .short('S')
                .long("separate-dirs")
                .help("do not include size of subdirectories")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::SUMMARIZE)
                .short('s')
                .long("summarize")
                .help("display only a total for each argument")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::SI)
                .long(options::SI)
                .help("like -h, but use powers of 1000 not 1024")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::ONE_FILE_SYSTEM)
                .short('x')
                .long(options::ONE_FILE_SYSTEM)
                .help("skip directories on different file systems")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::THRESHOLD)
                .short('t')
                .long(options::THRESHOLD)
                .value_name("SIZE")
                .num_args(1)
                .allow_hyphen_values(true)
                .help("exclude entries smaller than SIZE if positive, \
                          or entries greater than SIZE if negative")
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long("verbose")
                .help("verbose mode (option not present in GNU/Coreutils)")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(options::EXCLUDE)
                .long(options::EXCLUDE)
                .value_name("PATTERN")
                .help("exclude files that match PATTERN")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new(options::EXCLUDE_FROM)
                .short('X')
                .long("exclude-from")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .help("exclude files that match any pattern in FILE")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new(options::FILES0_FROM)
                .long("files0-from")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .help("summarize device usage of the NUL-terminated file names specified in file F; if F is -, then read names from standard input")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .value_name("WORD")
                .require_equals(true)
                .num_args(0..)
                .value_parser(["atime", "access", "use", "ctime", "status", "birth", "creation"])
                .help(
                    "show time of the last modification of any file in the \
                    directory, or any of its subdirectories. If WORD is given, show time as WORD instead \
                    of modification time: atime, access, use, ctime, status, birth or creation"
                )
        )
        .arg(
            Arg::new(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .value_name("STYLE")
                .help(
                    "show times using style STYLE: \
                    full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'"
                )
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .value_hint(clap::ValueHint::AnyPath)
                .action(ArgAction::Append)
        )
}

#[derive(Clone, Copy)]
enum Threshold {
    Lower(u64),
    Upper(u64),
}

impl FromStr for Threshold {
    type Err = ParseSizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let offset = usize::from(s.starts_with(&['-', '+'][..]));

        let size = parse_size_u64(&s[offset..])?;

        if s.starts_with('-') {
            // Threshold of '-0' excludes everything besides 0 sized entries
            // GNU's du treats '-0' as an invalid argument
            if size == 0 {
                return Err(ParseSizeError::ParseFailure(s.to_string()));
            }
            Ok(Self::Upper(size))
        } else {
            Ok(Self::Lower(size))
        }
    }
}

impl Threshold {
    fn should_exclude(&self, size: u64) -> bool {
        match *self {
            Self::Upper(threshold) => size > threshold,
            Self::Lower(threshold) => size < threshold,
        }
    }
}

fn format_error_message(error: &ParseSizeError, s: &str, option: &str) -> String {
    // NOTE:
    // GNU's du echos affected flag, -B or --block-size (-t or --threshold), depending user's selection
    match error {
        ParseSizeError::InvalidSuffix(_) => {
            format!("invalid suffix in --{} argument {}", option, s.quote())
        }
        ParseSizeError::ParseFailure(_) => format!("invalid --{} argument {}", option, s.quote()),
        ParseSizeError::SizeTooBig(_) => format!("--{} argument {} too large", option, s.quote()),
    }
}

#[cfg(test)]
mod test_du {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_read_block_size() {
        let test_data = [Some("1024".to_string()), Some("K".to_string()), None];
        for it in &test_data {
            assert!(matches!(read_block_size(it.as_deref()), Ok(1024)));
        }
    }
}
