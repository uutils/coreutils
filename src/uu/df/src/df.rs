// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore itotal iused iavail ipcent pcent tmpfs squashfs lofs
mod blocks;
mod columns;
mod filesystem;
mod table;

use blocks::HumanReadable;
use clap::builder::ValueParser;
use table::HeaderMode;
use uucore::display::Quotable;
use uucore::error::{UError, UResult, USimpleError, get_exit_code};
use uucore::fsext::{MountInfo, read_fs_list};
use uucore::parser::parse_size::ParseSizeError;
use uucore::translate;
use uucore::{format_usage, show};

use clap::{Arg, ArgAction, ArgMatches, Command, parser::ValueSource};

use std::ffi::OsString;
use std::io::stdout;
use std::path::Path;
use thiserror::Error;

use crate::blocks::{BlockSize, read_block_size};
use crate::columns::{Column, ColumnError};
use crate::filesystem::Filesystem;
use crate::filesystem::FsError;
use crate::table::Table;

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
    human_readable: Option<HumanReadable>,
    block_size: BlockSize,
    header_mode: HeaderMode,

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

    /// Whether to sync before operating.
    sync: bool,

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
            block_size: BlockSize::default(),
            human_readable: Option::default(),
            header_mode: HeaderMode::default(),
            include: Option::default(),
            exclude: Option::default(),
            sync: Default::default(),
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

#[derive(Debug, Error)]
enum OptionsError {
    // TODO This needs to vary based on whether `--block-size`
    // or `-B` were provided.
    #[error("{}", translate!("df-error-block-size-too-large", "size" => .0.clone()))]
    BlockSizeTooLarge(String),
    // TODO This needs to vary based on whether `--block-size`
    // or `-B` were provided.,
    #[error("{}", translate!("df-error-invalid-block-size", "size" => .0.clone()))]
    InvalidBlockSize(String),
    // TODO This needs to vary based on whether `--block-size`
    // or `-B` were provided.
    #[error("{}", translate!("df-error-invalid-suffix", "size" => .0.clone()))]
    InvalidSuffix(String),

    /// An error getting the columns to display in the output table.
    #[error("{}", translate!("df-error-field-used-more-than-once", "field" => format!("{}", .0)))]
    ColumnError(ColumnError),

    #[error(
        "{}",
        .0.iter()
            .map(|t| translate!("df-error-filesystem-type-both-selected-and-excluded", "type" => t.quote()))
            .collect::<Vec<_>>()
            .join(format!("\n{}: ", uucore::util_name()).as_str())
    )]
    FilesystemTypeBothSelectedAndExcluded(Vec<String>),
}

