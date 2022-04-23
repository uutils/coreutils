// This file is part of the uutils coreutils package.
//
// (c) Fangxu Hu <framlog@gmail.com>
// (c) Sylvestre Ledru <sylvestre@debian.org>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
// spell-checker:ignore itotal iused iavail ipcent pcent tmpfs squashfs lofs
mod blocks;
mod columns;
mod filesystem;
mod table;

use uucore::display::Quotable;
use uucore::error::{UError, UResult, USimpleError};
use uucore::fsext::{read_fs_list, MountInfo};
use uucore::{format_usage, show};

use clap::{crate_version, Arg, ArgMatches, Command};

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::blocks::{block_size_from_matches, BlockSize};
use crate::columns::{Column, ColumnError};
use crate::filesystem::Filesystem;
use crate::table::Table;

static ABOUT: &str = "Show information about the file system on which each FILE resides,\n\
                      or all file systems by default.";
const USAGE: &str = "{} [OPTION]... [FILE]...";

static OPT_HELP: &str = "help";
static OPT_ALL: &str = "all";
static OPT_BLOCKSIZE: &str = "blocksize";
static OPT_TOTAL: &str = "total";
static OPT_HUMAN_READABLE_BINARY: &str = "human-readable-binary";
static OPT_HUMAN_READABLE_DECIMAL: &str = "human-readable-decimal";
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
static OUTPUT_FIELD_LIST: [&str; 12] = [
    "source", "fstype", "itotal", "iused", "iavail", "ipcent", "size", "used", "avail", "pcent",
    "file", "target",
];

/// Parameters that control the behavior of `df`.
///
/// Most of these parameters control which rows and which columns are
/// displayed. The `block_size` determines the units to use when
/// displaying numbers of bytes or inodes.
struct Options {
    show_local_fs: bool,
    show_all_fs: bool,
    block_size: BlockSize,

    /// Optional list of filesystem types to include in the output table.
    ///
    /// If this is not `None`, only filesystems that match one of
    /// these types will be listed.
    include: Option<Vec<String>>,

    /// Optional list of filesystem types to exclude from the output table.
    ///
    /// If this is not `None`, filesystems that match one of these
    /// types will *not* be listed.
    exclude: Option<Vec<String>>,

    /// Whether to show a final row comprising the totals for each column.
    show_total: bool,

    /// Sequence of columns to display in the output table.
    columns: Vec<Column>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            show_local_fs: Default::default(),
            show_all_fs: Default::default(),
            block_size: Default::default(),
            include: Default::default(),
            exclude: Default::default(),
            show_total: Default::default(),
            columns: vec![
                Column::Source,
                Column::Size,
                Column::Used,
                Column::Avail,
                Column::Pcent,
                Column::Target,
            ],
        }
    }
}

#[derive(Debug)]
enum OptionsError {
    InvalidBlockSize,

    /// An error getting the columns to display in the output table.
    ColumnError(ColumnError),

    FilesystemTypeBothSelectedAndExcluded(Vec<String>),
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // TODO This should include the raw string provided as the argument.
            //
            // TODO This needs to vary based on whether `--block-size`
            // or `-B` were provided.
            Self::InvalidBlockSize => write!(f, "invalid --block-size argument"),
            Self::ColumnError(ColumnError::MultipleColumns(s)) => write!(
                f,
                "option --output: field {} used more than once",
                s.quote()
            ),
            Self::FilesystemTypeBothSelectedAndExcluded(types) => {
                for t in types {
                    eprintln!(
                        "{}: file system type {} both selected and excluded",
                        uucore::util_name(),
                        t.quote()
                    );
                }
                Ok(())
            }
        }
    }
}

impl Options {
    /// Convert command-line arguments into [`Options`].
    fn from(matches: &ArgMatches) -> Result<Self, OptionsError> {
        let include = matches.values_of_lossy(OPT_TYPE);
        let exclude = matches.values_of_lossy(OPT_EXCLUDE_TYPE);

        if let (Some(include), Some(exclude)) = (&include, &exclude) {
            if let Some(types) = Self::get_intersected_types(include, exclude) {
                return Err(OptionsError::FilesystemTypeBothSelectedAndExcluded(types));
            }
        }

        Ok(Self {
            show_local_fs: matches.is_present(OPT_LOCAL),
            show_all_fs: matches.is_present(OPT_ALL),
            block_size: block_size_from_matches(matches)
                .map_err(|_| OptionsError::InvalidBlockSize)?,
            include,
            exclude,
            show_total: matches.is_present(OPT_TOTAL),
            columns: Column::from_matches(matches).map_err(OptionsError::ColumnError)?,
        })
    }

