// This file is part of the uutils coreutils package.
//
// (c) Derek Chiang <derekchiang93@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) BLOCKSIZE inode inodes ment strs

#[macro_use]
extern crate uucore;

use chrono::prelude::DateTime;
use chrono::Local;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{stderr, Result, Write};
use std::iter;
#[cfg(not(windows))]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};
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

const NAME: &str = "du";
const SUMMARY: &str = "estimate file space usage";
const LONG_HELP: &str = "
 Display  values  are  in  units  of  the  first  available  SIZE from
 --block-size,  and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environ‐
 ment variables.  Otherwise, units default to  1024  bytes  (or  512  if
 POSIXLY_CORRECT is set).

 SIZE  is  an  integer and optional unit (example: 10M is 10*1024*1024).
 Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB, ...  (pow‐
 ers of 1000).
";

// TODO: Support Z & Y (currently limited by size of u64)
const UNITS: [(char, u32); 6] = [('E', 6), ('P', 5), ('T', 4), ('G', 3), ('M', 2), ('K', 1)];

struct Options {
    all: bool,
    program_name: String,
    max_depth: Option<usize>,
    total: bool,
    separate_dirs: bool,
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
    created: u64,
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
            created: metadata.mtime() as u64,
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
            created: windows_time_to_unix_time(metadata.creation_time()),
            accessed: windows_time_to_unix_time(metadata.last_access_time()),
            modified: windows_time_to_unix_time(metadata.last_write_time()),
        })
    }
}

#[cfg(windows)]
// https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html#tymethod.creation_time
// "The returned 64-bit value [...] which represents the number of 100-nanosecond intervals since January 1, 1601 (UTC)."
fn windows_time_to_unix_time(win_time: u64) -> u64 {
    win_time / 10_000_000 - 11_644_473_600
}

