//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore tmpfs
//! The filesystem usage data table.
//!
//! A table comprises a header row ([`Header`]) and a collection of
//! data rows ([`Row`]), one per filesystem. To display a [`Row`],
//! combine it with [`Options`] in the [`DisplayRow`] struct; the
//! [`DisplayRow`] implements [`std::fmt::Display`].
use number_prefix::NumberPrefix;

use crate::{BlockSize, Filesystem, Options};
use uucore::fsext::{FsUsage, MountInfo};

use std::fmt;

/// A row in the filesystem usage data table.
///
/// A row comprises several pieces of information, including the
/// filesystem device, the mountpoint, the number of bytes used, etc.
pub(crate) struct Row {
    /// Name of the device on which the filesystem lives.
    fs_device: String,

    /// Type of filesystem (for example, `"ext4"`, `"tmpfs"`, etc.).
    fs_type: String,

    /// Path at which the filesystem is mounted.
    fs_mount: String,

    /// Total number of bytes in the filesystem regardless of whether they are used.
    bytes: u64,

    /// Number of used bytes.
    bytes_used: u64,

    /// Number of free bytes.
    bytes_free: u64,

    /// Percentage of bytes that are used, given as a float between 0 and 1.
    ///
    /// If the filesystem has zero bytes, then this is `None`.
    bytes_usage: Option<f64>,

    /// Percentage of bytes that are available, given as a float between 0 and 1.
    ///
    /// These are the bytes that are available to non-privileged processes.
    ///
    /// If the filesystem has zero bytes, then this is `None`.
    #[cfg(target_os = "macos")]
    bytes_capacity: Option<f64>,

    /// Total number of inodes in the filesystem.
    inodes: u64,

    /// Number of used inodes.
    inodes_used: u64,

    /// Number of free inodes.
    inodes_free: u64,

    /// Percentage of inodes that are used, given as a float between 0 and 1.
    ///
    /// If the filesystem has zero bytes, then this is `None`.
    inodes_usage: Option<f64>,
}

impl From<Filesystem> for Row {
    fn from(fs: Filesystem) -> Self {
        let MountInfo {
            dev_name,
            fs_type,
            mount_dir,
            ..
        } = fs.mount_info;
        let FsUsage {
            blocksize,
            blocks,
            bfree,
            #[cfg(target_os = "macos")]
            bavail,
            files,
            ffree,
            ..
        } = fs.usage;
        Self {
            fs_device: dev_name,
            fs_type,
            fs_mount: mount_dir,
            bytes: blocksize * blocks,
            bytes_used: blocksize * (blocks - bfree),
            bytes_free: blocksize * bfree,
            bytes_usage: if blocks == 0 {
                None
            } else {
                Some(((blocks - bfree) as f64) / blocks as f64)
            },
            #[cfg(target_os = "macos")]
            bytes_capacity: if bavail == 0 {
                None
            } else {
                Some(bavail as f64 / ((blocks - bfree + bavail) as f64))
            },
            inodes: files,
            inodes_used: files - ffree,
            inodes_free: ffree,
            inodes_usage: if files == 0 {
                None
            } else {
                Some(ffree as f64 / files as f64)
            },
        }
    }
}

/// A displayable wrapper around a [`Row`].
///
/// The `options` control how the information in the row gets displayed.
pub(crate) struct DisplayRow<'a> {
    /// The data in this row.
    row: Row,

    /// Options that control how to display the data.
    options: &'a Options,
    // TODO We don't need all of the command-line options here. Some
    // of the command-line options indicate which rows to include or
    // exclude. Other command-line options indicate which columns to
    // include or exclude. Still other options indicate how to format
    // numbers. We could split the options up into those groups to
    // reduce the coupling between this `table.rs` module and the main
    // `df.rs` module.
}

impl<'a> DisplayRow<'a> {
    /// Instantiate this struct.
    pub(crate) fn new(row: Row, options: &'a Options) -> Self {
        Self { row, options }
    }