impl Options {
    /// Convert command-line arguments into [`Options`].
    fn from(matches: &ArgMatches) -> Result<Self, OptionsError> {
        let include: Option<Vec<_>> = matches
            .get_many::<OsString>(OPT_TYPE)
            .map(|v| v.map(|s| s.to_string_lossy().to_string()).collect());
        let exclude: Option<Vec<_>> = matches
            .get_many::<OsString>(OPT_EXCLUDE_TYPE)
            .map(|v| v.map(|s| s.to_string_lossy().to_string()).collect());

        if let (Some(include), Some(exclude)) = (&include, &exclude) {
            if let Some(types) = Self::get_intersected_types(include, exclude) {
                return Err(OptionsError::FilesystemTypeBothSelectedAndExcluded(types));
            }
        }

        Ok(Self {
            show_local_fs: matches.get_flag(OPT_LOCAL),
            show_all_fs: matches.get_flag(OPT_ALL),
            sync: matches.get_flag(OPT_SYNC),
            block_size: read_block_size(matches).map_err(|e| match e {
                ParseSizeError::InvalidSuffix(s) => OptionsError::InvalidSuffix(s),
                ParseSizeError::SizeTooBig(_) => OptionsError::BlockSizeTooLarge(
                    matches.get_one::<String>(OPT_BLOCKSIZE).unwrap().to_owned(),
                ),
                ParseSizeError::ParseFailure(s) => OptionsError::InvalidBlockSize(s),
                ParseSizeError::PhysicalMem(s) => OptionsError::InvalidBlockSize(s),
            })?,
            header_mode: {
                if matches.get_flag(OPT_HUMAN_READABLE_BINARY)
                    || matches.get_flag(OPT_HUMAN_READABLE_DECIMAL)
                {
                    HeaderMode::HumanReadable
                } else if matches.get_flag(OPT_PORTABILITY) {
                    HeaderMode::PosixPortability
                // get_flag() doesn't work here, it always returns true because OPT_OUTPUT has
                // default values and hence is always present
                } else if matches.value_source(OPT_OUTPUT) == Some(ValueSource::CommandLine) {
                    HeaderMode::Output
                } else {
                    HeaderMode::Default
                }
            },
            human_readable: {
                if matches.get_flag(OPT_HUMAN_READABLE_BINARY) {
                    Some(HumanReadable::Binary)
                } else if matches.get_flag(OPT_HUMAN_READABLE_DECIMAL) {
                    Some(HumanReadable::Decimal)
                } else {
                    None
                }
            },
            include,
            exclude,
            show_total: matches.get_flag(OPT_TOTAL),
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

        (!intersected_types.is_empty()).then_some(intersected_types)
    }
}

/// Whether to display the mount info given the inclusion settings.
fn is_included(mi: &MountInfo, opt: &Options) -> bool {
    // Don't show remote filesystems if `--local` has been given.
    if mi.remote && opt.show_local_fs {
        return false;
    }

    // Don't show pseudo filesystems unless `--all` has been given.
    // The "lofs" filesystem is a loopback
    // filesystem present on Solaris and FreeBSD systems. It
    // is similar to a symbolic link.
    if (mi.dummy || mi.fs_type == "lofs") && !opt.show_all_fs {
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

/// Get all currently mounted filesystems.
///
/// `opt` excludes certain filesystems from consideration and allows for the synchronization of filesystems before running; see
/// [`Options`] for more information.
fn get_all_filesystems(opt: &Options) -> UResult<Vec<Filesystem>> {
    // Run a sync call before any operation if so instructed.
    if opt.sync {
        #[cfg(not(any(windows, target_os = "redox")))]
        unsafe {
            #[cfg(not(target_os = "android"))]
            uucore::libc::sync();
            #[cfg(target_os = "android")]
            uucore::libc::syscall(uucore::libc::SYS_sync);
        }
    }

    let mut mounts = vec![];
    for mut mi in read_fs_list()? {
        // TODO The running time of the `is_best()` function is linear
        // in the length of `result`. That makes the running time of
        // this loop quadratic in the length of `vmi`. This could be
        // improved by a more efficient implementation of `is_best()`,
        // but `vmi` is probably not very long in practice.
        if is_included(&mi, opt) && is_best(&mounts, &mi) {
            let dev_path: &Path = Path::new(&mi.dev_name);
            if dev_path.is_symlink() {
                if let Ok(canonicalized_symlink) = uucore::fs::canonicalize(
                    dev_path,
                    uucore::fs::MissingHandling::Existing,
                    uucore::fs::ResolveMode::Logical,
                ) {
                    mi.dev_name = canonicalized_symlink.to_string_lossy().to_string();
                }
            }

            mounts.push(mi);
        }
    }

    // Convert each `MountInfo` into a `Filesystem`, which contains
    // both the mount information and usage information.
    #[cfg(not(windows))]
    {
        let maybe_mount = |m| Filesystem::from_mount(&mounts, &m, None).ok();
        Ok(mounts
            .clone()
            .into_iter()
            .filter_map(maybe_mount)
            .filter(|fs| opt.show_all_fs || fs.usage.blocks > 0)
            .collect())
    }
    #[cfg(windows)]
    {
        let maybe_mount = |m| Filesystem::from_mount(&m, None).ok();
        Ok(mounts
            .into_iter()
            .filter_map(maybe_mount)
            .filter(|fs| opt.show_all_fs || fs.usage.blocks > 0)
            .collect())
    }
}

/// For each path, get the filesystem that contains that path.
fn get_named_filesystems<P>(paths: &[P], opt: &Options) -> UResult<Vec<Filesystem>>
where
    P: AsRef<Path>,
{
    // The list of all mounted filesystems.
    let mounts: Vec<MountInfo> = read_fs_list()?;

    let mut result = vec![];

    // Convert each path into a `Filesystem`, which contains
    // both the mount information and usage information.
    for path in paths {
        match Filesystem::from_path(&mounts, path) {
            Ok(fs) => {
                if is_included(&fs.mount_info, opt) {
                    result.push(fs);
                }
            }
            Err(FsError::InvalidPath) => {
                show!(USimpleError::new(
                    1,
                    translate!("df-error-no-such-file-or-directory", "path" => path.as_ref().maybe_quote())
                ));
            }
            Err(FsError::MountMissing) => {
                show!(USimpleError::new(
                    1,
                    translate!("df-error-no-file-systems-processed")
                ));
            }
            #[cfg(not(windows))]
            Err(FsError::OverMounted) => {
                show!(USimpleError::new(
                    1,
                    translate!("df-error-cannot-access-over-mounted", "path" => path.as_ref().quote())
                ));
            }
        }
    }
    if get_exit_code() == 0 && result.is_empty() {
        show!(USimpleError::new(
            1,
            translate!("df-error-no-file-systems-processed")
        ));
        return Ok(result);
    }

    Ok(result)
}

#[derive(Debug, Error)]
enum DfError {
    /// A problem while parsing command-line options.
    #[error("{}", .0)]
    OptionsError(OptionsError),
}

impl UError for DfError {
    fn usage(&self) -> bool {
        matches!(self, Self::OptionsError(OptionsError::ColumnError(_)))
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    #[cfg(windows)]
    {
        if matches.get_flag(OPT_INODES) {
            println!(
                "{}",
                translate!("df-error-inodes-not-supported-windows", "program" => uucore::util_name())
            );
            return Ok(());
        }
    }

    let opt = Options::from(&matches).map_err(DfError::OptionsError)?;
    // Get the list of filesystems to display in the output table.
    let filesystems: Vec<Filesystem> = match matches.get_many::<OsString>(OPT_PATHS) {
        None => {
            let filesystems = get_all_filesystems(&opt).map_err(|e| {
                let context = translate!("df-error-cannot-read-table-of-mounted-filesystems");
                USimpleError::new(e.code(), format!("{context}: {e}"))
            })?;

            if filesystems.is_empty() {
                return Err(USimpleError::new(
                    1,
                    translate!("df-error-no-file-systems-processed"),
                ));
            }

            filesystems
        }
        Some(paths) => {
            let paths: Vec<_> = paths.collect();
            let filesystems = get_named_filesystems(&paths, &opt).map_err(|e| {
                let context = translate!("df-error-cannot-read-table-of-mounted-filesystems");
                USimpleError::new(e.code(), format!("{context}: {e}"))
            })?;

            // This can happen if paths are given as command-line arguments
            // but none of the paths exist.
            if filesystems.is_empty() {
                return Ok(());
            }

            filesystems
        }
    };

    Table::new(&opt, filesystems).write_to(&mut stdout())?;

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("df-about"))
        .override_usage(format_usage(&translate!("df-usage")))
        .after_help(translate!("df-after-help"))
        .infer_long_args(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(OPT_HELP)
                .long(OPT_HELP)
                .help(translate!("df-help-print-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(OPT_ALL)
                .short('a')
                .long("all")
                .overrides_with(OPT_ALL)
                .help(translate!("df-help-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_BLOCKSIZE)
                .short('B')
                .long("block-size")
                .value_name("SIZE")
                .overrides_with_all([OPT_KILO, OPT_BLOCKSIZE])
                .help(translate!("df-help-block-size")),
        )
        .arg(
            Arg::new(OPT_TOTAL)
                .long("total")
                .overrides_with(OPT_TOTAL)
                .help(translate!("df-help-total"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_HUMAN_READABLE_BINARY)
                .short('h')
                .long("human-readable")
                .overrides_with_all([OPT_HUMAN_READABLE_DECIMAL, OPT_HUMAN_READABLE_BINARY])
                .help(translate!("df-help-human-readable"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_HUMAN_READABLE_DECIMAL)
                .short('H')
                .long("si")
                .overrides_with_all([OPT_HUMAN_READABLE_BINARY, OPT_HUMAN_READABLE_DECIMAL])
                .help(translate!("df-help-si"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_INODES)
                .short('i')
                .long("inodes")
                .overrides_with(OPT_INODES)
                .help(translate!("df-help-inodes"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_KILO)
                .short('k')
                .help(translate!("df-help-kilo"))
                .overrides_with_all([OPT_BLOCKSIZE, OPT_KILO])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_LOCAL)
                .short('l')
                .long("local")
                .overrides_with(OPT_LOCAL)
                .help(translate!("df-help-local"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NO_SYNC)
                .long("no-sync")
                .overrides_with_all([OPT_SYNC, OPT_NO_SYNC])
                .help(translate!("df-help-no-sync"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_OUTPUT)
                .long("output")
                .value_name("FIELD_LIST")
                .action(ArgAction::Append)
                .num_args(0..)
                .require_equals(true)
                .use_value_delimiter(true)
                .value_parser(OUTPUT_FIELD_LIST)
                .default_missing_values(OUTPUT_FIELD_LIST)
                .default_values(["source", "size", "used", "avail", "pcent", "target"])
                .conflicts_with_all([OPT_INODES, OPT_PORTABILITY, OPT_PRINT_TYPE])
                .help(translate!("df-help-output")),
        )
        .arg(
            Arg::new(OPT_PORTABILITY)
                .short('P')
                .long("portability")
                .overrides_with(OPT_PORTABILITY)
                .help(translate!("df-help-portability"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_SYNC)
                .long("sync")
                .overrides_with_all([OPT_NO_SYNC, OPT_SYNC])
                .help(translate!("df-help-sync"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_TYPE)
                .short('t')
                .long("type")
                .value_parser(ValueParser::os_string())
                .value_name("TYPE")
                .action(ArgAction::Append)
                .help(translate!("df-help-type")),
        )
        .arg(
            Arg::new(OPT_PRINT_TYPE)
                .short('T')
                .long("print-type")
                .overrides_with(OPT_PRINT_TYPE)
                .help(translate!("df-help-print-type"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_EXCLUDE_TYPE)
                .short('x')
                .long("exclude-type")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_name("TYPE")
                .use_value_delimiter(true)
                .help(translate!("df-help-exclude-type")),
        )
        .arg(
            Arg::new(OPT_PATHS)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::AnyPath),
        )
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
                mount_dir: mount_dir.into(),
                mount_option: String::new(),
                mount_root: mount_root.into(),
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
                mount_dir: mount_dir.into(),
                mount_option: String::new(),
                mount_root: "/".into(),
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
            assert!(is_best(std::slice::from_ref(&m1), &m2));
            assert!(is_best(&[m2], &m1));
        }

        #[test]
        fn test_same_dev_id() {
            // There are several conditions under which a `MountInfo` is
            // considered "better" than the others, we're just checking
            // one condition in this test.
            let m1 = mount_info("0", "/mnt/bar");
            let m2 = mount_info("0", "/mnt/bar/baz");
            assert!(!is_best(std::slice::from_ref(&m1), &m2));
            assert!(is_best(&[m2], &m1));
        }
    }

    mod is_included {

        use crate::{Options, is_included};
        use uucore::fsext::MountInfo;

        /// Instantiate a [`MountInfo`] with the given fields.
        fn mount_info(fs_type: &str, mount_dir: &str, remote: bool, dummy: bool) -> MountInfo {
            MountInfo {
                dev_id: String::new(),
                dev_name: String::new(),
                fs_type: String::from(fs_type),
                mount_dir: mount_dir.into(),
                mount_option: String::new(),
                mount_root: "/".into(),
                remote,
                dummy,
            }
        }

        #[test]
        fn test_remote_included() {
            let opt = Options::default();
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
            let opt = Options::default();
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
}