    fn get_intersected_types(include: &[String], exclude: &[String]) -> Option<Vec<String>> {
        let mut intersected_types = Vec::new();

        for t in include {
            if exclude.contains(t) {
                intersected_types.push(t.clone());
            }
        }

        (!intersected_types.is_empty()).then(|| intersected_types)
    }
}

/// Whether to display the mount info given the inclusion settings.
fn is_included(mi: &MountInfo, opt: &Options) -> bool {
    // Don't show remote filesystems if `--local` has been given.
    if mi.remote && opt.show_local_fs {
        return false;
    }

    // Don't show pseudo filesystems unless `--all` has been given.
    if mi.dummy && !opt.show_all_fs {
        return false;
    }

    // Don't show filesystems if they have been explicitly excluded.
    if let Some(ref excludes) = opt.exclude {
        if excludes.contains(&mi.fs_type) {
            return false;
        }
    }
    if let Some(ref includes) = opt.include {
        if !includes.contains(&mi.fs_type) {
            return false;
        }
    }

    true
}

/// Whether the mount info in `m2` should be prioritized over `m1`.
///
/// The "lt" in the function name is in analogy to the
/// [`std::cmp::PartialOrd::lt`].
fn mount_info_lt(m1: &MountInfo, m2: &MountInfo) -> bool {
    // let "real" devices with '/' in the name win.
    if m1.dev_name.starts_with('/') && !m2.dev_name.starts_with('/') {
        return false;
    }

    let m1_nearer_root = m1.mount_dir.len() < m2.mount_dir.len();
    // With bind mounts, prefer items nearer the root of the source
    let m2_below_root = !m1.mount_root.is_empty()
        && !m2.mount_root.is_empty()
        && m1.mount_root.len() > m2.mount_root.len();
    // let points towards the root of the device win.
    if m1_nearer_root && !m2_below_root {
        return false;
    }

    // let an entry over-mounted on a new device win, but only when
    // matching an existing mnt point, to avoid problematic
    // replacement when given inaccurate mount lists, seen with some
    // chroot environments for example.
    if m1.dev_name != m2.dev_name && m1.mount_dir == m2.mount_dir {
        return false;
    }

    true
}

/// Whether to prioritize given mount info over all others on the same device.
///
/// This function decides whether the mount info `mi` is better than
/// all others in `previous` that mount the same device as `mi`.
fn is_best(previous: &[MountInfo], mi: &MountInfo) -> bool {
    for seen in previous {
        if seen.dev_id == mi.dev_id && mount_info_lt(mi, seen) {
            return false;
        }
    }
    true
}

/// Keep only the specified subset of [`MountInfo`] instances.
///
/// The `opt` argument specifies a variety of ways of excluding
/// [`MountInfo`] instances; see [`Options`] for more information.
///
/// Finally, if there are duplicate entries, the one with the shorter
/// path is kept.

fn filter_mount_list(vmi: Vec<MountInfo>, opt: &Options) -> Vec<MountInfo> {
    let mut result = vec![];
    for mi in vmi {
        // TODO The running time of the `is_best()` function is linear
        // in the length of `result`. That makes the running time of
        // this loop quadratic in the length of `vmi`. This could be
        // improved by a more efficient implementation of `is_best()`,
        // but `vmi` is probably not very long in practice.
        if is_included(&mi, opt) && is_best(&result, &mi) {
            result.push(mi);
        }
    }
    result
}

/// Get all currently mounted filesystems.
///
/// `opt` excludes certain filesystems from consideration; see
/// [`Options`] for more information.
fn get_all_filesystems(opt: &Options) -> Vec<Filesystem> {
    // The list of all mounted filesystems.
    //
    // Filesystems excluded by the command-line options are
    // not considered.
    let mounts: Vec<MountInfo> = filter_mount_list(read_fs_list(), opt);

    // Convert each `MountInfo` into a `Filesystem`, which contains
    // both the mount information and usage information.
    mounts
        .into_iter()
        .filter_map(|m| Filesystem::new(m, None))
        .filter(|fs| opt.show_all_fs || fs.usage.blocks > 0)
        .collect()
}