    /// Get a string giving the scaled version of the input number.
    ///
    /// The scaling factor is defined in the `options` field.
    ///
    /// # Errors
    ///
    /// If the scaling factor is not 1000, 1024, or a negative number.
    fn scaled(&self, size: u64) -> Result<String, fmt::Error> {
        let number_prefix = match self.options.block_size {
            BlockSize::HumanReadableDecimal => NumberPrefix::decimal(size as f64),
            BlockSize::HumanReadableBinary => NumberPrefix::binary(size as f64),
            BlockSize::Bytes(d) => return Ok((size / d).to_string()),
        };
        match number_prefix {
            NumberPrefix::Standalone(bytes) => Ok(bytes.to_string()),
            NumberPrefix::Prefixed(prefix, bytes) => Ok(format!("{:.1}{}", bytes, prefix.symbol())),
        }
    }

    /// Convert a float between 0 and 1 into a percentage string.
    ///
    /// If `None`, return the string `"-"` instead.
    fn percentage(fraction: Option<f64>) -> String {
        match fraction {
            None => "-".to_string(),
            Some(x) => format!("{:.0}%", 100.0 * x),
        }
    }

    /// Write the bytes data for this row.
    ///
    /// # Errors
    ///
    /// If there is a problem writing to `f`.
    ///
    /// If the scaling factor is not 1000, 1024, or a negative number.
    fn fmt_bytes(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{0: >12} ", self.scaled(self.row.bytes)?)?;
        write!(f, "{0: >12} ", self.scaled(self.row.bytes_used)?)?;
        write!(f, "{0: >12} ", self.scaled(self.row.bytes_free)?)?;
        #[cfg(target_os = "macos")]
        write!(
            f,
            "{0: >12} ",
            DisplayRow::percentage(self.row.bytes_capacity)
        )?;
        write!(f, "{0: >5} ", DisplayRow::percentage(self.row.bytes_usage))?;
        Ok(())
    }

    /// Write the inodes data for this row.
    ///
    /// # Errors
    ///
    /// If there is a problem writing to `f`.
    ///
    /// If the scaling factor is not 1000, 1024, or a negative number.
    fn fmt_inodes(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{0: >12} ", self.scaled(self.row.inodes)?)?;
        write!(f, "{0: >12} ", self.scaled(self.row.inodes_used)?)?;
        write!(f, "{0: >12} ", self.scaled(self.row.inodes_free)?)?;
        write!(f, "{0: >5} ", DisplayRow::percentage(self.row.inodes_usage))?;
        Ok(())
    }
}

impl fmt::Display for DisplayRow<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{0: <16} ", self.row.fs_device)?;
        if self.options.show_fs_type {
            write!(f, "{0: <5} ", self.row.fs_type)?;
        }
        if self.options.show_inode_instead {
            self.fmt_inodes(f)?;
        } else {
            self.fmt_bytes(f)?;
        }
        write!(f, "{0: <16}", self.row.fs_mount)?;
        Ok(())
    }
}

/// The header row.
///
/// The `options` control which columns are displayed.
pub(crate) struct Header<'a> {
    /// Options that control which columns are displayed.
    options: &'a Options,
}

impl<'a> Header<'a> {
    /// Instantiate this struct.
    pub(crate) fn new(options: &'a Options) -> Self {
        Self { options }
    }
}

