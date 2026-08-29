// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore itotal iused iavail ipcent pcent tmpfs squashfs lofs sysfs
mod blocks;
mod columns;
mod filesystem;
mod platform;
mod table;

use blocks::HumanReadable;
use clap::builder::ValueParser;
use table::HeaderMode;
use uucore::diagnostics::OptionValue;
use uucore::display::Quotable;
use uucore::error::{UError, UResult, USimpleError, get_exit_code};
use uucore::fsext::{MountInfo, read_fs_list};
use uucore::parser::parse_size::ParseSizeError;
use uucore::translate;
use uucore::{format_usage, show, show_warning};

use clap::{Arg, ArgAction, ArgMatches, Command, parser::ValueSource};

use std::ffi::OsString;
use std::io::{BufWriter, Write, stdout};
use std::path::Path;
use thiserror::Error;

use crate::blocks::{BlockSize, read_block_size};
use crate::columns::{Column, ColumnError};
pub use crate::filesystem::Filesystem;
use crate::filesystem::FsError;
use crate::table::Table;

static OPT_HELP: &str = "help";
static OPT_ALL: &str = "all";
static OPT_BLOCKSIZE: &str = "blocksize";
/// The long name of [`OPT_BLOCKSIZE`], which its clap id does not spell.
///
/// The caret report looks the option up on the command line by this name, so
/// the `Arg` and the report have to agree on it.
static OPT_BLOCKSIZE_LONG: &str = "block-size";
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
pub struct Options {
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

impl Options {
    /// Convert command-line arguments into [`Options`].
    pub fn from_matches(matches: &ArgMatches, diag_args: Option<&[OsString]>) -> UResult<Self> {
        Self::from(matches, diag_args)
    }

    /// Whether -a, -l, -t, or -x options require the mount table.
    fn requires_mount_table(&self) -> bool {
        self.show_all_fs || self.show_local_fs || self.include.is_some() || self.exclude.is_some()
    }
}

#[derive(Debug, Error)]
enum OptionsError {
    // TODO This needs to vary based on whether `--block-size`
    // or `-B` were provided.
    #[error("{}", translate!("df-error-block-size-too-large", "size" => .0))]
    BlockSizeTooLarge(String),
    // TODO This needs to vary based on whether `--block-size`
    // or `-B` were provided.,
    #[error("{}", translate!("df-error-invalid-block-size", "size" => .0))]
    InvalidBlockSize(String),
    // TODO This needs to vary based on whether `--block-size`
    // or `-B` were provided.
    #[error("{}", translate!("df-error-invalid-suffix", "size" => .0))]
    InvalidSuffix(String),

    /// An error getting the columns to display in the output table.
    #[error("{}", translate!("df-error-field-used-more-than-once", "field" => format!("{}", .0)))]
    ColumnError(ColumnError),

    #[error(
        "{}",
        .0.iter()
            .map(|t| translate!("df-error-filesystem-type-both-selected-and-excluded", "type" => t.quote()))
            .collect::<Vec<_>>()
            .join("\ndf: ")
    )]
    FilesystemTypeBothSelectedAndExcluded(Vec<String>),
}

/// The error for a `--block-size` that could not be parsed, with a caret under
/// the part of it that is at fault.
///
/// # Arguments
///
/// * `error` - What the size parser rejected the value with.
/// * `matches` - The parsed command line, for the value as it was typed.
/// * `diag_args` - The arguments as typed, or `None` when they were not kept.
fn block_size_error(
    error: &ParseSizeError,
    matches: &ArgMatches,
    diag_args: Option<&[OsString]>,
) -> Box<dyn UError> {
    // Only `-B`/`--block-size` reaches the parser with a value to point at:
    // `read_block_size` parses a size only under `contains_id(OPT_BLOCKSIZE)`,
    // and the `DF_BLOCK_SIZE` fallbacks go through `found()`, which drops an
    // invalid value silently. So the value the caret points at is always there.
    let size = matches
        .get_one::<String>(OPT_BLOCKSIZE)
        .expect("a block size error can only come from --block-size");
    let options_error = match error {
        ParseSizeError::InvalidSuffix(s) => OptionsError::InvalidSuffix(s.clone()),
        ParseSizeError::SizeTooBig(_) => OptionsError::BlockSizeTooLarge(size.clone()),
        ParseSizeError::ParseFailure(s) | ParseSizeError::PhysicalMem(s) => {
            OptionsError::InvalidBlockSize(s.clone())
        }
    };
    let message = options_error.to_string();
    error.size_value_error(
        diag_args,
        &OptionValue::new(size, 'B', OPT_BLOCKSIZE_LONG),
        0,
        &message,
        DfError::OptionsError(options_error),
    )
}