/// For each path, get the filesystem that contains that path.
fn get_named_filesystems<P>(paths: &[P], opt: &Options) -> Vec<Filesystem>
where
    P: AsRef<Path>,
{
    // The list of all mounted filesystems.
    //
    // Filesystems marked as `dummy` or of type "lofs" are not
    // considered. The "lofs" filesystem is a loopback
    // filesystem present on Solaris and FreeBSD systems. It
    // is similar to a symbolic link.
    let mounts: Vec<MountInfo> = filter_mount_list(read_fs_list(), opt)
        .into_iter()
        .filter(|mi| mi.fs_type != "lofs" && !mi.dummy)
        .collect();

    let mut result = vec![];

    // this happens if the file system type doesn't exist
    if mounts.is_empty() {
        show!(USimpleError::new(1, "no file systems processed"));
        return result;
    }

    // Convert each path into a `Filesystem`, which contains
    // both the mount information and usage information.
    for path in paths {
        match Filesystem::from_path(&mounts, path) {
            Some(fs) => result.push(fs),
            None => {
                // this happens if specified file system type != file system type of the file
                if path.as_ref().exists() {
                    show!(USimpleError::new(1, "no file systems processed"));
                } else {
                    show!(USimpleError::new(
                        1,
                        format!("{}: No such file or directory", path.as_ref().display())
                    ));
                }
            }
        }
    }
    result
}

#[derive(Debug)]
enum DfError {
    /// A problem while parsing command-line options.
    OptionsError(OptionsError),
}

impl Error for DfError {}

impl UError for DfError {
    fn usage(&self) -> bool {
        matches!(self, Self::OptionsError(OptionsError::ColumnError(_)))
    }
}

impl fmt::Display for DfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OptionsError(e) => e.fmt(f),
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    #[cfg(windows)]
    {
        if matches.is_present(OPT_INODES) {
            println!("{}: doesn't support -i option", uucore::util_name());
            return Ok(());
        }
    }

    let opt = Options::from(&matches).map_err(DfError::OptionsError)?;
    // Get the list of filesystems to display in the output table.
    let filesystems: Vec<Filesystem> = match matches.values_of(OPT_PATHS) {
        None => {
            let filesystems = get_all_filesystems(&opt);

            if filesystems.is_empty() {
                return Err(USimpleError::new(1, "no file systems processed"));
            }

            filesystems
        }
        Some(paths) => {
            let paths: Vec<&str> = paths.collect();
            let filesystems = get_named_filesystems(&paths, &opt);

            // This can happen if paths are given as command-line arguments
            // but none of the paths exist.
            if filesystems.is_empty() {
                return Ok(());
            }

            filesystems
        }
    };

    println!("{}", Table::new(&opt, filesystems));

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_HELP)
                .long(OPT_HELP)
                .help("Print help information."),
        )
        .arg(
            Arg::new(OPT_ALL)
                .short('a')
                .long("all")
                .overrides_with(OPT_ALL)
                .help("include dummy file systems"),
        )
        .arg(
            Arg::new(OPT_BLOCKSIZE)
                .short('B')
                .long("block-size")
                .takes_value(true)
                .overrides_with_all(&[OPT_KILO, OPT_BLOCKSIZE])
                .help(
                    "scale sizes by SIZE before printing them; e.g.\
                    '-BM' prints sizes in units of 1,048,576 bytes",
                ),
        )
        .arg(
            Arg::new(OPT_TOTAL)
                .long("total")
                .overrides_with(OPT_TOTAL)
                .help("produce a grand total"),
        )
        .arg(
            Arg::new(OPT_HUMAN_READABLE_BINARY)
                .short('h')
                .long("human-readable")
                .overrides_with_all(&[OPT_HUMAN_READABLE_DECIMAL, OPT_HUMAN_READABLE_BINARY])
                .help("print sizes in human readable format (e.g., 1K 234M 2G)"),
        )
        .arg(
            Arg::new(OPT_HUMAN_READABLE_DECIMAL)
                .short('H')
                .long("si")
                .overrides_with_all(&[OPT_HUMAN_READABLE_BINARY, OPT_HUMAN_READABLE_DECIMAL])
                .help("likewise, but use powers of 1000 not 1024"),
        )
        .arg(
            Arg::new(OPT_INODES)
                .short('i')
                .long("inodes")
                .overrides_with(OPT_INODES)
                .help("list inode information instead of block usage"),
        )
        .arg(
            Arg::new(OPT_KILO)
                .short('k')
                .help("like --block-size=1K")
                .overrides_with_all(&[OPT_BLOCKSIZE, OPT_KILO]),
        )
        .arg(
            Arg::new(OPT_LOCAL)
                .short('l')
                .long("local")
                .overrides_with(OPT_LOCAL)
                .help("limit listing to local file systems"),
        )
        .arg(
            Arg::new(OPT_NO_SYNC)
                .long("no-sync")
                .overrides_with_all(&[OPT_SYNC, OPT_NO_SYNC])
                .help("do not invoke sync before getting usage info (default)"),
        )
        .arg(
            Arg::new(OPT_OUTPUT)
                .long("output")
                .takes_value(true)
                .min_values(0)
                .require_equals(true)
                .use_value_delimiter(true)
                .multiple_occurrences(true)
                .possible_values(OUTPUT_FIELD_LIST)
                .default_missing_values(&OUTPUT_FIELD_LIST)
                .default_values(&["source", "size", "used", "avail", "pcent", "target"])
                .conflicts_with_all(&[OPT_INODES, OPT_PORTABILITY, OPT_PRINT_TYPE])
                .help(
                    "use the output format defined by FIELD_LIST,\
                     or print all fields if FIELD_LIST is omitted.",
                ),
        )
        .arg(
            Arg::new(OPT_PORTABILITY)
                .short('P')
                .long("portability")
                .overrides_with(OPT_PORTABILITY)
                .help("use the POSIX output format"),
        )
        .arg(
            Arg::new(OPT_SYNC)
                .long("sync")
                .overrides_with_all(&[OPT_NO_SYNC, OPT_SYNC])
                .help("invoke sync before getting usage info"),
        )
        .arg(
            Arg::new(OPT_TYPE)
                .short('t')
                .long("type")
                .allow_invalid_utf8(true)
                .takes_value(true)
                .multiple_occurrences(true)
                .help("limit listing to file systems of type TYPE"),
        )
        .arg(
            Arg::new(OPT_PRINT_TYPE)
                .short('T')
                .long("print-type")
                .overrides_with(OPT_PRINT_TYPE)
                .help("print file system type"),
        )
        .arg(
            Arg::new(OPT_EXCLUDE_TYPE)
                .short('x')
                .long("exclude-type")
                .allow_invalid_utf8(true)
                .takes_value(true)
                .use_value_delimiter(true)
                .multiple_occurrences(true)
                .help("limit listing to file systems not of type TYPE"),
        )
        .arg(Arg::new(OPT_PATHS).multiple_occurrences(true))
}