impl fmt::Display for Header<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{0: <16} ", "Filesystem")?;
        if self.options.show_fs_type {
            write!(f, "{0: <5} ", "Type")?;
        }
        if self.options.show_inode_instead {
            write!(f, "{0: >12} ", "Inodes")?;
            write!(f, "{0: >12} ", "IUsed")?;
            write!(f, "{0: >12} ", "IFree")?;
            write!(f, "{0: >5} ", "IUse%")?;
        } else {
            // TODO Support arbitrary positive scaling factors (from
            // the `--block-size` command-line argument).
            if let BlockSize::Bytes(_) = self.options.block_size {
                write!(f, "{0: >12} ", "1k-blocks")?;
            } else {
                write!(f, "{0: >12} ", "Size")?;
            };
            write!(f, "{0: >12} ", "Used")?;
            write!(f, "{0: >12} ", "Available")?;
            #[cfg(target_os = "macos")]
            write!(f, "{0: >12} ", "Capacity")?;
            write!(f, "{0: >5} ", "Use%")?;
        }
        write!(f, "{0: <16} ", "Mounted on")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::table::{DisplayRow, Header, Row};
    use crate::{BlockSize, Options};

    #[test]
    fn test_header_display() {
        let options = Default::default();
        assert_eq!(
            Header::new(&options).to_string(),
            "Filesystem          1k-blocks         Used    Available  Use% Mounted on       "
        );
    }

    #[test]
    fn test_header_display_fs_type() {
        let options = Options {
            show_fs_type: true,
            ..Default::default()
        };
        assert_eq!(
            Header::new(&options).to_string(),
            "Filesystem       Type     1k-blocks         Used    Available  Use% Mounted on       "
        );
    }

    #[test]
    fn test_header_display_inode() {
        let options = Options {
            show_inode_instead: true,
            ..Default::default()
        };
        assert_eq!(
            Header::new(&options).to_string(),
            "Filesystem             Inodes        IUsed        IFree IUse% Mounted on       "
        );
    }

    #[test]
    fn test_header_display_human_readable_binary() {
        let options = Options {
            block_size: BlockSize::HumanReadableBinary,
            ..Default::default()
        };
        assert_eq!(
            Header::new(&options).to_string(),
            "Filesystem               Size         Used    Available  Use% Mounted on       "
        );
    }

    #[test]
    fn test_header_display_human_readable_si() {
        let options = Options {
            block_size: BlockSize::HumanReadableDecimal,
            ..Default::default()
        };
        assert_eq!(
            Header::new(&options).to_string(),
            "Filesystem               Size         Used    Available  Use% Mounted on       "
        );
    }

    #[test]
    fn test_row_display() {
        let options = Options {
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_free: 75,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        assert_eq!(
            DisplayRow::new(row, &options).to_string(),
            "my_device                 100           25           75   25% my_mount        "
        );
    }

    #[test]
    fn test_row_display_fs_type() {
        let options = Options {
            block_size: BlockSize::Bytes(1),
            show_fs_type: true,
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_free: 75,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        assert_eq!(
            DisplayRow::new(row, &options).to_string(),
            "my_device        my_type          100           25           75   25% my_mount        "
        );
    }

    #[test]
    fn test_row_display_inodes() {
        let options = Options {
            block_size: BlockSize::Bytes(1),
            show_inode_instead: true,
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_free: 75,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        assert_eq!(
            DisplayRow::new(row, &options).to_string(),
            "my_device                  10            2            8   20% my_mount        "
        );
    }

    #[test]
    fn test_row_display_human_readable_si() {
        let options = Options {
            block_size: BlockSize::HumanReadableDecimal,
            show_fs_type: true,
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 4000,
            bytes_used: 1000,
            bytes_free: 3000,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        assert_eq!(
            DisplayRow::new(row, &options).to_string(),
            "my_device        my_type         4.0k         1.0k         3.0k   25% my_mount        "
        );
    }

    #[test]
    fn test_row_display_human_readable_binary() {
        let options = Options {
            block_size: BlockSize::HumanReadableBinary,
            show_fs_type: true,
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 4096,
            bytes_used: 1024,
            bytes_free: 3072,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        assert_eq!(
            DisplayRow::new(row, &options).to_string(),
            "my_device        my_type        4.0Ki        1.0Ki        3.0Ki   25% my_mount        "
        );
    }
}
