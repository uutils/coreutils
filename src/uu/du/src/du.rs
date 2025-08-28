// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, ArgMatches, Command, builder::PossibleValue};
use glob::Pattern;
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::Metadata;
use std::fs::{self, DirEntry, File};
use std::io::{BufRead, BufReader, stdout};
#[cfg(not(windows))]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;
use thiserror::Error;
use uucore::display::{Quotable, print_verbatim};
use uucore::error::{FromIo, UError, UResult, USimpleError, set_exit_code};
use uucore::fsext::{MetadataTimeField, metadata_get_time};
use uucore::line_ending::LineEnding;
use uucore::translate;

use uucore::LocalizedCommand;
use uucore::parser::parse_glob;
use uucore::parser::parse_size::{ParseSizeError, parse_size_u64};
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::time::{FormatSystemTimeFallback, format, format_system_time};
use uucore::{format_usage, show, show_error, show_warning};
#[cfg(windows)]
use windows_sys::Win32::Foundation::HANDLE;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ID_128, FILE_ID_INFO, FILE_STANDARD_INFO, FileIdInfo, FileStandardInfo,
    GetFileInformationByHandleEx,
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
}

struct TraversalOptions {
    all: bool,
    separate_dirs: bool,
    one_file_system: bool,
    dereference: Deref,
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
    time: Option<MetadataTimeField>,
    time_format: String,
    line_ending: LineEnding,
    summarize: bool,
    total_text: String,
}

#[derive(PartialEq, Clone)]
enum Deref {
    All,
    Args(Vec<PathBuf>),
    None,
}

#[derive(Clone)]
enum SizeFormat {
    HumanDecimal,
    HumanBinary,
    BlockSize(u64),
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct FileInfo {
    file_id: u128,
    dev_id: u64,
}

struct Stat {
    path: PathBuf,
    size: u64,
    blocks: u64,
    inodes: u64,
    inode: Option<FileInfo>,
    metadata: Metadata,
}

impl Stat {
    fn new(
        path: &Path,
        dir_entry: Option<&DirEntry>,
        options: &TraversalOptions,
    ) -> std::io::Result<Self> {
        // Determine whether to dereference (follow) the symbolic link
        let should_dereference = match &options.dereference {
            Deref::All => true,
            Deref::Args(paths) => paths.contains(&path.to_path_buf()),
            Deref::None => false,
        };

        let metadata = if should_dereference {
            // Get metadata, following symbolic links if necessary
            fs::metadata(path)
        } else if let Some(dir_entry) = dir_entry {
            // Get metadata directly from the DirEntry, which is faster on Windows
            dir_entry.metadata()
        } else {
            // Get metadata from the filesystem without following symbolic links
            fs::symlink_metadata(path)
        }?;

        let file_info = get_file_info(path, &metadata);
        let blocks = get_blocks(path, &metadata);

        Ok(Self {
            path: path.to_path_buf(),
            size: if metadata.is_dir() { 0 } else { metadata.len() },
            blocks,
            inodes: 1,
            inode: file_info,
            metadata,
        })
    }
}

#[cfg(not(windows))]
fn get_blocks(_path: &Path, metadata: &Metadata) -> u64 {
    metadata.blocks()
}

#[cfg(windows)]
fn get_blocks(path: &Path, _metadata: &Metadata) -> u64 {
    let mut size_on_disk = 0;

    // bind file so it stays in scope until end of function
    // if it goes out of scope the handle below becomes invalid
    let Ok(file) = File::open(path) else {
        return size_on_disk; // opening directories will fail
    };

    unsafe {
        let mut file_info: FILE_STANDARD_INFO = core::mem::zeroed();
        let file_info_ptr: *mut FILE_STANDARD_INFO = &raw mut file_info;

        let success = GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileStandardInfo,
            file_info_ptr.cast(),
            size_of::<FILE_STANDARD_INFO>() as u32,
        );

        if success != 0 {
            size_on_disk = file_info.AllocationSize as u64;
        }
    }

    size_on_disk / 1024 * 2
}

#[cfg(not(windows))]
fn get_file_info(_path: &Path, metadata: &Metadata) -> Option<FileInfo> {
    Some(FileInfo {
        file_id: metadata.ino() as u128,
        dev_id: metadata.dev(),
    })
}

