//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Derek Chiang <derekchiang93@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use chrono::prelude::DateTime;
use chrono::Local;
use clap::{crate_version, Arg, ArgMatches, Command};
use std::collections::HashSet;
use std::env;
use std::fs;
#[cfg(not(windows))]
use std::fs::Metadata;
use std::io::{ErrorKind, Result};
use std::iter;
#[cfg(not(windows))]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, UNIX_EPOCH};
use std::{error::Error, fmt::Display};
use uucore::display::{print_verbatim, Quotable};
use uucore::error::{UError, UResult};
use uucore::format_usage;
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::InvalidEncodingHandling;
#[cfg(windows)]
use winapi::shared::minwindef::{DWORD, LPVOID};
#[cfg(windows)]
use winapi::um::fileapi::{FILE_ID_INFO, FILE_STANDARD_INFO};
#[cfg(windows)]
use winapi::um::minwinbase::{FileIdInfo, FileStandardInfo};
#[cfg(windows)]
use winapi::um::winbase::GetFileInformationByHandleEx;
#[cfg(windows)]
use winapi::um::winnt::{FILE_ID_128, ULONGLONG};

mod options {
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
    pub const INODES: &str = "inodes";
    pub const FILE: &str = "FILE";
}

const SUMMARY: &str = "estimate file space usage";
const LONG_HELP: &str = "
Display values are in units of the first available SIZE from --block-size,
and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environment variables.
Otherwise, units default to 1024 bytes (or 512 if POSIXLY_CORRECT is set).

SIZE is an integer and optional unit (example: 10M is 10*1024*1024).
Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB,... (powers
of 1000).
";
const USAGE: &str = "\
    {} [OPTION]... [FILE]...
    {} [OPTION]... --files0-from=F";

// TODO: Support Z & Y (currently limited by size of u64)
const UNITS: [(char, u32); 6] = [('E', 6), ('P', 5), ('T', 4), ('G', 3), ('M', 2), ('K', 1)];

