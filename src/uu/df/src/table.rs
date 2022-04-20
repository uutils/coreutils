//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore tmpfs Pcent Itotal Iused Iavail Ipcent
//! The filesystem usage data table.
//!
//! A table ([`Table`]) comprises a header row ([`Header`]) and a
//! collection of data rows ([`Row`]), one per filesystem.
use number_prefix::NumberPrefix;
use unicode_width::UnicodeWidthStr;

use crate::columns::{Alignment, Column};
use crate::filesystem::Filesystem;
use crate::{BlockSize, Options};
use uucore::fsext::{FsUsage, MountInfo};

use std::fmt;
use std::ops::AddAssign;

/// A row in the filesystem usage data table.
///
/// A row comprises several pieces of information, including the
/// filesystem device, the mountpoint, the number of bytes used, etc.
pub(crate) struct Row {
    /// The filename given on the command-line, if given.
    file: Option<String>,

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

    /// Number of available bytes.
    bytes_avail: u64,

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

impl Row {
    pub(crate) fn new(source: &str) -> Self {
        Self {
            file: None,
            fs_device: source.into(),
            fs_type: "-".into(),
            fs_mount: "-".into(),
            bytes: 0,
            bytes_used: 0,
            bytes_avail: 0,
            bytes_usage: None,
            #[cfg(target_os = "macos")]
            bytes_capacity: None,
            inodes: 0,
            inodes_used: 0,
            inodes_free: 0,
            inodes_usage: None,
        }
    }
}

impl AddAssign for Row {
    /// Sum the numeric values of two rows.
    ///
    /// The `Row::fs_device` field is set to `"total"` and the
    /// remaining `String` fields are set to `"-"`.
    fn add_assign(&mut self, rhs: Self) {
        let bytes = self.bytes + rhs.bytes;
        let bytes_used = self.bytes_used + rhs.bytes_used;
        let bytes_avail = self.bytes_avail + rhs.bytes_avail;
        let inodes = self.inodes + rhs.inodes;
        let inodes_used = self.inodes_used + rhs.inodes_used;
        *self = Self {
            file: None,
            fs_device: "total".into(),
            fs_type: "-".into(),
            fs_mount: "-".into(),
            bytes,
            bytes_used,
            bytes_avail,
            bytes_usage: if bytes == 0 {
                None
            } else {
                // We use "(bytes_used + bytes_avail)" instead of "bytes" because on some filesystems (e.g.
                // ext4) "bytes" also includes reserved blocks we ignore for the usage calculation.
                // https://www.gnu.org/software/coreutils/faq/coreutils-faq.html#df-Size-and-Used-and-Available-do-not-add-up
                Some(bytes_used as f64 / (bytes_used + bytes_avail) as f64)
            },
            // TODO Figure out how to compute this.
            #[cfg(target_os = "macos")]
            bytes_capacity: None,
            inodes,
            inodes_used,
            inodes_free: self.inodes_free + rhs.inodes_free,
            inodes_usage: if inodes == 0 {
                None
            } else {
                Some(inodes_used as f64 / inodes as f64)
            },
        }
    }
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
            bavail,
            files,
            ffree,
            ..
        } = fs.usage;
        let bused = blocks - bfree;
        let fused = files - ffree;
        Self {
            file: fs.file,
            fs_device: dev_name,
            fs_type,
            fs_mount: mount_dir,
            bytes: blocksize * blocks,
            bytes_used: blocksize * bused,
            bytes_avail: blocksize * bavail,
            bytes_usage: if blocks == 0 {
                None
            } else {
                // We use "(bused + bavail)" instead of "blocks" because on some filesystems (e.g.
                // ext4) "blocks" also includes reserved blocks we ignore for the usage calculation.
                // https://www.gnu.org/software/coreutils/faq/coreutils-faq.html#df-Size-and-Used-and-Available-do-not-add-up
                Some(bused as f64 / (bused + bavail) as f64)
            },
            #[cfg(target_os = "macos")]
            bytes_capacity: if bavail == 0 {
                None
            } else {
                Some(bavail as f64 / ((bused + bavail) as f64))
            },
            inodes: files,
            inodes_used: fused,
            inodes_free: ffree,
            inodes_usage: if files == 0 {
                None
            } else {
                Some(fused as f64 / files as f64)
            },
        }
    }
}