#[cfg(windows)]
fn get_file_info(path: &Path, _metadata: &Metadata) -> Option<FileInfo> {
    let mut result = None;

    let Ok(file) = File::open(path) else {
        return result;
    };

    unsafe {
        let mut file_info: FILE_ID_INFO = core::mem::zeroed();
        let file_info_ptr: *mut FILE_ID_INFO = &raw mut file_info;

        let success = GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileIdInfo,
            file_info_ptr.cast(),
            size_of::<FILE_ID_INFO>() as u32,
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

// this takes `my_stat` to avoid having to stat files multiple times.
#[allow(clippy::cognitive_complexity)]
fn du(
    mut my_stat: Stat,
    options: &TraversalOptions,
    depth: usize,
    seen_inodes: &mut HashSet<FileInfo>,
    print_tx: &mpsc::Sender<UResult<StatPrintInfo>>,
) -> Result<Stat, Box<mpsc::SendError<UResult<StatPrintInfo>>>> {
    if my_stat.metadata.is_dir() {
        let read = match fs::read_dir(&my_stat.path) {
            Ok(read) => read,
            Err(e) => {
                print_tx.send(Err(e.map_err_context(
                    || translate!("du-error-cannot-read-directory", "path" => my_stat.path.quote()),
                )))?;
                return Ok(my_stat);
            }
        };

        'file_loop: for f in read {
            match f {
                Ok(entry) => {
                    match Stat::new(&entry.path(), Some(&entry), options) {
                        Ok(this_stat) => {
                            // We have an exclude list
                            for pattern in &options.excludes {
                                // Look at all patterns with both short and long paths
                                // if we have 'du foo' but search to exclude 'foo/bar'
                                // we need the full path
                                if pattern.matches(&this_stat.path.to_string_lossy())
                                    || pattern.matches(&entry.file_name().into_string().unwrap())
                                {
                                    // if the directory is ignored, leave early
                                    if options.verbose {
                                        println!(
                                            "{}",
                                            translate!("du-verbose-ignored", "path" => this_stat.path.quote())
                                        );
                                    }
                                    // Go to the next file
                                    continue 'file_loop;
                                }
                            }

                            if let Some(inode) = this_stat.inode {
                                // Check if the inode has been seen before and if we should skip it
                                if seen_inodes.contains(&inode)
                                    && (!options.count_links || !options.all)
                                {
                                    // If `count_links` is enabled and `all` is not, increment the inode count
                                    if options.count_links && !options.all {
                                        my_stat.inodes += 1;
                                    }
                                    // Skip further processing for this inode
                                    continue;
                                }
                                // Mark this inode as seen
                                seen_inodes.insert(inode);
                            }

                            if this_stat.metadata.is_dir() {
                                if options.one_file_system {
                                    if let (Some(this_inode), Some(my_inode)) =
                                        (this_stat.inode, my_stat.inode)
                                    {
                                        if this_inode.dev_id != my_inode.dev_id {
                                            continue;
                                        }
                                    }
                                }

                                let this_stat =
                                    du(this_stat, options, depth + 1, seen_inodes, print_tx)?;

                                if !options.separate_dirs {
                                    my_stat.size += this_stat.size;
                                    my_stat.blocks += this_stat.blocks;
                                    my_stat.inodes += this_stat.inodes;
                                }
                                print_tx.send(Ok(StatPrintInfo {
                                    stat: this_stat,
                                    depth: depth + 1,
                                }))?;
                            } else {
                                my_stat.size += this_stat.size;
                                my_stat.blocks += this_stat.blocks;
                                my_stat.inodes += 1;
                                if options.all {
                                    print_tx.send(Ok(StatPrintInfo {
                                        stat: this_stat,
                                        depth: depth + 1,
                                    }))?;
                                }
                            }
                        }
                        Err(e) => print_tx.send(Err(e.map_err_context(
                            || translate!("du-error-cannot-access", "path" => entry.path().quote()),
                        )))?,
                    }
                }
                Err(error) => print_tx.send(Err(error.into()))?,
            }
        }
    }

    Ok(my_stat)
}

#[derive(Debug, Error)]
enum DuError {
    #[error("{}", translate!("du-error-invalid-max-depth", "depth" => _0.quote()))]
    InvalidMaxDepthArg(String),

    #[error("{}", translate!("du-error-summarize-depth-conflict", "depth" => _0.maybe_quote()))]
    SummarizeDepthConflict(String),

    #[error("{}", translate!("du-error-invalid-time-style", "style" => _0.quote(), "help" => uucore::execution_phrase()))]
    InvalidTimeStyleArg(String),

    #[error("{}", translate!("du-error-invalid-glob", "error" => _0))]
    InvalidGlob(String),
}

impl UError for DuError {
    fn code(&self) -> i32 {
        match self {
            Self::InvalidMaxDepthArg(_)
            | Self::SummarizeDepthConflict(_)
            | Self::InvalidTimeStyleArg(_)
            | Self::InvalidGlob(_) => 1,
        }
    }
}

/// Read a file and return each line in a vector of String
fn file_as_vec(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);

    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

/// Given the `--exclude-from` and/or `--exclude` arguments, returns the globset lists
/// to ignore the files
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
            println!(
                "{}",
                translate!("du-verbose-adding-to-exclude-list", "pattern" => f.clone())
            );
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
                            .is_some_and(|threshold| threshold.should_exclude(size))
                            && self
                                .max_depth
                                .is_none_or(|max_depth| stat_info.depth <= max_depth)
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
            print!("{}\t{}", self.convert_size(grand_total), self.total_text);
            print!("{}", self.line_ending);
        }

        Ok(())
    }

    fn convert_size(&self, size: u64) -> String {
        match self.size_format {
            SizeFormat::HumanDecimal => uucore::format::human::human_readable(
                size,
                uucore::format::human::SizeFormat::Decimal,
            ),
            SizeFormat::HumanBinary => uucore::format::human::human_readable(
                size,
                uucore::format::human::SizeFormat::Binary,
            ),
            SizeFormat::BlockSize(block_size) => {
                if self.inodes {
                    // we ignore block size (-B) with --inodes
                    size.to_string()
                } else {
                    size.div_ceil(block_size).to_string()
                }
            }
        }
    }

    fn print_stat(&self, stat: &Stat, size: u64) -> UResult<()> {
        print!("{}\t", self.convert_size(size));

        if let Some(md_time) = &self.time {
            if let Some(time) = metadata_get_time(&stat.metadata, *md_time) {
                format_system_time(
                    &mut stdout(),
                    time,
                    &self.time_format,
                    FormatSystemTimeFallback::IntegerError,
                )?;
                print!("\t");
            } else {
                print!("???\t");
            }
        }

        print_verbatim(&stat.path).unwrap();
        print!("{}", self.line_ending);

        Ok(())
    }
}