#[cfg(test)]
mod tests {

    mod mount_info_lt {

        use crate::mount_info_lt;
        use uucore::fsext::MountInfo;

        /// Instantiate a [`MountInfo`] with the given fields.
        fn mount_info(dev_name: &str, mount_root: &str, mount_dir: &str) -> MountInfo {
            MountInfo {
                dev_id: String::new(),
                dev_name: String::from(dev_name),
                fs_type: String::new(),
                mount_dir: String::from(mount_dir),
                mount_option: String::new(),
                mount_root: String::from(mount_root),
                remote: false,
                dummy: false,
            }
        }

        #[test]
        fn test_absolute() {
            // Prefer device name "/dev/foo" over "dev_foo".
            let m1 = mount_info("/dev/foo", "/", "/mnt/bar");
            let m2 = mount_info("dev_foo", "/", "/mnt/bar");
            assert!(!mount_info_lt(&m1, &m2));
        }

        #[test]
        fn test_shorter() {
            // Prefer mount directory "/mnt/bar" over "/mnt/bar/baz"...
            let m1 = mount_info("/dev/foo", "/", "/mnt/bar");
            let m2 = mount_info("/dev/foo", "/", "/mnt/bar/baz");
            assert!(!mount_info_lt(&m1, &m2));

            // ..but prefer mount root "/root" over "/".
            let m1 = mount_info("/dev/foo", "/root", "/mnt/bar");
            let m2 = mount_info("/dev/foo", "/", "/mnt/bar/baz");
            assert!(mount_info_lt(&m1, &m2));
        }

        #[test]
        fn test_over_mounted() {
            // Prefer the earlier entry if the devices are different but
            // the mount directory is the same.
            let m1 = mount_info("/dev/foo", "/", "/mnt/baz");
            let m2 = mount_info("/dev/bar", "/", "/mnt/baz");
            assert!(!mount_info_lt(&m1, &m2));
        }
    }

    mod is_best {

        use crate::is_best;
        use uucore::fsext::MountInfo;