impl Options {
    /// Convert command-line arguments into [`Options`].
    fn from(matches: &ArgMatches, diag_args: Option<&[OsString]>) -> UResult<Self> {
        let include: Option<Vec<_>> = matches
            .get_many::<OsString>(OPT_TYPE)
            .map(|v| v.map(|s| s.to_string_lossy().to_string()).collect());
        let exclude: Option<Vec<_>> = matches
            .get_many::<OsString>(OPT_EXCLUDE_TYPE)
            .map(|v| v.map(|s| s.to_string_lossy().to_string()).collect());

        if let (Some(include), Some(exclude)) = (&include, &exclude)
            && let Some(types) = Self::get_intersected_types(include, exclude)
        {
            return Err(DfError::OptionsError(
                OptionsError::FilesystemTypeBothSelectedAndExcluded(types),
            )
            .into());
        }

        Ok(Self {
            show_local_fs: matches.get_flag(OPT_LOCAL),
            show_all_fs: matches.get_flag(OPT_ALL),
            sync: matches.get_flag(OPT_SYNC),
            block_size: read_block_size(matches)
                .map_err(|error| block_size_error(&error, matches, diag_args))?,
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
            columns: Column::from_matches(matches)
                .map_err(|e| DfError::OptionsError(OptionsError::ColumnError(e)))?,
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
    !(mi.remote && opt.show_local_fs) &&

    // Don't show pseudo filesystems unless `--all` has been given.
    // The "lofs" filesystem is a loopback
    // filesystem present on Solaris and FreeBSD systems. It
    // is similar to a symbolic link.
    !((mi.dummy || mi.fs_type == "lofs") && !opt.show_all_fs) &&

    // Don't show filesystems if they have been explicitly excluded.
    !opt.exclude.as_ref().is_some_and(|e| e.contains(&mi.fs_type)) &&
    opt.include.as_ref().is_none_or(|i| i.contains(&mi.fs_type))
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
    !(m1.dev_name != m2.dev_name && m1.mount_dir == m2.mount_dir)
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
/// `opt` excludes certain filesystems from consideration; see [`Options`] for more information.
fn get_all_filesystems(opt: &Options) -> UResult<Vec<Filesystem>> {
    let mut mounts = vec![];
    for mut mi in read_fs_list()? {
        // TODO The running time of the `is_best()` function is linear
        // in the length of `result`. That makes the running time of
        // this loop quadratic in the length of `vmi`. This could be
        // improved by a more efficient implementation of `is_best()`,
        // but `vmi` is probably not very long in practice.
        if is_included(&mi, opt) && is_best(&mounts, &mi) {
            let dev_path: &Path = Path::new(&mi.dev_name);
            // Only check is_symlink() for absolute paths. For non-absolute paths
            // like "tmpfs", "sysfs", etc., is_symlink() would resolve relative to
            // the current working directory, which is extremely slow in deeply
            // nested directories (O(n) syscalls where n is the directory depth).
            if dev_path.is_absolute()
                && dev_path.is_symlink()
                && let Ok(canonicalized_symlink) = uucore::fs::canonicalize(
                    dev_path,
                    uucore::fs::MissingHandling::Existing,
                    uucore::fs::ResolveMode::Logical,
                )
            {
                mi.dev_name = canonicalized_symlink.to_string_lossy().to_string();
            }

            mounts.push(mi);
        }
    }

    // Convert each `MountInfo` into a `Filesystem`, which contains
    // both the mount information and usage information.

    let maybe_mount = |m| platform::filesystem_from_mount(&mounts, m, None).ok();

    Ok(mounts
        .iter()
        .filter_map(maybe_mount)
        .filter(|fs| opt.show_all_fs || fs.usage.blocks > 0)
        .collect())
}

/// For each path, get the filesystem that contains that path.
fn get_named_filesystems<P>(paths: &[P], opt: &Options) -> UResult<Vec<Filesystem>>
where
    P: AsRef<Path>,
{
    // The list of all mounted filesystems.
    let mounts_result = read_fs_list();

    let (mounts, use_fallback) = match mounts_result {
        Ok(m) => (m, false),
        Err(e) => {
            if opt.requires_mount_table() {
                return Err(e);
            }
            show_warning!(
                "{}",
                translate!("df-error-cannot-read-table-of-mounted-filesystems")
            );
            (vec![], true)
        }
    };

    let mut result = vec![];

    // Convert each path into a `Filesystem`, which contains
    // both the mount information and usage information.
    for path in paths {
        let fs_result = platform::filesystem_for_path(&mounts, use_fallback, path);

        match fs_result {
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

/// Gather the filesystems that `df` would report on, without formatting anything.
///
/// `paths` is `None` to report on every mounted filesystem, as `df` does when
/// called without operands, or `Some` to report on the filesystems containing
/// the given paths.
pub fn filesystems(paths: Option<&[&Path]>, opt: &Options) -> UResult<Vec<Filesystem>> {
    // Run a sync call before any operation if so instructed.
    if opt.sync {
        platform::sync();
    }

    let unreadable_mount_table = |e: Box<dyn UError>| {
        let context = translate!("df-error-cannot-read-table-of-mounted-filesystems");
        USimpleError::new(e.code(), format!("{context}: {e}"))
    };

    match paths {
        None => {
            let filesystems = get_all_filesystems(opt).map_err(unreadable_mount_table)?;

            if filesystems.is_empty() {
                return Err(USimpleError::new(
                    1,
                    translate!("df-error-no-file-systems-processed"),
                ));
            }

            Ok(filesystems)
        }
        Some(paths) => get_named_filesystems(paths, opt).map_err(unreadable_mount_table),
    }
}

/// Write `filesystems` to `writer` as the standard `df` table.
pub fn write_table<W>(writer: &mut W, filesystems: Vec<Filesystem>, opt: &Options) -> UResult<()>
where
    W: Write,
{
    Table::new(opt, filesystems).write_to(writer)?;
    Ok(())
}

/// Display filesystem usage information as a table on stdout.
///
/// `paths` has the same meaning as in [`filesystems`].
pub fn df(paths: Option<&[&Path]>, opt: &Options) -> UResult<()> {
    let filesystems = filesystems(paths, opt)?;

    // Every path given on the command line was rejected; `filesystems` has
    // already emitted a diagnostic for each, so there is no table to print.
    if filesystems.is_empty() {
        return Ok(());
    }

    let mut writer = BufWriter::new(stdout().lock());
    write_table(&mut writer, filesystems, opt)?;

    // `BufWriter` swallows errors per drop, so flush explicitly.
    writer.flush()?;

    Ok(())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // The arguments are kept for the caret in SIZE diagnostics, which echoes
    // the command line.
    let (matches, diag_args) = uucore::clap_localization::handle_clap_result_with_diagnostics(
        uu_app(),
        args.collect(),
        1,
    )?;

    if let Some(result) = platform::maybe_unsupported_options(&matches) {
        return result;
    }

    let opt = Options::from_matches(&matches, diag_args.as_deref())?;
    let paths: Option<Vec<&Path>> = matches
        .get_many::<OsString>(OPT_PATHS)
        .map(|paths| paths.map(Path::new).collect());

    df(paths.as_deref(), &opt)
}

pub fn uu_app() -> Command {
    Command::new("df")
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
                .long(OPT_BLOCKSIZE_LONG)
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

    // spell-checker:ignore apfs
    mod write_table {

        use crate::blocks::BlockSize;
        use crate::{Filesystem, Options, write_table};
        use std::ffi::OsString;
        use uucore::fsext::{FsUsage, MountInfo};

        /// `Options::default()` picks up the block size from the environment
        /// (`POSIXLY_CORRECT` halves it), so pin it for reproducible output.
        fn options() -> Options {
            Options {
                block_size: BlockSize::Bytes(1024),
                ..Options::default()
            }
        }

        fn filesystem() -> Filesystem {
            Filesystem {
                file: Some(OsString::from("/tmp/file")),
                mount_info: MountInfo {
                    dev_id: String::from("1"),
                    dev_name: String::from("/dev/disk1"),
                    fs_type: String::from("apfs"),
                    mount_dir: OsString::from("/"),
                    mount_option: String::new(),
                    mount_root: OsString::new(),
                    remote: false,
                    dummy: false,
                },
                usage: FsUsage {
                    blocksize: 1024,
                    blocks: 10,
                    bfree: 4,
                    bavail: 3,
                    bavail_top_bit_set: false,
                    files: 20,
                    ffree: 5,
                },
            }
        }

        /// Rendering must be possible without touching stdout, so that crate
        /// users can capture the table themselves.
        #[test]
        fn test_write_table_to_arbitrary_writer() {
            let mut buffer: Vec<u8> = Vec::new();
            write_table(&mut buffer, vec![filesystem()], &options()).unwrap();

            // Header text is localized and the bundle is not loaded in unit
            // tests, so only the data row is asserted on here.
            let table = String::from_utf8(buffer).unwrap();
            let row = table.lines().nth(1).unwrap();

            assert_eq!(table.lines().count(), 2);
            assert!(row.starts_with("/dev/disk1"));
            assert!(row.ends_with(" /"));
            // 10 blocks of 1K, of which 6 are used and 3 available.
            assert_eq!(
                row.split_whitespace().collect::<Vec<_>>(),
                ["/dev/disk1", "10", "6", "3", "67%", "/"]
            );
        }

        #[test]
        fn test_write_table_empty() {
            let mut buffer: Vec<u8> = Vec::new();
            write_table(&mut buffer, vec![], &options()).unwrap();

            let table = String::from_utf8(buffer).unwrap();
            assert_eq!(table.lines().count(), 1);
        }
    }

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