/// Read file paths from the specified file, separated by null characters
fn read_files_from(file_name: &OsStr) -> Result<Vec<PathBuf>, std::io::Error> {
    let reader: Box<dyn BufRead> = if file_name == "-" {
        // Read from standard input
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        // First, check if the file_name is a directory
        let path = PathBuf::from(file_name);
        if path.is_dir() {
            return Err(std::io::Error::other(
                translate!("du-error-read-error-is-directory", "file" => file_name.to_string_lossy()),
            ));
        }

        // Attempt to open the file and handle the error if it does not exist
        match File::open(file_name) {
            Ok(file) => Box::new(BufReader::new(file)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(std::io::Error::other(
                    translate!("du-error-cannot-open-for-reading", "file" => file_name.to_string_lossy()),
                ));
            }
            Err(e) => return Err(e),
        }
    };

    let mut paths = Vec::new();

    for (i, line) in reader.split(b'\0').enumerate() {
        let path = line?;

        if path.is_empty() {
            let line_number = i + 1;
            show_error!(
                "{}",
                translate!("du-error-invalid-zero-length-file-name", "file" => file_name.to_string_lossy(), "line" => line_number)
            );
            set_exit_code(1);
        } else {
            let p = PathBuf::from(&*uucore::os_str_from_bytes(&path).unwrap());
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
    }

    Ok(paths)
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from_localized(args);

    let summarize = matches.get_flag(options::SUMMARIZE);

    let count_links = matches.get_flag(options::COUNT_LINKS);

    let max_depth = parse_depth(
        matches
            .get_one::<String>(options::MAX_DEPTH)
            .map(|s| s.as_str()),
        summarize,
    )?;

    let files = if let Some(file_from) = matches.get_one::<OsString>(options::FILES0_FROM) {
        if file_from == "-" && matches.get_one::<OsString>(options::FILE).is_some() {
            return Err(std::io::Error::other(
                translate!("du-error-extra-operand-with-files0-from",
                    "file" => matches
                        .get_one::<OsString>(options::FILE)
                        .unwrap()
                        .to_string_lossy()
                        .quote()
                ),
            )
            .into());
        }

        read_files_from(file_from)?
    } else if let Some(files) = matches.get_many::<OsString>(options::FILE) {
        let files = files.map(PathBuf::from);
        if count_links {
            files.collect()
        } else {
            // Deduplicate while preserving order
            let mut seen = HashSet::new();
            files
                .filter(|path| seen.insert(path.clone()))
                .collect::<Vec<_>>()
        }
    } else {
        vec![PathBuf::from(".")]
    };

    let time = matches.contains_id(options::TIME).then(|| {
        matches
            .get_one::<String>(options::TIME)
            .map_or(MetadataTimeField::Modification, |s| s.as_str().into())
    });

    let size_format = if matches.get_flag(options::HUMAN_READABLE) {
        SizeFormat::HumanBinary
    } else if matches.get_flag(options::SI) {
        SizeFormat::HumanDecimal
    } else if matches.get_flag(options::BYTES) {
        SizeFormat::BlockSize(1)
    } else if matches.get_flag(options::BLOCK_SIZE_1K) {
        SizeFormat::BlockSize(1024)
    } else if matches.get_flag(options::BLOCK_SIZE_1M) {
        SizeFormat::BlockSize(1024 * 1024)
    } else {
        let block_size_str = matches.get_one::<String>(options::BLOCK_SIZE);
        let block_size = read_block_size(block_size_str.map(AsRef::as_ref))?;
        if block_size == 0 {
            return Err(std::io::Error::other(translate!("du-error-invalid-block-size-argument", "option" => options::BLOCK_SIZE, "value" => block_size_str.map_or("???BUG", |v| v).quote()))
            .into());
        }
        SizeFormat::BlockSize(block_size)
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
        count_links,
        verbose: matches.get_flag(options::VERBOSE),
        excludes: build_exclude_patterns(&matches)?,
    };

    let time_format = if time.is_some() {
        parse_time_style(matches.get_one::<String>("time-style"))?
    } else {
        format::LONG_ISO.to_string()
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
        time_format,
        line_ending: LineEnding::from_zero_flag(matches.get_flag(options::NULL)),
        total_text: translate!("du-total"),
    };

    if stat_printer.inodes
        && (matches.get_flag(options::APPARENT_SIZE) || matches.get_flag(options::BYTES))
    {
        show_warning!(
            "{}",
            translate!("du-warning-apparent-size-ineffective-with-inodes")
        );
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
                        println!(
                            "{}",
                            translate!("du-verbose-ignored", "path" => path_string.quote())
                        );
                    }
                    continue 'loop_file;
                }
            }
        }

        // Check existence of path provided in argument
        if let Ok(stat) = Stat::new(&path, None, &traversal_options) {
            // Kick off the computation of disk usage from the initial path
            let mut seen_inodes: HashSet<FileInfo> = HashSet::new();
            if let Some(inode) = stat.inode {
                seen_inodes.insert(inode);
            }
            let stat = du(stat, &traversal_options, 0, &mut seen_inodes, &print_tx)
                .map_err(|e| USimpleError::new(1, e.to_string()))?;

            print_tx
                .send(Ok(StatPrintInfo { stat, depth: 0 }))
                .map_err(|e| USimpleError::new(1, e.to_string()))?;
        } else {
            print_tx
                .send(Err(USimpleError::new(
                    1,
                    translate!("du-error-cannot-access-no-such-file", "path" => path.to_string_lossy().quote()),
                )))
                .map_err(|e| USimpleError::new(1, e.to_string()))?;
        }
    }

    drop(print_tx);

    printing_thread
        .join()
        .map_err(|_| USimpleError::new(1, translate!("du-error-printing-thread-panicked")))??;

    Ok(())
}