#[cfg(windows)]
fn get_size_on_disk(path: &PathBuf) -> u64 {
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
fn get_file_info(path: &PathBuf) -> Option<FileInfo> {
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

fn unit_string_to_number(s: &str) -> Option<u64> {
    let mut offset = 0;
    let mut s_chars = s.chars().rev();

    let (mut ch, multiple) = match s_chars.next() {
        Some('B') | Some('b') => ('B', 1000u64),
        Some(ch) => (ch, 1024u64),
        None => return None,
    };
    if ch == 'B' {
        ch = s_chars.next()?;
        offset += 1;
    }
    ch = ch.to_ascii_uppercase();

    let unit = UNITS
        .iter()
        .rev()
        .find(|&&(unit_ch, _)| unit_ch == ch)
        .map(|&(_, val)| {
            // we found a match, so increment offset
            offset += 1;
            val
        })
        .or_else(|| if multiple == 1024 { Some(0) } else { None })?;

    let number = s[..s.len() - offset].parse::<u64>().ok()?;

    Some(number * multiple.pow(unit))
}

fn translate_to_pure_number(s: &Option<String>) -> Option<u64> {
    match *s {
        Some(ref s) => unit_string_to_number(s),
        None => None,
    }
}

fn read_block_size(s: Option<String>) -> u64 {
    match translate_to_pure_number(&s) {
        Some(v) => v,
        None => {
            if let Some(value) = s {
                show_error!("invalid --block-size argument '{}'", value);
            };

            for env_var in &["DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"] {
                if let Some(quantity) = translate_to_pure_number(&env::var(env_var).ok()) {
                    return quantity;
                }
            }

            if env::var("POSIXLY_CORRECT").is_ok() {
                512
            } else {
                1024
            }
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
                            futures.push(du(this_stat, options, depth + 1, inodes));
                        } else {
                            if this_stat.inode.is_some() {
                                let inode = this_stat.inode.unwrap();
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
                    Err(error) => show_error!("{}", error),
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
        options.max_depth == None || depth < options.max_depth.unwrap()
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
        return format!("0");
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

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let syntax = format!(
        "[OPTION]... [FILE]...
 {0} [OPTION]... --files0-from=F",
        NAME
    );
    let matches = app!(&syntax, SUMMARY, LONG_HELP)
        // In task
        .optflag(
            "a",
            "all",
            " write counts for all files, not just directories",
        )
        // In main
        .optflag(
            "",
            "apparent-size",
            "print apparent sizes,  rather  than  disk  usage
            although  the apparent  size is usually smaller, it may be larger due to holes
            in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like",
        )
        // In main
        .optopt(
            "B",
            "block-size",
            "scale sizes  by  SIZE before printing them.
            E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below.",
            "SIZE",
        )
        // In main
        .optflag(
            "b",
            "bytes",
            "equivalent to '--apparent-size --block-size=1'",
        )
        // In main
        .optflag("c", "total", "produce a grand total")
        // In task
        // opts.optflag("D", "dereference-args", "dereference only symlinks that are listed
        //     on the command line"),
        // In main
        // opts.optopt("", "files0-from", "summarize disk usage of the NUL-terminated file
        //                   names specified in file F;
        //                   If F is - then read names from standard input", "F"),
        // // In task
        // opts.optflag("H", "", "equivalent to --dereference-args (-D)"),
        // In main
        .optflag(
            "h",
            "human-readable",
            "print sizes in human readable format (e.g., 1K 234M 2G)",
        )
        // In main
        .optflag("", "si", "like -h, but use powers of 1000 not 1024")
        // In main
        .optflag("k", "", "like --block-size=1K")
        // In task
        .optflag("l", "count-links", "count sizes many times if hard linked")
        // // In main
        .optflag("m", "", "like --block-size=1M")
        // // In task
        // opts.optflag("L", "dereference", "dereference all symbolic links"),
        // // In task
        // opts.optflag("P", "no-dereference", "don't follow any symbolic links (this is the default)"),
        // // In main
        .optflag(
            "0",
            "null",
            "end each output line with 0 byte rather than newline",
        )
        // In main
        .optflag(
            "S",
            "separate-dirs",
            "do not include size of subdirectories",
        )
        // In main
        .optflag("s", "summarize", "display only a total for each argument")
        // // In task
        // opts.optflag("x", "one-file-system", "skip directories on different file systems"),
        // // In task
        // opts.optopt("X", "exclude-from", "exclude files that match any pattern in FILE", "FILE"),
        // // In task
        // opts.optopt("", "exclude", "exclude files that match PATTERN", "PATTERN"),
        // In main
        .optopt(
            "d",
            "max-depth",
            "print the total for a directory (or file, with --all)
            only if it is N or fewer levels below the command
            line argument;  --max-depth=0 is the same as --summarize",
            "N",
        )
        // In main
        .optflagopt(
            "",
            "time",
            "show time of the last modification of any file in the
            directory, or any of its subdirectories.  If WORD is given, show time as WORD instead
            of modification time: atime, access, use, ctime or status",
            "WORD",
        )
        // In main
        .optopt(
            "",
            "time-style",
            "show times using style STYLE:
            full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'",
            "STYLE",
        )
        .parse(args);

    let summarize = matches.opt_present("summarize");

    let max_depth_str = matches.opt_str("max-depth");
    let max_depth = max_depth_str.as_ref().and_then(|s| s.parse::<usize>().ok());
    match (max_depth_str, max_depth) {
        (Some(ref s), _) if summarize => {
            show_error!("summarizing conflicts with --max-depth={}", *s);
            return 1;
        }
        (Some(ref s), None) => {
            show_error!("invalid maximum depth '{}'", *s);
            return 1;
        }
        (Some(_), Some(_)) | (None, _) => { /* valid */ }
    }

    let options = Options {
        all: matches.opt_present("all"),
        program_name: NAME.to_owned(),
        max_depth,
        total: matches.opt_present("total"),
        separate_dirs: matches.opt_present("S"),
    };

    let strs = if matches.free.is_empty() {
        vec!["./".to_owned()]
    } else {
        matches.free.clone()
    };

    let block_size = read_block_size(matches.opt_str("block-size"));

    let multiplier: u64 = if matches.opt_present("si") {
        1000
    } else {
        1024
    };
    let convert_size_fn = {
        if matches.opt_present("human-readable") || matches.opt_present("si") {
            convert_size_human
        } else if matches.opt_present("b") {
            convert_size_b
        } else if matches.opt_present("k") {
            convert_size_k
        } else if matches.opt_present("m") {
            convert_size_m
        } else {
            convert_size_other
        }
    };
    let convert_size = |size| convert_size_fn(size, multiplier, block_size);

    let time_format_str = match matches.opt_str("time-style") {
        Some(s) => match &s[..] {
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
                    NAME
                );
                return 1;
            }
        },
        None => "%Y-%m-%d %H:%M",
    };

    let line_separator = if matches.opt_present("0") { "\0" } else { "\n" };

    let mut grand_total = 0;
    for path_str in strs {
        let path = PathBuf::from(&path_str);
        match Stat::new(path) {
            Ok(stat) => {
                let mut inodes: HashSet<FileInfo> = HashSet::new();

                let iter = du(stat, &options, 0, &mut inodes);
                let (_, len) = iter.size_hint();
                let len = len.unwrap();
                for (index, stat) in iter.enumerate() {
                    let size = if matches.opt_present("apparent-size") || matches.opt_present("b") {
                        stat.size
                    } else {
                        // C's stat is such that each block is assume to be 512 bytes
                        // See: http://linux.die.net/man/2/stat
                        stat.blocks * 512
                    };
                    if matches.opt_present("time") {
                        let tm = {
                            let secs = {
                                match matches.opt_str("time") {
                                    Some(s) => match &s[..] {
                                        "accessed" => stat.accessed,
                                        "created" => stat.created,
                                        "modified" => stat.modified,
                                        _ => {
                                            show_error!(
                                                "invalid argument 'modified' for '--time'
    Valid arguments are:
      - 'accessed', 'created', 'modified'
    Try '{} --help' for more information.",
                                                NAME
                                            );
                                            return 1;
                                        }
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
                        // path_str.  We add it to the grand total.
                        grand_total += size;
                    }
                }
            }
            Err(_) => {
                show_error!("{}: {}", path_str, "No such file or directory");
            }
        }
    }

    if options.total {
        print!("{}\ttotal", convert_size(grand_total));
        print!("{}", line_separator);
    }

    0
}

#[cfg(test)]
mod test_du {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_translate_to_pure_number() {
        let test_data = [
            (Some("10".to_string()), Some(10)),
            (Some("10K".to_string()), Some(10 * 1024)),
            (Some("5M".to_string()), Some(5 * 1024 * 1024)),
            (Some("900KB".to_string()), Some(900 * 1000)),
            (Some("BAD_STRING".to_string()), None),
        ];
        for it in test_data.into_iter() {
            assert_eq!(translate_to_pure_number(&it.0), it.1);
        }
    }

    #[test]
    fn test_read_block_size() {
        let test_data = [
            (Some("10".to_string()), 10),
            (None, 1024),
            (Some("BAD_STRING".to_string()), 1024),
        ];
        for it in test_data.into_iter() {
            assert_eq!(read_block_size(it.0.clone()), it.1);
        }
    }
}