/// A formatter for [`Row`].
///
/// The `options` control how the information in the row gets formatted.
pub(crate) struct RowFormatter<'a> {
    /// The data in this row.
    row: &'a Row,

    /// Options that control how to format the data.
    options: &'a Options,
    // TODO We don't need all of the command-line options here. Some
    // of the command-line options indicate which rows to include or
    // exclude. Other command-line options indicate which columns to
    // include or exclude. Still other options indicate how to format
    // numbers. We could split the options up into those groups to
    // reduce the coupling between this `table.rs` module and the main
    // `df.rs` module.
}

impl<'a> RowFormatter<'a> {
    /// Instantiate this struct.
    pub(crate) fn new(row: &'a Row, options: &'a Options) -> Self {
        Self { row, options }
    }

    /// Get a human readable string giving the scaled version of the input number.
    ///
    /// The scaling factor is defined in the `options` field.
    ///
    /// This function is supposed to be used by `scaled_bytes()` and `scaled_inodes()` only.
    fn scaled_human_readable(&self, size: u64) -> String {
        let number_prefix = match self.options.block_size {
            BlockSize::HumanReadableDecimal => NumberPrefix::decimal(size as f64),
            BlockSize::HumanReadableBinary => NumberPrefix::binary(size as f64),
            _ => unreachable!(),
        };
        match number_prefix {
            NumberPrefix::Standalone(bytes) => bytes.to_string(),
            NumberPrefix::Prefixed(prefix, bytes) => format!("{:.1}{}", bytes, prefix.symbol()),
        }
    }

    /// Get a string giving the scaled version of the input number.
    ///
    /// The scaling factor is defined in the `options` field.
    fn scaled_bytes(&self, size: u64) -> String {
        if let BlockSize::Bytes(d) = self.options.block_size {
            (size / d).to_string()
        } else {
            self.scaled_human_readable(size)
        }
    }

    /// Get a string giving the scaled version of the input number.
    ///
    /// The scaling factor is defined in the `options` field.
    fn scaled_inodes(&self, size: u64) -> String {
        if let BlockSize::Bytes(_) = self.options.block_size {
            size.to_string()
        } else {
            self.scaled_human_readable(size)
        }
    }

    /// Convert a float between 0 and 1 into a percentage string.
    ///
    /// If `None`, return the string `"-"` instead.
    fn percentage(fraction: Option<f64>) -> String {
        match fraction {
            None => "-".to_string(),
            Some(x) => format!("{:.0}%", (100.0 * x).ceil()),
        }
    }

    /// Returns formatted row data.
    fn get_values(&self) -> Vec<String> {
        let mut strings = Vec::new();

        for column in &self.options.columns {
            let string = match column {
                Column::Source => self.row.fs_device.to_string(),
                Column::Size => self.scaled_bytes(self.row.bytes),
                Column::Used => self.scaled_bytes(self.row.bytes_used),
                Column::Avail => self.scaled_bytes(self.row.bytes_avail),
                Column::Pcent => Self::percentage(self.row.bytes_usage),

                Column::Target => self.row.fs_mount.to_string(),
                Column::Itotal => self.scaled_inodes(self.row.inodes),
                Column::Iused => self.scaled_inodes(self.row.inodes_used),
                Column::Iavail => self.scaled_inodes(self.row.inodes_free),
                Column::Ipcent => Self::percentage(self.row.inodes_usage),
                Column::File => self.row.file.as_ref().unwrap_or(&"-".into()).to_string(),

                Column::Fstype => self.row.fs_type.to_string(),
                #[cfg(target_os = "macos")]
                Column::Capacity => Self::percentage(self.row.bytes_capacity),
            };

            strings.push(string);
        }

        strings
    }
}

/// The data of the header row.
struct Header {}