        /// Instantiate a [`MountInfo`] with the given fields.
        fn mount_info(dev_id: &str, mount_dir: &str) -> MountInfo {
            MountInfo {
                dev_id: String::from(dev_id),
                dev_name: String::new(),
                fs_type: String::new(),
                mount_dir: String::from(mount_dir),
                mount_option: String::new(),
                mount_root: String::new(),
                remote: false,
                dummy: false,
            }
        }

        #[test]
        fn test_empty() {
            let m = mount_info("0", "/mnt/bar");
            assert!(is_best(&[], &m));
        }

        #[test]
        fn test_different_dev_id() {
            let m1 = mount_info("0", "/mnt/bar");
            let m2 = mount_info("1", "/mnt/bar");
            assert!(is_best(&[m1.clone()], &m2));
            assert!(is_best(&[m2], &m1));
        }

        #[test]
        fn test_same_dev_id() {
            // There are several conditions under which a `MountInfo` is
            // considered "better" than the others, we're just checking
            // one condition in this test.
            let m1 = mount_info("0", "/mnt/bar");
            let m2 = mount_info("0", "/mnt/bar/baz");
            assert!(!is_best(&[m1.clone()], &m2));
            assert!(is_best(&[m2], &m1));
        }
    }

    mod is_included {

        use crate::{is_included, Options};
        use uucore::fsext::MountInfo;

        /// Instantiate a [`MountInfo`] with the given fields.
        fn mount_info(fs_type: &str, mount_dir: &str, remote: bool, dummy: bool) -> MountInfo {
            MountInfo {
                dev_id: String::new(),
                dev_name: String::new(),
                fs_type: String::from(fs_type),
                mount_dir: String::from(mount_dir),
                mount_option: String::new(),
                mount_root: String::new(),
                remote,
                dummy,
            }
        }

        #[test]
        fn test_remote_included() {
            let opt = Default::default();
            let m = mount_info("ext4", "/mnt/foo", true, false);
            assert!(is_included(&m, &opt));
        }

        #[test]
        fn test_remote_excluded() {
            let opt = Options {
                show_local_fs: true,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", true, false);
            assert!(!is_included(&m, &opt));
        }

        #[test]
        fn test_dummy_included() {
            let opt = Options {
                show_all_fs: true,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, true);
            assert!(is_included(&m, &opt));
        }

        #[test]
        fn test_dummy_excluded() {
            let opt = Default::default();
            let m = mount_info("ext4", "/mnt/foo", false, true);
            assert!(!is_included(&m, &opt));
        }

        #[test]
        fn test_exclude_match() {
            let exclude = Some(vec![String::from("ext4")]);
            let opt = Options {
                exclude,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, false);
            assert!(!is_included(&m, &opt));
        }

        #[test]
        fn test_exclude_no_match() {
            let exclude = Some(vec![String::from("tmpfs")]);
            let opt = Options {
                exclude,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, false);
            assert!(is_included(&m, &opt));
        }

        #[test]
        fn test_include_match() {
            let include = Some(vec![String::from("ext4")]);
            let opt = Options {
                include,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, false);
            assert!(is_included(&m, &opt));
        }

        #[test]
        fn test_include_no_match() {
            let include = Some(vec![String::from("tmpfs")]);
            let opt = Options {
                include,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, false);
            assert!(!is_included(&m, &opt));
        }

        #[test]
        fn test_include_and_exclude_match_neither() {
            let include = Some(vec![String::from("tmpfs")]);
            let exclude = Some(vec![String::from("squashfs")]);
            let opt = Options {
                include,
                exclude,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, false);
            assert!(!is_included(&m, &opt));
        }

        #[test]
        fn test_include_and_exclude_match_exclude() {
            let include = Some(vec![String::from("tmpfs")]);
            let exclude = Some(vec![String::from("ext4")]);
            let opt = Options {
                include,
                exclude,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, false);
            assert!(!is_included(&m, &opt));
        }

        #[test]
        fn test_include_and_exclude_match_include() {
            let include = Some(vec![String::from("ext4")]);
            let exclude = Some(vec![String::from("squashfs")]);
            let opt = Options {
                include,
                exclude,
                ..Default::default()
            };
            let m = mount_info("ext4", "/mnt/foo", false, false);
            assert!(is_included(&m, &opt));
        }
    }

    mod filter_mount_list {

        use crate::filter_mount_list;

        #[test]
        fn test_empty() {
            let opt = Default::default();
            let mount_infos = vec![];
            assert!(filter_mount_list(mount_infos, &opt).is_empty());
        }
    }
}
