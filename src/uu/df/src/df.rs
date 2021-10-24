// This file is part of the uutils coreutils package.
//
// (c) Fangxu Hu <framlog@gmail.com>
// (c) Sylvestre Ledru <sylvestre@debian.org>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

use uucore::error::UError;
use uucore::error::UResult;
#[cfg(unix)]
use uucore::fsext::statfs_fn;
use uucore::fsext::{read_fs_list, FsUsage, MountInfo};

use clap::{crate_version, App, Arg};

use number_prefix::NumberPrefix;
use std::cell::Cell;
use std::collections::HashMap;
use std::collections::HashSet;

use std::error::Error;
#[cfg(unix)]
use std::ffi::CString;
use std::fmt::Display;
#[cfg(unix)]
use std::mem;

#[cfg(windows)]
use std::path::Path;

static ABOUT: &str = "Show information about the file system on which each FILE resides,\n\
                      or all file systems by default.";

static OPT_ALL: &str = "all";
static OPT_BLOCKSIZE: &str = "blocksize";
static OPT_DIRECT: &str = "direct";
static OPT_TOTAL: &str = "total";
static OPT_HUMAN_READABLE: &str = "human-readable";
static OPT_HUMAN_READABLE_2: &str = "human-readable-2";
static OPT_INODES: &str = "inodes";
static OPT_KILO: &str = "kilo";
static OPT_LOCAL: &str = "local";
static OPT_NO_SYNC: &str = "no-sync";
static OPT_OUTPUT: &str = "output";
static OPT_PATHS: &str = "paths";
static OPT_PORTABILITY: &str = "portability";
static OPT_SYNC: &str = "sync";
static OPT_TYPE: &str = "type";
static OPT_PRINT_TYPE: &str = "print-type";
static OPT_EXCLUDE_TYPE: &str = "exclude-type";

/// Store names of file systems as a selector.
/// Note: `exclude` takes priority over `include`.
struct FsSelector {
    include: HashSet<String>,
    exclude: HashSet<String>,
}

struct Options {
    show_local_fs: bool,
    show_all_fs: bool,
    show_listed_fs: bool,
    show_fs_type: bool,
    show_inode_instead: bool,
    // block_size: usize,
    human_readable_base: i64,
    fs_selector: FsSelector,
}

#[derive(Debug, Clone)]
struct Filesystem {
    mount_info: MountInfo,
    usage: FsUsage,
}

fn usage() -> String {
    format!("{0} [OPTION]... [FILE]...", uucore::execution_phrase())
}

impl FsSelector {
    fn new() -> FsSelector {
        FsSelector {
            include: HashSet::new(),
            exclude: HashSet::new(),
        }
    }

    #[inline(always)]
    fn include(&mut self, fs_type: String) {
        self.include.insert(fs_type);
    }

    #[inline(always)]
    fn exclude(&mut self, fs_type: String) {
        self.exclude.insert(fs_type);
    }

    fn should_select(&self, fs_type: &str) -> bool {
        if self.exclude.contains(fs_type) {
            return false;
        }
        self.include.is_empty() || self.include.contains(fs_type)
    }
}

impl Options {
    fn new() -> Options {
        Options {
            show_local_fs: false,
            show_all_fs: false,
            show_listed_fs: false,
            show_fs_type: false,
            show_inode_instead: false,
            // block_size: match env::var("BLOCKSIZE") {
            //     Ok(size) => size.parse().unwrap(),
            //     Err(_) => 512,
            // },
            human_readable_base: -1,
            fs_selector: FsSelector::new(),
        }
    }
}

