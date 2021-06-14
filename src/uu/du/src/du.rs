//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Derek Chiang <derekchiang93@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

pub mod app;

#[macro_use]
extern crate uucore;

use chrono::prelude::DateTime;
use chrono::Local;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::env;
use std::fs;
#[cfg(not(windows))]
use std::fs::Metadata;
use std::io::{stderr, ErrorKind, Result, Write};
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
use std::time::{Duration, UNIX_EPOCH};
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

use crate::app::get_app;
use crate::app::options;

// TODO: Support Z & Y (currently limited by size of u64)
const UNITS: [(char, u32); 6] = [('E', 6), ('P', 5), ('T', 4), ('G', 3), ('M', 2), ('K', 1)];

struct Options {
    all: bool,
    program_name: String,
    max_depth: Option<usize>,
    total: bool,
    separate_dirs: bool,
    one_file_system: bool,
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
    inode: Option<FileInfo>,
    created: Option<u64>,
    accessed: u64,
    modified: u64,
}

impl Stat {
    fn new(path: PathBuf) -> Result<Stat> {
        let metadata = fs::symlink_metadata(&path)?;

        #[cfg(not(windows))]
        let file_info = FileInfo {
            file_id: metadata.ino() as u128,
            dev_id: metadata.dev(),
        };
        #[cfg(not(windows))]
        return Ok(Stat {
            path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            blocks: metadata.blocks() as u64,
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
        Ok(Stat {
            path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            blocks: size_on_disk / 1024 * 2,
            inode: file_info,
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

fn read_block_size(s: Option<&str>) -> usize {
    if let Some(s) = s {
        parse_size(s)
            .unwrap_or_else(|e| crash!(1, "{}", format_error_message(e, s, options::BLOCK_SIZE)))
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
                safe_writeln!(
                    stderr(),
                    "{}: cannot read directory ‘{}‘: {}",
                    options.program_name,
                    my_stat.path.display(),
                    e
                );
                return Box::new(iter::once(my_stat));
            }
        };

        for f in read {
            match f {
                Ok(entry) => match Stat::new(entry.path()) {
                    Ok(this_stat) => {
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
                            if let Some(inode) = this_stat.inode {
                                if inodes.contains(&inode) {
                                    continue;
                                }
                                inodes.insert(inode);
                            }
                            my_stat.size += this_stat.size;
                            my_stat.blocks += this_stat.blocks;
                            if options.all {
                                stats.push(this_stat);
                            }
                        }
                    }
                    Err(error) => match error.kind() {
                        ErrorKind::PermissionDenied => {
                            let description = format!(
                                "cannot access '{}'",
                                entry
                                    .path()
                                    .as_os_str()
                                    .to_str()
                                    .unwrap_or("<Un-printable path>")
                            );
                            let error_message = "Permission denied";
                            show_error_custom_description!(description, "{}", error_message)
                        }
                        _ => show_error!("{}", error),
                    },
                },
                Err(error) => show_error!("{}", error),
            }
        }
    }

    stats.extend(futures.into_iter().flatten().rev().filter(|stat| {
        if !options.separate_dirs && stat.path.parent().unwrap() == my_stat.path {
            my_stat.size += stat.size;
            my_stat.blocks += stat.blocks;
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

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... [FILE]...
    {0} [OPTION]... --files0-from=F",
        executable!()
    )
}

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let summarize = matches.is_present(options::SUMMARIZE);

    let max_depth_str = matches.value_of(options::MAX_DEPTH);
    let max_depth = max_depth_str.as_ref().and_then(|s| s.parse::<usize>().ok());
    match (max_depth_str, max_depth) {
        (Some(s), _) if summarize => {
            show_error!("summarizing conflicts with --max-depth={}", s);
            return 1;
        }
        (Some(s), None) => {
            show_error!("invalid maximum depth '{}'", s);
            return 1;
        }
        (Some(_), Some(_)) | (None, _) => { /* valid */ }
    }

    let options = Options {
        all: matches.is_present(options::ALL),
        program_name: executable!().to_owned(),
        max_depth,
        total: matches.is_present(options::TOTAL),
        separate_dirs: matches.is_present(options::SEPARATE_DIRS),
        one_file_system: matches.is_present(options::ONE_FILE_SYSTEM),
    };

    let files = match matches.value_of(options::FILE) {
        Some(_) => matches.values_of(options::FILE).unwrap().collect(),
        None => {
            vec!["./"] // TODO: gnu `du` doesn't use trailing "/" here
        }
    };

    let block_size = u64::try_from(read_block_size(matches.value_of(options::BLOCK_SIZE))).unwrap();

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
    let convert_size = |size| convert_size_fn(size, multiplier, block_size);

    let time_format_str = match matches.value_of("time-style") {
        Some(s) => match s {
            "full-iso" => "%Y-%m-%d %H:%M:%S.%f %z",
            "long-iso" => "%Y-%m-%d %H:%M",
            "iso" => "%Y-%m-%d",
            _ => {
                show_error!(
                    "invalid argument '{}' for 'time style'
Valid arguments are:
- 'full-iso'
- 'long-iso'
- 'iso'
Try '{} --help' for more information.",
                    s,
                    executable!()
                );
                return 1;
            }
        },
        None => "%Y-%m-%d %H:%M",
    };

    let line_separator = if matches.is_present(options::NULL) {
        "\0"
    } else {
        "\n"
    };

    let mut grand_total = 0;
    for path_string in files {
        let path = PathBuf::from(&path_string);
        match Stat::new(path) {
            Ok(stat) => {
                let mut inodes: HashSet<FileInfo> = HashSet::new();

                let iter = du(stat, &options, 0, &mut inodes);
                let (_, len) = iter.size_hint();
                let len = len.unwrap();
                for (index, stat) in iter.enumerate() {
                    let size = if matches.is_present(options::APPARENT_SIZE)
                        || matches.is_present(options::BYTES)
                    {
                        stat.size
                    } else {
                        // C's stat is such that each block is assume to be 512 bytes
                        // See: http://linux.die.net/man/2/stat
                        stat.blocks * 512
                    };
                    if matches.is_present(options::TIME) {
                        let tm = {
                            let secs = {
                                match matches.value_of(options::TIME) {
                                    Some(s) => match s {
                                        "ctime" | "status" => stat.modified,
                                        "access" | "atime" | "use" => stat.accessed,
                                        "birth" | "creation" => {
                                            if let Some(time) = stat.created {
                                                time
                                            } else {
                                                show_error!(
                                                    "Invalid argument ‘{}‘ for --time.
‘birth‘ and ‘creation‘ arguments are not supported on this platform.",
                                                    s
                                                );
                                                return 1;
                                            }
                                        }
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
                            print!(
                                "{}\t{}\t{}{}",
                                convert_size(size),
                                time_str,
                                stat.path.display(),
                                line_separator
                            );
                        }
                    } else if !summarize || index == len - 1 {
                        print!(
                            "{}\t{}{}",
                            convert_size(size),
                            stat.path.display(),
                            line_separator
                        );
                    }
                    if options.total && index == (len - 1) {
                        // The last element will be the total size of the the path under
                        // path_string.  We add it to the grand total.
                        grand_total += size;
                    }
                }
            }
            Err(_) => {
                show_error!("{}: {}", path_string, "No such file or directory");
            }
        }
    }

    if options.total {
        print!("{}\ttotal", convert_size(grand_total));
        print!("{}", line_separator);
    }

    0
}

fn format_error_message(error: ParseSizeError, s: &str, option: &str) -> String {
    // NOTE:
    // GNU's du echos affected flag, -B or --block-size (-t or --threshold), depending user's selection
    // GNU's du does distinguish between "invalid (suffix in) argument"
    match error {
        ParseSizeError::ParseFailure(_) => format!("invalid --{} argument '{}'", option, s),
        ParseSizeError::SizeTooBig(_) => format!("--{} argument '{}' too large", option, s),
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
        for it in test_data.iter() {
            assert_eq!(read_block_size(it.0.as_deref()), it.1);
        }
    }
}