struct Options {
    all: bool,
    util_name: String,
    max_depth: Option<usize>,
    total: bool,
    separate_dirs: bool,
    one_file_system: bool,
    dereference: bool,
    inodes: bool,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct FileInfo {
    file_id: u128,
    dev_id: u64,
}

struct Stat {
    path: PathBuf,
    is_dir: bool,
    size: u64,
    blocks: u64,
    inodes: u64,
    inode: Option<FileInfo>,
    created: Option<u64>,
    accessed: u64,
    modified: u64,
}

impl Stat {
    fn new(path: PathBuf, options: &Options) -> Result<Self> {
        let metadata = if options.dereference {
            fs::metadata(&path)?
        } else {
            fs::symlink_metadata(&path)?
        };

        #[cfg(not(windows))]
        let file_info = FileInfo {
            file_id: metadata.ino() as u128,
            dev_id: metadata.dev(),
        };
        #[cfg(not(windows))]
        return Ok(Self {
            path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            blocks: metadata.blocks() as u64,
            inodes: 1,
            inode: Some(file_info),
            created: birth_u64(&metadata),
            accessed: metadata.atime() as u64,
            modified: metadata.mtime() as u64,
        });

        #[cfg(windows)]
        let size_on_disk = get_size_on_disk(&path);
        #[cfg(windows)]
        let file_info = get_file_info(&path);
        #[cfg(windows)]
        Ok(Self {
            path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            blocks: size_on_disk / 1024 * 2,
            inode: file_info,
            inodes: 1,
            created: windows_creation_time_to_unix_time(metadata.creation_time()),
            accessed: windows_time_to_unix_time(metadata.last_access_time()),
            modified: windows_time_to_unix_time(metadata.last_write_time()),
        })
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
        .map(|e| e.as_secs() as u64)
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

    let handle = file.as_raw_handle();

    unsafe {
        let mut file_info: FILE_STANDARD_INFO = core::mem::zeroed();
        let file_info_ptr: *mut FILE_STANDARD_INFO = &mut file_info;

        let success = GetFileInformationByHandleEx(
            handle,
            FileStandardInfo,
            file_info_ptr as LPVOID,
            std::mem::size_of::<FILE_STANDARD_INFO>() as DWORD,
        );

        if success != 0 {
            size_on_disk = *file_info.AllocationSize.QuadPart() as u64;
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

    let handle = file.as_raw_handle();

    unsafe {
        let mut file_info: FILE_ID_INFO = core::mem::zeroed();
        let file_info_ptr: *mut FILE_ID_INFO = &mut file_info;

        let success = GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            file_info_ptr as LPVOID,
            std::mem::size_of::<FILE_ID_INFO>() as DWORD,
        );

        if success != 0 {
            result = Some(FileInfo {
                file_id: std::mem::transmute::<FILE_ID_128, u128>(file_info.FileId),
                dev_id: std::mem::transmute::<ULONGLONG, u64>(file_info.VolumeSerialNumber),
            });
        }
    }

    result
}

fn read_block_size(s: Option<&str>) -> u64 {
    if let Some(s) = s {
        parse_size(s)
            .unwrap_or_else(|e| crash!(1, "{}", format_error_message(&e, s, options::BLOCK_SIZE)))
    } else {
        for env_var in &["DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"] {
            if let Ok(env_size) = env::var(env_var) {
                if let Ok(v) = parse_size(&env_size) {
                    return v;
                }
            }
        }
        if env::var("POSIXLY_CORRECT").is_ok() {
            512
        } else {
            1024
        }
    }
}

fn choose_size(matches: &ArgMatches, stat: &Stat) -> u64 {
    if matches.is_present(options::INODES) {
        stat.inodes
    } else if matches.is_present(options::APPARENT_SIZE) || matches.is_present(options::BYTES) {
        stat.size
    } else {
        // The st_blocks field indicates the number of blocks allocated to the file, 512-byte units.
        // See: http://linux.die.net/man/2/stat
        stat.blocks * 512
    }
}

// this takes `my_stat` to avoid having to stat files multiple times.
// XXX: this should use the impl Trait return type when it is stabilized
fn du(
    mut my_stat: Stat,
    options: &Options,
    depth: usize,
    inodes: &mut HashSet<FileInfo>,
) -> Box<dyn DoubleEndedIterator<Item = Stat>> {
    let mut stats = vec![];
    let mut futures = vec![];

    if my_stat.is_dir {
        let read = match fs::read_dir(&my_stat.path) {
            Ok(read) => read,
            Err(e) => {
                eprintln!(
                    "{}: cannot read directory {}: {}",
                    options.util_name,
                    my_stat.path.quote(),
                    e
                );
                return Box::new(iter::once(my_stat));
            }
        };

        for f in read {
            match f {
                Ok(entry) => match Stat::new(entry.path(), options) {
                    Ok(this_stat) => {
                        if let Some(inode) = this_stat.inode {
                            if inodes.contains(&inode) {
                                continue;
                            }
                            inodes.insert(inode);
                        }
                        if this_stat.is_dir {
                            if options.one_file_system {
                                if let (Some(this_inode), Some(my_inode)) =
                                    (this_stat.inode, my_stat.inode)
                                {
                                    if this_inode.dev_id != my_inode.dev_id {
                                        continue;
                                    }
                                }
                            }
                            futures.push(du(this_stat, options, depth + 1, inodes));
                        } else {
                            my_stat.size += this_stat.size;
                            my_stat.blocks += this_stat.blocks;
                            my_stat.inodes += 1;
                            if options.all {
                                stats.push(this_stat);
                            }
                        }
                    }
                    Err(error) => match error.kind() {
                        ErrorKind::PermissionDenied => {
                            let description = format!("cannot access {}", entry.path().quote());
                            let error_message = "Permission denied";
                            show_error_custom_description!(description, "{}", error_message);
                        }
                        _ => show_error!("cannot access {}: {}", entry.path().quote(), error),
                    },
                },
                Err(error) => show_error!("{}", error),
            }
        }
    }

    stats.extend(futures.into_iter().flatten().filter(|stat| {
        if !options.separate_dirs && stat.path.parent().unwrap() == my_stat.path {
            my_stat.size += stat.size;
            my_stat.blocks += stat.blocks;
            my_stat.inodes += stat.inodes;
        }
        options
            .max_depth
            .map_or(true, |max_depth| depth < max_depth)
    }));
    stats.push(my_stat);
    Box::new(stats.into_iter())
}

fn convert_size_human(size: u64, multiplier: u64, _block_size: u64) -> String {
    for &(unit, power) in &UNITS {
        let limit = multiplier.pow(power);
        if size >= limit {
            return format!("{:.1}{}", (size as f64) / (limit as f64), unit);
        }
    }
    if size == 0 {
        return "0".to_string();
    }
    format!("{}B", size)
}

fn convert_size_b(size: u64, _multiplier: u64, _block_size: u64) -> String {
    format!("{}", ((size as f64) / (1_f64)).ceil())
}

fn convert_size_k(size: u64, multiplier: u64, _block_size: u64) -> String {
    format!("{}", ((size as f64) / (multiplier as f64)).ceil())
}

fn convert_size_m(size: u64, multiplier: u64, _block_size: u64) -> String {
    format!(
        "{}",
        ((size as f64) / ((multiplier * multiplier) as f64)).ceil()
    )
}

fn convert_size_other(size: u64, _multiplier: u64, block_size: u64) -> String {
    format!("{}", ((size as f64) / (block_size as f64)).ceil())
}

#[derive(Debug)]
enum DuError {
    InvalidMaxDepthArg(String),
    SummarizeDepthConflict(String),
    InvalidTimeStyleArg(String),
    InvalidTimeArg(String),
}

impl Display for DuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuError::InvalidMaxDepthArg(s) => write!(f, "invalid maximum depth {}", s.quote()),
            DuError::SummarizeDepthConflict(s) => {
                write!(
                    f,
                    "summarizing conflicts with --max-depth={}",
                    s.maybe_quote()
                )
            }
            DuError::InvalidTimeStyleArg(s) => write!(
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
            DuError::InvalidTimeArg(s) => write!(
                f,
                "Invalid argument {} for --time.
'birth' and 'creation' arguments are not supported on this platform.",
                s.quote()
            ),
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
            | Self::InvalidTimeArg(_) => 1,
        }
    }
}

#[uucore::main]
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let summarize = matches.is_present(options::SUMMARIZE);