impl Filesystem {
    // TODO: resolve uuid in `mount_info.dev_name` if exists
    fn new(mount_info: MountInfo) -> Option<Filesystem> {
        let _stat_path = if !mount_info.mount_dir.is_empty() {
            mount_info.mount_dir.clone()
        } else {
            #[cfg(unix)]
            {
                mount_info.dev_name.clone()
            }
            #[cfg(windows)]
            {
                // On windows, we expect the volume id
                mount_info.dev_id.clone()
            }
        };
        #[cfg(unix)]
        unsafe {
            let path = CString::new(_stat_path).unwrap();
            let mut statvfs = mem::zeroed();
            if statfs_fn(path.as_ptr(), &mut statvfs) < 0 {
                None
            } else {
                Some(Filesystem {
                    mount_info,
                    usage: FsUsage::new(statvfs),
                })
            }
        }
        #[cfg(windows)]
        Some(Filesystem {
            mount_info,
            usage: FsUsage::new(Path::new(&_stat_path)),
        })
    }
}

fn filter_mount_list(vmi: Vec<MountInfo>, paths: &[String], opt: &Options) -> Vec<MountInfo> {
    vmi.into_iter()
        .filter_map(|mi| {
            if (mi.remote && opt.show_local_fs)
                || (mi.dummy && !opt.show_all_fs && !opt.show_listed_fs)
                || !opt.fs_selector.should_select(&mi.fs_type)
            {
                None
            } else {
                if paths.is_empty() {
                    // No path specified
                    return Some((mi.dev_id.clone(), mi));
                }
                if paths.contains(&mi.mount_dir) {
                    // One or more paths have been provided
                    Some((mi.dev_id.clone(), mi))
                } else {
                    // Not a path we want to see
                    None
                }
            }
        })
        .fold(
            HashMap::<String, Cell<MountInfo>>::new(),
            |mut acc, (id, mi)| {
                #[allow(clippy::map_entry)]
                {
                    if acc.contains_key(&id) {
                        let seen = acc[&id].replace(mi.clone());
                        let target_nearer_root = seen.mount_dir.len() > mi.mount_dir.len();
                        // With bind mounts, prefer items nearer the root of the source
                        let source_below_root = !seen.mount_root.is_empty()
                            && !mi.mount_root.is_empty()
                            && seen.mount_root.len() < mi.mount_root.len();
                        // let "real" devices with '/' in the name win.
                        if (!mi.dev_name.starts_with('/') || seen.dev_name.starts_with('/'))
                            // let points towards the root of the device win.
                            && (!target_nearer_root || source_below_root)
                            // let an entry over-mounted on a new device win...
                            && (seen.dev_name == mi.dev_name
                            /* ... but only when matching an existing mnt point,
                            to avoid problematic replacement when given
                            inaccurate mount lists, seen with some chroot
                            environments for example.  */
                            || seen.mount_dir != mi.mount_dir)
                        {
                            acc[&id].replace(seen);
                        }
                    } else {
                        acc.insert(id, Cell::new(mi));
                    }
                    acc
                }
            },
        )
        .into_iter()
        .map(|ent| ent.1.into_inner())
        .collect::<Vec<_>>()
}

/// Convert `value` to a human readable string based on `base`.
/// e.g. It returns 1G when value is 1 * 1024 * 1024 * 1024 and base is 1024.
/// Note: It returns `value` if `base` isn't positive.
fn human_readable(value: u64, base: i64) -> UResult<String> {
    let base_str = match base {
        d if d < 0 => value.to_string(),

        // ref: [Binary prefix](https://en.wikipedia.org/wiki/Binary_prefix) @@ <https://archive.is/cnwmF>
        // ref: [SI/metric prefix](https://en.wikipedia.org/wiki/Metric_prefix) @@ <https://archive.is/QIuLj>
        1000 => match NumberPrefix::decimal(value as f64) {
            NumberPrefix::Standalone(bytes) => bytes.to_string(),
            NumberPrefix::Prefixed(prefix, bytes) => format!("{:.1}{}", bytes, prefix.symbol()),
        },

        1024 => match NumberPrefix::binary(value as f64) {
            NumberPrefix::Standalone(bytes) => bytes.to_string(),
            NumberPrefix::Prefixed(prefix, bytes) => format!("{:.1}{}", bytes, prefix.symbol()),
        },

        _ => return Err(DfError::InvalidBaseValue(base.to_string()).into()),
    };

    Ok(base_str)
}