// Parse --time-style argument, falling back to environment variable if necessary.
fn parse_time_style(s: Option<&String>) -> UResult<String> {
    let s = match s {
        Some(s) => Some(s.into()),
        None => {
            match env::var("TIME_STYLE") {
                // Per GNU manual, strip `posix-` if present, ignore anything after a newline if
                // the string starts with +, and ignore "locale".
                Ok(s) => {
                    let s = s.strip_prefix("posix-").unwrap_or(s.as_str());
                    let s = match s.chars().next().unwrap() {
                        '+' => s.split('\n').next().unwrap(),
                        _ => s,
                    };
                    match s {
                        "locale" => None,
                        _ => Some(s.to_string()),
                    }
                }
                Err(_) => None,
            }
        }
    };
    match s {
        Some(s) => match s.as_ref() {
            "full-iso" => Ok(format::FULL_ISO.to_string()),
            "long-iso" => Ok(format::LONG_ISO.to_string()),
            "iso" => Ok(format::ISO.to_string()),
            _ => match s.chars().next().unwrap() {
                '+' => Ok(s[1..].to_string()),
                _ => Err(DuError::InvalidTimeStyleArg(s).into()),
            },
        },
        None => Ok(format::LONG_ISO.to_string()),
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
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("du-about"))
        .after_help(translate!("du-after-help"))
        .override_usage(format_usage(&translate!("du-usage")))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("du-help-print-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help(translate!("du-help-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::APPARENT_SIZE)
                .long(options::APPARENT_SIZE)
                .help(translate!("du-help-apparent-size"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BLOCK_SIZE)
                .short('B')
                .long(options::BLOCK_SIZE)
                .value_name("SIZE")
                .help(translate!("du-help-block-size")),
        )
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long("bytes")
                .help(translate!("du-help-bytes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TOTAL)
                .long("total")
                .short('c')
                .help(translate!("du-help-total"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MAX_DEPTH)
                .short('d')
                .long("max-depth")
                .value_name("N")
                .help(translate!("du-help-max-depth")),
        )
        .arg(
            Arg::new(options::HUMAN_READABLE)
                .long("human-readable")
                .short('h')
                .help(translate!("du-help-human-readable"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INODES)
                .long(options::INODES)
                .help(translate!("du-help-inodes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BLOCK_SIZE_1K)
                .short('k')
                .help(translate!("du-help-block-size-1k"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COUNT_LINKS)
                .short('l')
                .long("count-links")
                .help(translate!("du-help-count-links"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help(translate!("du-help-dereference"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEREFERENCE_ARGS)
                .short('D')
                .visible_short_alias('H')
                .long(options::DEREFERENCE_ARGS)
                .help(translate!("du-help-dereference-args"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE)
                .short('P')
                .long(options::NO_DEREFERENCE)
                .help(translate!("du-help-no-dereference"))
                .overrides_with(options::DEREFERENCE)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BLOCK_SIZE_1M)
                .short('m')
                .help(translate!("du-help-block-size-1m"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NULL)
                .short('0')
                .long("null")
                .help(translate!("du-help-null"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SEPARATE_DIRS)
                .short('S')
                .long("separate-dirs")
                .help(translate!("du-help-separate-dirs"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SUMMARIZE)
                .short('s')
                .long("summarize")
                .help(translate!("du-help-summarize"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SI)
                .long(options::SI)
                .help(translate!("du-help-si"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONE_FILE_SYSTEM)
                .short('x')
                .long(options::ONE_FILE_SYSTEM)
                .help(translate!("du-help-one-file-system"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::THRESHOLD)
                .short('t')
                .long(options::THRESHOLD)
                .value_name("SIZE")
                .num_args(1)
                .allow_hyphen_values(true)
                .help(translate!("du-help-threshold")),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long("verbose")
                .help(translate!("du-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::EXCLUDE)
                .long(options::EXCLUDE)
                .value_name("PATTERN")
                .help(translate!("du-help-exclude"))
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::EXCLUDE_FROM)
                .short('X')
                .long("exclude-from")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .help(translate!("du-help-exclude-from"))
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::FILES0_FROM)
                .long("files0-from")
                .value_name("FILE")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString))
                .help(translate!("du-help-files0-from"))
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .value_name("WORD")
                .require_equals(true)
                .num_args(0..)
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("atime").alias("access").alias("use"),
                    PossibleValue::new("ctime").alias("status"),
                    PossibleValue::new("creation").alias("birth"),
                ]))
                .help(translate!("du-help-time")),
        )
        .arg(
            Arg::new(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .value_name("STYLE")
                .help(translate!("du-help-time-style")),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(clap::value_parser!(OsString))
                .action(ArgAction::Append),
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
            translate!("du-error-invalid-suffix", "option" => option, "value" => s.quote())
        }
        ParseSizeError::ParseFailure(_) | ParseSizeError::PhysicalMem(_) => {
            translate!("du-error-invalid-argument", "option" => option, "value" => s.quote())
        }
        ParseSizeError::SizeTooBig(_) => {
            translate!("du-error-argument-too-large", "option" => option, "value" => s.quote())
        }
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