    let max_depth = parse_depth(matches.value_of(options::MAX_DEPTH), summarize)?;

    let options = Options {
        all: matches.is_present(options::ALL),
        util_name: uucore::util_name().to_owned(),
        max_depth,
        total: matches.is_present(options::TOTAL),
        separate_dirs: matches.is_present(options::SEPARATE_DIRS),
        one_file_system: matches.is_present(options::ONE_FILE_SYSTEM),
        dereference: matches.is_present(options::DEREFERENCE),
        inodes: matches.is_present(options::INODES),
    };

    let files = match matches.value_of(options::FILE) {
        Some(_) => matches.values_of(options::FILE).unwrap().collect(),
        None => vec!["."],
    };

    if options.inodes
        && (matches.is_present(options::APPARENT_SIZE) || matches.is_present(options::BYTES))
    {
        show_warning!("options --apparent-size and -b are ineffective with --inodes");
    }

    let block_size = read_block_size(matches.value_of(options::BLOCK_SIZE));

    let threshold = matches.value_of(options::THRESHOLD).map(|s| {
        Threshold::from_str(s)
            .unwrap_or_else(|e| crash!(1, "{}", format_error_message(&e, s, options::THRESHOLD)))
    });

    let multiplier: u64 = if matches.is_present(options::SI) {
        1000
    } else {
        1024
    };
    let convert_size_fn = {
        if matches.is_present(options::HUMAN_READABLE) || matches.is_present(options::SI) {
            convert_size_human
        } else if matches.is_present(options::BYTES) {
            convert_size_b
        } else if matches.is_present(options::BLOCK_SIZE_1K) {
            convert_size_k
        } else if matches.is_present(options::BLOCK_SIZE_1M) {
            convert_size_m
        } else {
            convert_size_other
        }
    };
    let convert_size = |size: u64| {
        if options.inodes {
            size.to_string()
        } else {
            convert_size_fn(size, multiplier, block_size)
        }
    };

    let time_format_str = parse_time_style(matches.value_of("time-style"))?;

    let line_separator = if matches.is_present(options::NULL) {
        "\0"
    } else {
        "\n"
    };

    let mut grand_total = 0;
    for path_string in files {
        let path = PathBuf::from(&path_string);
        match Stat::new(path, &options) {
            Ok(stat) => {
                let mut inodes: HashSet<FileInfo> = HashSet::new();
                if let Some(inode) = stat.inode {
                    inodes.insert(inode);
                }
                let iter = du(stat, &options, 0, &mut inodes);
                let (_, len) = iter.size_hint();
                let len = len.unwrap();
                for (index, stat) in iter.enumerate() {
                    let size = choose_size(&matches, &stat);

                    if threshold.map_or(false, |threshold| threshold.should_exclude(size)) {
                        continue;
                    }

                    if matches.is_present(options::TIME) {
                        let tm = {
                            let secs = {
                                match matches.value_of(options::TIME) {
                                    Some(s) => match s {
                                        "ctime" | "status" => stat.modified,
                                        "access" | "atime" | "use" => stat.accessed,
                                        "birth" | "creation" => stat
                                            .created
                                            .ok_or_else(|| DuError::InvalidTimeArg(s.into()))?,
                                        // below should never happen as clap already restricts the values.
                                        _ => unreachable!("Invalid field for --time"),
                                    },
                                    None => stat.modified,
                                }
                            };
                            DateTime::<Local>::from(UNIX_EPOCH + Duration::from_secs(secs))
                        };
                        if !summarize || index == len - 1 {
                            let time_str = tm.format(time_format_str).to_string();
                            print!("{}\t{}\t", convert_size(size), time_str);
                            print_verbatim(stat.path).unwrap();
                            print!("{}", line_separator);
                        }
                    } else if !summarize || index == len - 1 {
                        print!("{}\t", convert_size(size));
                        print_verbatim(stat.path).unwrap();
                        print!("{}", line_separator);
                    }
                    if options.total && index == (len - 1) {
                        // The last element will be the total size of the the path under
                        // path_string.  We add it to the grand total.
                        grand_total += size;
                    }
                }
            }
            Err(_) => {
                show_error!(
                    "{}: {}",
                    path_string.maybe_quote(),
                    "No such file or directory"
                );
            }
        }
    }

