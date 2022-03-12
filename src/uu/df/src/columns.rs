//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore itotal iused iavail ipcent pcent squashfs
use crate::{OPT_INODES, OPT_OUTPUT, OPT_PRINT_TYPE};
use clap::ArgMatches;

/// The columns in the output table produced by `df`.
///
/// The [`Row`] struct has a field corresponding to each of the
/// variants of this enumeration.
///
/// [`Row`]: crate::table::Row
#[derive(PartialEq, Copy, Clone)]
pub(crate) enum Column {
    /// The source of the mount point, usually a device.
    Source,

    /// Total number of blocks.
    Size,

    /// Number of used blocks.
    Used,

    /// Number of available blocks.
    Avail,

    /// Percentage of blocks used out of total number of blocks.
    Pcent,

    /// The mount point.
    Target,

    /// Total number of inodes.
    Itotal,

    /// Number of used inodes.
    Iused,

    /// Number of available inodes.
    Iavail,

    /// Percentage of inodes used out of total number of inodes.
    Ipcent,

    /// The filename given as a command-line argument.
    File,

    /// The filesystem type, like "ext4" or "squashfs".
    Fstype,

    /// Percentage of bytes available to non-privileged processes.
    #[cfg(target_os = "macos")]
    Capacity,
}

impl Column {
    /// Convert from command-line arguments to sequence of columns.
    ///
    /// The set of columns that will appear in the output table can be
    /// specified by command-line arguments. This function converts
    /// those arguments to a [`Vec`] of [`Column`] variants.
    pub(crate) fn from_matches(matches: &ArgMatches) -> Vec<Self> {
        match (
            matches.is_present(OPT_PRINT_TYPE),
            matches.is_present(OPT_INODES),
            matches.occurrences_of(OPT_OUTPUT) > 0,
        ) {
            (false, false, false) => vec![
                Self::Source,
                Self::Size,
                Self::Used,
                Self::Avail,
                #[cfg(target_os = "macos")]
                Self::Capacity,
                Self::Pcent,
                Self::Target,
            ],
            (false, false, true) => {
                matches
                    .values_of(OPT_OUTPUT)
                    .unwrap()
                    .map(|s| {
                        // Unwrapping here should not panic because the
                        // command-line argument parsing library should be
                        // responsible for ensuring each comma-separated
                        // string is a valid column label.
                        Self::parse(s).unwrap()
                    })
                    .collect()
            }
            (false, true, false) => vec![
                Self::Source,
                Self::Itotal,
                Self::Iused,
                Self::Iavail,
                Self::Ipcent,
                Self::Target,
            ],
            (true, false, false) => vec![
                Self::Source,
                Self::Fstype,
                Self::Size,
                Self::Used,
                Self::Avail,
                #[cfg(target_os = "macos")]
                Self::Capacity,
                Self::Pcent,
                Self::Target,
            ],
            (true, true, false) => vec![
                Self::Source,
                Self::Fstype,
                Self::Itotal,
                Self::Iused,
                Self::Iavail,
                Self::Ipcent,
                Self::Target,
            ],
            // The command-line arguments -T and -i are each mutually
            // exclusive with --output, so the command-line argument
            // parser should reject those combinations before we get
            // to this point in the code.
            _ => unreachable!(),
        }
    }

    /// Convert a column name to the corresponding enumeration variant.
    ///
    /// There are twelve valid column names, one for each variant:
    ///
    /// - "source"
    /// - "fstype"
    /// - "itotal"
    /// - "iused"
    /// - "iavail"
    /// - "ipcent"
    /// - "size"
    /// - "used"
    /// - "avail"
    /// - "pcent"
    /// - "file"
    /// - "target"
    ///
    /// # Errors
    ///
    /// If the string `s` is not one of the valid column names.
    fn parse(s: &str) -> Result<Self, ()> {
        match s {
            "source" => Ok(Self::Source),
            "fstype" => Ok(Self::Fstype),
            "itotal" => Ok(Self::Itotal),
            "iused" => Ok(Self::Iused),
            "iavail" => Ok(Self::Iavail),
            "ipcent" => Ok(Self::Ipcent),
            "size" => Ok(Self::Size),
            "used" => Ok(Self::Used),
            "avail" => Ok(Self::Avail),
            "pcent" => Ok(Self::Pcent),
            "file" => Ok(Self::File),
            "target" => Ok(Self::Target),
            _ => Err(()),
        }
    }
}