impl Header {
    /// Return the headers for the specified columns.
    ///
    /// The `options` control which column headers are returned.
    fn get_headers(options: &Options) -> Vec<String> {
        let mut headers = Vec::new();

        for column in &options.columns {
            let header = match column {
                Column::Source => String::from("Filesystem"),
                Column::Size => options.block_size.to_string(),
                Column::Used => String::from("Used"),
                Column::Avail => String::from("Available"),
                Column::Pcent => String::from("Use%"),
                Column::Target => String::from("Mounted on"),
                Column::Itotal => String::from("Inodes"),
                Column::Iused => String::from("IUsed"),
                Column::Iavail => String::from("IFree"),
                Column::Ipcent => String::from("IUse%"),
                Column::File => String::from("File"),
                Column::Fstype => String::from("Type"),
                #[cfg(target_os = "macos")]
                Column::Capacity => String::from("Capacity"),
            };

            headers.push(header);
        }

        headers
    }
}

/// The output table.
pub(crate) struct Table {
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
    widths: Vec<usize>,
}

impl Table {
    pub(crate) fn new(options: &Options, filesystems: Vec<Filesystem>) -> Self {
        let headers = Header::get_headers(options);
        let mut widths: Vec<_> = options
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| Column::min_width(col).max(headers[i].len()))
            .collect();

        let mut rows = vec![headers];

        // The running total of filesystem sizes and usage.
        //
        // This accumulator is computed in case we need to display the
        // total counts in the last row of the table.
        let mut total = Row::new("total");

        for filesystem in filesystems {
            // If the filesystem is not empty, or if the options require
            // showing all filesystems, then print the data as a row in
            // the output table.
            if options.show_all_fs || filesystem.usage.blocks > 0 {
                let row = Row::from(filesystem);
                let fmt = RowFormatter::new(&row, options);
                let values = fmt.get_values();
                total += row;

                for (i, value) in values.iter().enumerate() {
                    if UnicodeWidthStr::width(value.as_str()) > widths[i] {
                        widths[i] = UnicodeWidthStr::width(value.as_str());
                    }
                }

                rows.push(values);
            }
        }

        if options.show_total {
            let total_row = RowFormatter::new(&total, options);
            rows.push(total_row.get_values());
        }