fn use_size(free_size: u64, total_size: u64) -> String {
    if total_size == 0 {
        return String::from("-");
    }
    return format!(
        "{:.0}%",
        100f64 - 100f64 * (free_size as f64 / total_size as f64)
    );
}

#[derive(Debug)]
enum DfError {
    InvalidBaseValue(String),
}

impl Display for DfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DfError::InvalidBaseValue(s) => write!(f, "Internal error: Unknown base value {}", s),
        }
    }
}

impl Error for DfError {}

impl UError for DfError {
    fn code(&self) -> i32 {
        match self {
            DfError::InvalidBaseValue(_) => 1,
        }
    }
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();
    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let paths: Vec<String> = matches
        .values_of(OPT_PATHS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    #[cfg(windows)]
    {
        if matches.is_present(OPT_INODES) {
            println!("{}: doesn't support -i option", uucore::util_name());
            return Ok(());
        }
    }

    let mut opt = Options::new();
    if matches.is_present(OPT_LOCAL) {
        opt.show_local_fs = true;
    }
    if matches.is_present(OPT_ALL) {
        opt.show_all_fs = true;
    }
    if matches.is_present(OPT_INODES) {
        opt.show_inode_instead = true;
    }
    if matches.is_present(OPT_PRINT_TYPE) {
        opt.show_fs_type = true;
    }
    if matches.is_present(OPT_HUMAN_READABLE) {
        opt.human_readable_base = 1024;
    }
    if matches.is_present(OPT_HUMAN_READABLE_2) {
        opt.human_readable_base = 1000;
    }
    for fs_type in matches.values_of_lossy(OPT_TYPE).unwrap_or_default() {
        opt.fs_selector.include(fs_type.to_owned());
    }
    for fs_type in matches
        .values_of_lossy(OPT_EXCLUDE_TYPE)
        .unwrap_or_default()
    {
        opt.fs_selector.exclude(fs_type.to_owned());
    }

    let fs_list = filter_mount_list(read_fs_list(), &paths, &opt)
        .into_iter()
        .filter_map(Filesystem::new)
        .filter(|fs| fs.usage.blocks != 0 || opt.show_all_fs || opt.show_listed_fs)
        .collect::<Vec<_>>();

    // set headers
    let mut header = vec!["Filesystem"];
    if opt.show_fs_type {
        header.push("Type");
    }
    header.extend_from_slice(&if opt.show_inode_instead {
        // spell-checker:disable-next-line
        ["Inodes", "Iused", "IFree", "IUses%"]
    } else {
        [
            if opt.human_readable_base == -1 {
                "1k-blocks"
            } else {
                "Size"
            },
            "Used",
            "Available",
            "Use%",
        ]
    });
    if cfg!(target_os = "macos") && !opt.show_inode_instead {
        header.insert(header.len() - 1, "Capacity");
    }
    header.push("Mounted on");

    for (idx, title) in header.iter().enumerate() {
        if idx == 0 || idx == header.len() - 1 {
            print!("{0: <16} ", title);
        } else if opt.show_fs_type && idx == 1 {
            print!("{0: <5} ", title);
        } else if idx == header.len() - 2 {
            print!("{0: >5} ", title);
        } else {
            print!("{0: >12} ", title);
        }
    }
    println!();
    for fs in fs_list.iter() {
        print!("{0: <16} ", fs.mount_info.dev_name);
        if opt.show_fs_type {
            print!("{0: <5} ", fs.mount_info.fs_type);
        }
        if opt.show_inode_instead {
            print!(
                "{0: >12} ",
                human_readable(fs.usage.files, opt.human_readable_base)?
            );
            print!(
                "{0: >12} ",
                human_readable(fs.usage.files - fs.usage.ffree, opt.human_readable_base)?
            );
            print!(
                "{0: >12} ",
                human_readable(fs.usage.ffree, opt.human_readable_base)?
            );
            print!(
                "{0: >5} ",
                format!(
                    "{0:.1}%",
                    100f64 - 100f64 * (fs.usage.ffree as f64 / fs.usage.files as f64)
                )
            );
        } else {
            let total_size = fs.usage.blocksize * fs.usage.blocks;
            let free_size = fs.usage.blocksize * fs.usage.bfree;
            print!(
                "{0: >12} ",
                human_readable(total_size, opt.human_readable_base)?
            );
            print!(
                "{0: >12} ",
                human_readable(total_size - free_size, opt.human_readable_base)?
            );
            print!(
                "{0: >12} ",
                human_readable(free_size, opt.human_readable_base)?
            );
            if cfg!(target_os = "macos") {
                let used = fs.usage.blocks - fs.usage.bfree;
                let blocks = used + fs.usage.bavail;
                print!("{0: >12} ", use_size(used, blocks));
            }
            print!("{0: >5} ", use_size(free_size, total_size));
        }
        print!("{0: <16}", fs.mount_info.mount_dir);
        println!();
    }

    Ok(())
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_ALL)
                .short("a")
                .long("all")
                .help("include dummy file systems"),
        )
        .arg(
            Arg::with_name(OPT_BLOCKSIZE)
                .short("B")
                .long("block-size")
                .takes_value(true)
                .help(
                    "scale sizes by SIZE before printing them; e.g.\
                     '-BM' prints sizes in units of 1,048,576 bytes",
                ),
        )
        .arg(
            Arg::with_name(OPT_DIRECT)
                .long("direct")
                .help("show statistics for a file instead of mount point"),
        )
        .arg(
            Arg::with_name(OPT_TOTAL)
                .long("total")
                .help("produce a grand total"),
        )
        .arg(
            Arg::with_name(OPT_HUMAN_READABLE)
                .short("h")
                .long("human-readable")
                .conflicts_with(OPT_HUMAN_READABLE_2)
                .help("print sizes in human readable format (e.g., 1K 234M 2G)"),
        )
        .arg(
            Arg::with_name(OPT_HUMAN_READABLE_2)
                .short("H")
                .long("si")
                .conflicts_with(OPT_HUMAN_READABLE)
                .help("likewise, but use powers of 1000 not 1024"),
        )
        .arg(
            Arg::with_name(OPT_INODES)
                .short("i")
                .long("inodes")
                .help("list inode information instead of block usage"),
        )
        .arg(
            Arg::with_name(OPT_KILO)
                .short("k")
                .help("like --block-size=1K"),
        )
        .arg(
            Arg::with_name(OPT_LOCAL)
                .short("l")
                .long("local")
                .help("limit listing to local file systems"),
        )
        .arg(
            Arg::with_name(OPT_NO_SYNC)
                .long("no-sync")
                .conflicts_with(OPT_SYNC)
                .help("do not invoke sync before getting usage info (default)"),
        )
        .arg(
            Arg::with_name(OPT_OUTPUT)
                .long("output")
                .takes_value(true)
                .use_delimiter(true)
                .help(
                    "use the output format defined by FIELD_LIST,\
                     or print all fields if FIELD_LIST is omitted.",
                ),
        )
        .arg(
            Arg::with_name(OPT_PORTABILITY)
                .short("P")
                .long("portability")
                .help("use the POSIX output format"),
        )
        .arg(
            Arg::with_name(OPT_SYNC)
                .long("sync")
                .conflicts_with(OPT_NO_SYNC)
                .help("invoke sync before getting usage info"),
        )
        .arg(
            Arg::with_name(OPT_TYPE)
                .short("t")
                .long("type")
                .takes_value(true)
                .use_delimiter(true)
                .help("limit listing to file systems of type TYPE"),
        )
        .arg(
            Arg::with_name(OPT_PRINT_TYPE)
                .short("T")
                .long("print-type")
                .help("print file system type"),
        )
        .arg(
            Arg::with_name(OPT_EXCLUDE_TYPE)
                .short("x")
                .long("exclude-type")
                .takes_value(true)
                .use_delimiter(true)
                .help("limit listing to file systems not of type TYPE"),
        )
        .arg(Arg::with_name(OPT_PATHS).multiple(true))
        .help("Filesystem(s) to list")
}