    if options.total {
        print!("{}\ttotal", convert_size(grand_total));
        print!("{}", line_separator);
    }

    Ok(())
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

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(SUMMARY)
        .after_help(LONG_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL)
                .short('a')
                .long(options::ALL)
                .help("write counts for all files, not just directories"),
        )
        .arg(
            Arg::new(options::APPARENT_SIZE)
                .long(options::APPARENT_SIZE)
                .help(
                    "print apparent sizes,  rather  than  disk  usage \
                    although  the apparent  size is usually smaller, it may be larger due to holes \
                    in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like"
                )
        )
        .arg(
            Arg::new(options::BLOCK_SIZE)
                .short('B')
                .long(options::BLOCK_SIZE)
                .value_name("SIZE")
                .help(
                    "scale sizes  by  SIZE before printing them. \
                    E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below."
                )
        )
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long("bytes")
                .help("equivalent to '--apparent-size --block-size=1'")
        )
        .arg(
            Arg::new(options::TOTAL)
                .long("total")
                .short('c')
                .help("produce a grand total")
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
        )
        .arg(
            Arg::new(options::INODES)
                .long(options::INODES)
                .help(
                    "list inode usage information instead of block usage like --block-size=1K"
                )
        )
        .arg(
            Arg::new(options::BLOCK_SIZE_1K)
                .short('k')
                .help("like --block-size=1K")
        )
        .arg(
            Arg::new(options::COUNT_LINKS)
                .short('l')
                .long("count-links")
                .help("count sizes many times if hard linked")
        )
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help("dereference all symbolic links")
        )
        // .arg(
        //     Arg::new("no-dereference")
        //         .short('P')
        //         .long("no-dereference")
        //         .help("don't follow any symbolic links (this is the default)")
        // )
        .arg(
            Arg::new(options::BLOCK_SIZE_1M)
                .short('m')
                .help("like --block-size=1M")
        )
        .arg(
            Arg::new(options::NULL)
                .short('0')
                .long("null")
                .help("end each output line with 0 byte rather than newline")
        )
        .arg(
            Arg::new(options::SEPARATE_DIRS)
                .short('S')
                .long("separate-dirs")
                .help("do not include size of subdirectories")
        )
        .arg(
            Arg::new(options::SUMMARIZE)
                .short('s')
                .long("summarize")
                .help("display only a total for each argument")
        )
        .arg(
            Arg::new(options::SI)
                .long(options::SI)
                .help("like -h, but use powers of 1000 not 1024")
        )
        .arg(
            Arg::new(options::ONE_FILE_SYSTEM)
                .short('x')
                .long(options::ONE_FILE_SYSTEM)
                .help("skip directories on different file systems")
        )
        .arg(
            Arg::new(options::THRESHOLD)
                .short('t')
                .long(options::THRESHOLD)
                .value_name("SIZE")
                .number_of_values(1)
                .allow_hyphen_values(true)
                .help("exclude entries smaller than SIZE if positive, \
                          or entries greater than SIZE if negative")
        )
        // .arg(
        //     Arg::new("")
        //         .short('x')
        //         .long("exclude-from")
        //         .value_name("FILE")
        //         .help("exclude files that match any pattern in FILE")
        // )
        // .arg(
        //     Arg::new("exclude")
        //         .long("exclude")
        //         .value_name("PATTERN")
        //         .help("exclude files that match PATTERN")
        // )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .value_name("WORD")
                .require_equals(true)
                .min_values(0)
                .possible_values(&["atime", "access", "use", "ctime", "status", "birth", "creation"])
                .help(
                    "show time of the last modification of any file in the \
                    directory, or any of its subdirectories.  If WORD is given, show time as WORD instead \
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
                .multiple_occurrences(true)
        )
}

#[derive(Clone, Copy)]
enum Threshold {
    Lower(u64),
    Upper(u64),
}

impl FromStr for Threshold {
    type Err = ParseSizeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let offset = if s.starts_with(&['-', '+'][..]) { 1 } else { 0 };

        let size = parse_size(&s[offset..])?;

        if s.starts_with('-') {
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
    // GNU's du does distinguish between "invalid (suffix in) argument"
    match error {
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
        let test_data = [
            (Some("1024".to_string()), 1024),
            (Some("K".to_string()), 1024),
            (None, 1024),
        ];
        for it in &test_data {
            assert_eq!(read_block_size(it.0.as_deref()), it.1);
        }
    }
}