        Self {
            rows,
            widths,
            alignments: Self::get_alignments(&options.columns),
        }
    }

    fn get_alignments(columns: &Vec<Column>) -> Vec<Alignment> {
        let mut alignments = Vec::new();

        for column in columns {
            alignments.push(Column::alignment(column));
        }

        alignments
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut row_iter = self.rows.iter().peekable();
        while let Some(row) = row_iter.next() {
            let mut col_iter = row.iter().enumerate().peekable();
            while let Some((i, elem)) = col_iter.next() {
                match self.alignments[i] {
                    Alignment::Left => write!(f, "{:<width$}", elem, width = self.widths[i])?,
                    Alignment::Right => write!(f, "{:>width$}", elem, width = self.widths[i])?,
                }

                if col_iter.peek().is_some() {
                    // column separator
                    write!(f, " ")?;
                }
            }

            if row_iter.peek().is_some() {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::columns::Column;
    use crate::table::{Header, Row, RowFormatter};
    use crate::{BlockSize, Options};

    const COLUMNS_WITH_FS_TYPE: [Column; 7] = [
        Column::Source,
        Column::Fstype,
        Column::Size,
        Column::Used,
        Column::Avail,
        Column::Pcent,
        Column::Target,
    ];
    const COLUMNS_WITH_INODES: [Column; 6] = [
        Column::Source,
        Column::Itotal,
        Column::Iused,
        Column::Iavail,
        Column::Ipcent,
        Column::Target,
    ];

    #[test]
    fn test_default_header() {
        let options = Default::default();
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "1K-blocks",
                "Used",
                "Available",
                "Use%",
                "Mounted on"
            )
        );
    }

    #[test]
    fn test_header_with_fs_type() {
        let options = Options {
            columns: COLUMNS_WITH_FS_TYPE.to_vec(),
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "Type",
                "1K-blocks",
                "Used",
                "Available",
                "Use%",
                "Mounted on"
            )
        );
    }

    #[test]
    fn test_header_with_inodes() {
        let options = Options {
            columns: COLUMNS_WITH_INODES.to_vec(),
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "Inodes",
                "IUsed",
                "IFree",
                "IUse%",
                "Mounted on"
            )
        );
    }

    #[test]
    fn test_header_with_block_size_1024() {
        let options = Options {
            block_size: BlockSize::Bytes(3 * 1024),
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "3K-blocks",
                "Used",
                "Available",
                "Use%",
                "Mounted on"
            )
        );
    }

    #[test]
    fn test_header_with_human_readable_binary() {
        let options = Options {
            block_size: BlockSize::HumanReadableBinary,
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "Size",
                "Used",
                "Available",
                "Use%",
                "Mounted on"
            )
        );
    }

    #[test]
    fn test_header_with_human_readable_si() {
        let options = Options {
            block_size: BlockSize::HumanReadableDecimal,
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "Size",
                "Used",
                "Available",
                "Use%",
                "Mounted on"
            )
        );
    }

    #[test]
    fn test_row_formatter() {
        let options = Options {
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            file: Some("/path/to/file".to_string()),
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_avail: 75,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        let fmt = RowFormatter::new(&row, &options);
        assert_eq!(
            fmt.get_values(),
            vec!("my_device", "100", "25", "75", "25%", "my_mount")
        );
    }

    #[test]
    fn test_row_formatter_with_fs_type() {
        let options = Options {
            columns: COLUMNS_WITH_FS_TYPE.to_vec(),
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            file: Some("/path/to/file".to_string()),
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_avail: 75,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        let fmt = RowFormatter::new(&row, &options);
        assert_eq!(
            fmt.get_values(),
            vec!("my_device", "my_type", "100", "25", "75", "25%", "my_mount")
        );
    }

    #[test]
    fn test_row_formatter_with_inodes() {
        let options = Options {
            columns: COLUMNS_WITH_INODES.to_vec(),
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            file: Some("/path/to/file".to_string()),
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_avail: 75,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        let fmt = RowFormatter::new(&row, &options);
        assert_eq!(
            fmt.get_values(),
            vec!("my_device", "10", "2", "8", "20%", "my_mount")
        );
    }

    #[test]
    fn test_row_formatter_with_bytes_and_inodes() {
        let options = Options {
            columns: vec![Column::Size, Column::Itotal],
            block_size: BlockSize::Bytes(100),
            ..Default::default()
        };
        let row = Row {
            file: Some("/path/to/file".to_string()),
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_avail: 75,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        let fmt = RowFormatter::new(&row, &options);
        assert_eq!(fmt.get_values(), vec!("1", "10"));
    }

    #[test]
    fn test_row_formatter_with_human_readable_si() {
        let options = Options {
            block_size: BlockSize::HumanReadableDecimal,
            columns: COLUMNS_WITH_FS_TYPE.to_vec(),
            ..Default::default()
        };
        let row = Row {
            file: Some("/path/to/file".to_string()),
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 4000,
            bytes_used: 1000,
            bytes_avail: 3000,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        let fmt = RowFormatter::new(&row, &options);
        assert_eq!(
            fmt.get_values(),
            vec!(
                "my_device",
                "my_type",
                "4.0k",
                "1.0k",
                "3.0k",
                "25%",
                "my_mount"
            )
        );
    }

    #[test]
    fn test_row_formatter_with_human_readable_binary() {
        let options = Options {
            block_size: BlockSize::HumanReadableBinary,
            columns: COLUMNS_WITH_FS_TYPE.to_vec(),
            ..Default::default()
        };
        let row = Row {
            file: Some("/path/to/file".to_string()),
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 4096,
            bytes_used: 1024,
            bytes_avail: 3072,
            bytes_usage: Some(0.25),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        let fmt = RowFormatter::new(&row, &options);
        assert_eq!(
            fmt.get_values(),
            vec!(
                "my_device",
                "my_type",
                "4.0Ki",
                "1.0Ki",
                "3.0Ki",
                "25%",
                "my_mount"
            )
        );
    }

    #[test]
    fn test_row_formatter_with_round_up_usage() {
        let options = Options {
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            file: Some("/path/to/file".to_string()),
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".to_string(),

            bytes: 100,
            bytes_used: 25,
            bytes_avail: 75,
            bytes_usage: Some(0.251),

            #[cfg(target_os = "macos")]
            bytes_capacity: Some(0.5),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),
        };
        let fmt = RowFormatter::new(&row, &options);
        assert_eq!(
            fmt.get_values(),
            vec!("my_device", "100", "25", "75", "26%", "my_mount")
        );
    }
}
