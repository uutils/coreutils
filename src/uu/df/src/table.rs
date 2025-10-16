// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore tmpfs Pcent Itotal Iused Iavail Ipcent nosuid nodev
//! The filesystem usage data table.
//!
//! A table ([`Table`]) comprises a header row ([`Header`]) and a
//! collection of data rows ([`Row`]), one per filesystem.
use unicode_width::UnicodeWidthStr;

use crate::blocks::{SuffixType, to_magnitude_and_suffix};
use crate::columns::{Alignment, Column};
use crate::filesystem::Filesystem;
use crate::{BlockSize, Options};
use uucore::fsext::{FsUsage, MountInfo};
use uucore::translate;

use std::ffi::OsString;
use std::iter;
use std::ops::AddAssign;

/// A row in the filesystem usage data table.
///
/// A row comprises several pieces of information, including the
/// filesystem device, the mountpoint, the number of bytes used, etc.
pub(crate) struct Row {
    /// The filename given on the command-line, if given.
    file: Option<OsString>,

    /// Name of the device on which the filesystem lives.
    fs_device: String,

    /// Type of filesystem (for example, `"ext4"`, `"tmpfs"`, etc.).
    fs_type: String,

    /// Path at which the filesystem is mounted.
    fs_mount: OsString,

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
    inodes: u128,

    /// Number of used inodes.
    inodes_used: u128,

    /// Number of free inodes.
    inodes_free: u128,

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
            fs_device: translate!("df-total"),
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

        // On Windows WSL, files can be less than ffree. Protect such cases via saturating_sub.
        let bused = blocks.saturating_sub(bfree);
        let fused = files.saturating_sub(ffree);
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
            inodes: files as u128,
            inodes_used: fused as u128,
            inodes_free: ffree as u128,
            inodes_usage: if files == 0 {
                None
            } else {
                Some(fused as f64 / files as f64)
            },
        }
    }
}

/// A `Cell` in the table. We store raw `bytes` as the data (e.g. directory name
/// may be non-Unicode). We also record the printed `width` for alignment purpose,
/// as it is easier to compute on the original string.
struct Cell {
    bytes: Vec<u8>,
    width: usize,
}

impl Cell {
    /// Create a cell, knowing that s contains only 1-length chars
    fn from_ascii_string<T: AsRef<str>>(s: T) -> Self {
        let s = s.as_ref();
        Self {
            bytes: s.as_bytes().into(),
            width: s.len(),
        }
    }

    /// Create a cell from an unknown origin string that may contain
    /// wide characters.
    fn from_string<T: AsRef<str>>(s: T) -> Self {
        let s = s.as_ref();
        Self {
            bytes: s.as_bytes().into(),
            width: UnicodeWidthStr::width(s),
        }
    }

    /// Create a cell from an `OsString`
    fn from_os_string(os: &OsString) -> Self {
        Self {
            bytes: uucore::os_str_as_bytes(os).unwrap().to_vec(),
            width: UnicodeWidthStr::width(os.to_string_lossy().as_ref()),
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
    /// Whether to use the special rules for displaying the total row.
    is_total_row: bool,
}

impl<'a> RowFormatter<'a> {
    /// Instantiate this struct.
    pub(crate) fn new(row: &'a Row, options: &'a Options, is_total_row: bool) -> Self {
        Self {
            row,
            options,
            is_total_row,
        }
    }

    /// Get a string giving the scaled version of the input number.
    ///
    /// The scaling factor is defined in the `options` field.
    fn scaled_bytes(&self, size: u64) -> Cell {
        let s = if let Some(h) = self.options.human_readable {
            to_magnitude_and_suffix(size.into(), SuffixType::HumanReadable(h), true)
        } else {
            let BlockSize::Bytes(d) = self.options.block_size;
            (size as f64 / d as f64).ceil().to_string()
        };
        Cell::from_ascii_string(s)
    }

    /// Get a string giving the scaled version of the input number.
    ///
    /// The scaling factor is defined in the `options` field.
    fn scaled_inodes(&self, size: u128) -> Cell {
        let s = if let Some(h) = self.options.human_readable {
            to_magnitude_and_suffix(size, SuffixType::HumanReadable(h), true)
        } else {
            size.to_string()
        };
        Cell::from_ascii_string(s)
    }

    /// Convert a float between 0 and 1 into a percentage string.
    ///
    /// If `None`, return the string `"-"` instead.
    fn percentage(fraction: Option<f64>) -> Cell {
        let s = match fraction {
            None => "-".to_string(),
            Some(x) => format!("{:.0}%", (100.0 * x).ceil()),
        };
        Cell::from_ascii_string(s)
    }

    /// Returns formatted row data.
    fn get_cells(&self) -> Vec<Cell> {
        let mut cells = Vec::new();

        for column in &self.options.columns {
            let cell = match column {
                Column::Source => {
                    if self.is_total_row {
                        Cell::from_string(translate!("df-total"))
                    } else {
                        Cell::from_string(&self.row.fs_device)
                    }
                }
                Column::Size => self.scaled_bytes(self.row.bytes),
                Column::Used => self.scaled_bytes(self.row.bytes_used),
                Column::Avail => self.scaled_bytes(self.row.bytes_avail),
                Column::Pcent => Self::percentage(self.row.bytes_usage),

                Column::Target => {
                    if self.is_total_row && !self.options.columns.contains(&Column::Source) {
                        Cell::from_string(translate!("df-total"))
                    } else {
                        Cell::from_os_string(&self.row.fs_mount)
                    }
                }
                Column::Itotal => self.scaled_inodes(self.row.inodes),
                Column::Iused => self.scaled_inodes(self.row.inodes_used),
                Column::Iavail => self.scaled_inodes(self.row.inodes_free),
                Column::Ipcent => Self::percentage(self.row.inodes_usage),
                Column::File => self
                    .row
                    .file
                    .as_ref()
                    .map_or(Cell::from_ascii_string("-"), Cell::from_os_string),

                Column::Fstype => Cell::from_string(&self.row.fs_type),
                #[cfg(target_os = "macos")]
                Column::Capacity => Self::percentage(self.row.bytes_capacity),
            };

            cells.push(cell);
        }

        cells
    }
}

/// A `HeaderMode` defines what header labels should be shown.
#[derive(Default)]
pub(crate) enum HeaderMode {
    #[default]
    Default,
    // the user used -h or -H
    HumanReadable,
    // the user used -P
    PosixPortability,
    // the user used --output
    Output,
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
                Column::Source => translate!("df-header-filesystem"),
                Column::Size => match options.header_mode {
                    HeaderMode::HumanReadable => translate!("df-header-size"),
                    HeaderMode::PosixPortability => {
                        format!(
                            "{}{}",
                            options.block_size.as_u64(),
                            translate!("df-blocks-suffix")
                        )
                    }
                    _ => format!(
                        "{}{}",
                        options.block_size.to_header(),
                        translate!("df-blocks-suffix")
                    ),
                },
                Column::Used => translate!("df-header-used"),
                Column::Avail => match options.header_mode {
                    HeaderMode::HumanReadable | HeaderMode::Output => {
                        translate!("df-header-avail")
                    }
                    _ => translate!("df-header-available"),
                },
                Column::Pcent => match options.header_mode {
                    HeaderMode::PosixPortability => translate!("df-header-capacity"),
                    _ => translate!("df-header-use-percent"),
                },
                Column::Target => translate!("df-header-mounted-on"),
                Column::Itotal => translate!("df-header-inodes"),
                Column::Iused => translate!("df-header-iused"),
                Column::Iavail => translate!("df-header-iavail"),
                Column::Ipcent => translate!("df-header-iuse-percent"),
                Column::File => translate!("df-header-file"),
                Column::Fstype => translate!("df-header-type"),
                #[cfg(target_os = "macos")]
                Column::Capacity => translate!("df-header-capacity"),
            };

            headers.push(header);
        }

        headers
    }
}

/// The output table.
pub(crate) struct Table {
    alignments: Vec<Alignment>,
    rows: Vec<Vec<Cell>>,
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

        let mut rows = vec![headers.iter().map(Cell::from_string).collect()];

        // The running total of filesystem sizes and usage.
        //
        // This accumulator is computed in case we need to display the
        // total counts in the last row of the table.
        let mut total = Row::new(&translate!("df-total"));

        for filesystem in filesystems {
            // If the filesystem is not empty, or if the options require
            // showing all filesystems, then print the data as a row in
            // the output table.
            if options.show_all_fs || filesystem.usage.blocks > 0 {
                let row = Row::from(filesystem);
                let fmt = RowFormatter::new(&row, options, false);
                let values = fmt.get_cells();
                total += row;

                rows.push(values);
            }
        }

        if options.show_total {
            let total_row = RowFormatter::new(&total, options, true);
            rows.push(total_row.get_cells());
        }

        // extend the column widths (in chars) for long values in rows
        // do it here, after total row was added to the list of rows
        for row in &rows {
            for (i, value) in row.iter().enumerate() {
                if value.width > widths[i] {
                    widths[i] = value.width;
                }
            }
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

    pub(crate) fn write_to(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        for row in &self.rows {
            let mut col_iter = row.iter().enumerate().peekable();
            while let Some((i, elem)) = col_iter.next() {
                let is_last_col = col_iter.peek().is_none();

                let pad_width = self.widths[i].saturating_sub(elem.width);
                match self.alignments.get(i) {
                    Some(Alignment::Left) => {
                        writer.write_all(&elem.bytes)?;
                        // no trailing spaces in last column
                        if !is_last_col {
                            writer
                                .write_all(&iter::repeat_n(b' ', pad_width).collect::<Vec<_>>())?;
                        }
                    }
                    Some(Alignment::Right) => {
                        writer.write_all(&iter::repeat_n(b' ', pad_width).collect::<Vec<_>>())?;
                        writer.write_all(&elem.bytes)?;
                    }
                    None => break,
                }

                if !is_last_col {
                    // column separator
                    writer.write_all(b" ")?;
                }
            }

            writeln!(writer)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::vec;
    use uucore::locale::setup_localization;

    use crate::blocks::HumanReadable;
    use crate::columns::Column;
    use crate::table::{Cell, Header, HeaderMode, Row, RowFormatter, Table};
    use crate::{BlockSize, Options};

    fn init() {
        unsafe {
            std::env::set_var("LANG", "C");
        }
        let _ = setup_localization("df");
    }

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

    impl Default for Row {
        fn default() -> Self {
            Self {
                file: Some("/path/to/file".into()),
                fs_device: "my_device".to_string(),
                fs_type: "my_type".to_string(),
                fs_mount: "my_mount".into(),

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
            }
        }
    }

    #[test]
    fn test_default_header() {
        init();
        let options = Options::default();

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
        init();
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
        init();
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
        init();
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
    fn test_human_readable_header() {
        init();
        let options = Options {
            header_mode: HeaderMode::HumanReadable,
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!("Filesystem", "Size", "Used", "Avail", "Use%", "Mounted on")
        );
    }

    #[test]
    fn test_posix_portability_header() {
        init();
        let options = Options {
            header_mode: HeaderMode::PosixPortability,
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "1024-blocks",
                "Used",
                "Available",
                "Capacity",
                "Mounted on"
            )
        );
    }

    #[test]
    fn test_output_header() {
        init();
        let options = Options {
            header_mode: HeaderMode::Output,
            ..Default::default()
        };
        assert_eq!(
            Header::get_headers(&options),
            vec!(
                "Filesystem",
                "1K-blocks",
                "Used",
                "Avail",
                "Use%",
                "Mounted on"
            )
        );
    }

    fn compare_cell_content(cells: Vec<Cell>, expected: Vec<&str>) -> bool {
        cells
            .into_iter()
            .zip(expected)
            .all(|(c, s)| c.bytes == s.as_bytes())
    }

    #[test]
    fn test_row_formatter() {
        init();
        let options = Options {
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_mount: "my_mount".into(),

            bytes: 100,
            bytes_used: 25,
            bytes_avail: 75,
            bytes_usage: Some(0.25),

            ..Default::default()
        };
        let fmt = RowFormatter::new(&row, &options, false);
        assert!(compare_cell_content(
            fmt.get_cells(),
            vec!("my_device", "100", "25", "75", "25%", "my_mount")
        ));
    }

    #[test]
    fn test_row_formatter_with_fs_type() {
        init();
        let options = Options {
            columns: COLUMNS_WITH_FS_TYPE.to_vec(),
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".into(),

            bytes: 100,
            bytes_used: 25,
            bytes_avail: 75,
            bytes_usage: Some(0.25),

            ..Default::default()
        };
        let fmt = RowFormatter::new(&row, &options, false);
        assert!(compare_cell_content(
            fmt.get_cells(),
            vec!("my_device", "my_type", "100", "25", "75", "25%", "my_mount")
        ));
    }

    #[test]
    fn test_row_formatter_with_inodes() {
        init();
        let options = Options {
            columns: COLUMNS_WITH_INODES.to_vec(),
            block_size: BlockSize::Bytes(1),
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_mount: "my_mount".into(),

            inodes: 10,
            inodes_used: 2,
            inodes_free: 8,
            inodes_usage: Some(0.2),

            ..Default::default()
        };
        let fmt = RowFormatter::new(&row, &options, false);
        assert!(compare_cell_content(
            fmt.get_cells(),
            vec!("my_device", "10", "2", "8", "20%", "my_mount")
        ));
    }

    #[test]
    fn test_row_formatter_with_bytes_and_inodes() {
        init();
        let options = Options {
            columns: vec![Column::Size, Column::Itotal],
            block_size: BlockSize::Bytes(100),
            ..Default::default()
        };
        let row = Row {
            bytes: 100,
            inodes: 10,
            ..Default::default()
        };
        let fmt = RowFormatter::new(&row, &options, false);
        assert!(compare_cell_content(fmt.get_cells(), vec!("1", "10")));
    }

    #[test]
    fn test_row_formatter_with_human_readable_si() {
        init();
        let options = Options {
            human_readable: Some(HumanReadable::Decimal),
            columns: COLUMNS_WITH_FS_TYPE.to_vec(),
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".into(),

            bytes: 40000,
            bytes_used: 1000,
            bytes_avail: 39000,
            bytes_usage: Some(0.025),

            ..Default::default()
        };
        let fmt = RowFormatter::new(&row, &options, false);
        assert!(compare_cell_content(
            fmt.get_cells(),
            vec!(
                "my_device",
                "my_type",
                "40k",
                "1.0k",
                "39k",
                "3%",
                "my_mount"
            )
        ));
    }

    #[test]
    fn test_row_formatter_with_human_readable_binary() {
        init();
        let options = Options {
            human_readable: Some(HumanReadable::Binary),
            columns: COLUMNS_WITH_FS_TYPE.to_vec(),
            ..Default::default()
        };
        let row = Row {
            fs_device: "my_device".to_string(),
            fs_type: "my_type".to_string(),
            fs_mount: "my_mount".into(),

            bytes: 4096,
            bytes_used: 1024,
            bytes_avail: 3072,
            bytes_usage: Some(0.25),

            ..Default::default()
        };
        let fmt = RowFormatter::new(&row, &options, false);
        assert!(compare_cell_content(
            fmt.get_cells(),
            vec!(
                "my_device",
                "my_type",
                "4.0K",
                "1.0K",
                "3.0K",
                "25%",
                "my_mount"
            )
        ));
    }

    #[test]
    fn test_row_formatter_with_round_up_usage() {
        init();
        let options = Options {
            columns: vec![Column::Pcent],
            ..Default::default()
        };
        let row = Row {
            bytes_usage: Some(0.251),
            ..Default::default()
        };
        let fmt = RowFormatter::new(&row, &options, false);
        assert!(compare_cell_content(fmt.get_cells(), vec!("26%")));
    }

    #[test]
    fn test_row_formatter_with_round_up_byte_values() {
        init();
        fn get_formatted_values(bytes: u64, bytes_used: u64, bytes_avail: u64) -> Vec<Cell> {
            let options = Options {
                block_size: BlockSize::Bytes(1000),
                columns: vec![Column::Size, Column::Used, Column::Avail],
                ..Default::default()
            };

            let row = Row {
                bytes,
                bytes_used,
                bytes_avail,
                ..Default::default()
            };
            RowFormatter::new(&row, &options, false).get_cells()
        }

        assert!(compare_cell_content(
            get_formatted_values(100, 100, 0),
            vec!("1", "1", "0")
        ));
        assert!(compare_cell_content(
            get_formatted_values(100, 99, 1),
            vec!("1", "1", "1")
        ));
        assert!(compare_cell_content(
            get_formatted_values(1000, 1000, 0),
            vec!("1", "1", "0")
        ));
        assert!(compare_cell_content(
            get_formatted_values(1001, 1000, 1),
            vec!("2", "1", "1")
        ));
    }

    #[test]
    fn test_row_converter_with_invalid_numbers() {
        init();
        // copy from wsl linux
        let d = crate::Filesystem {
            file: None,
            mount_info: crate::MountInfo {
                dev_id: "28".to_string(),
                dev_name: "none".to_string(),
                fs_type: "9p".to_string(),
                mount_dir: "/usr/lib/wsl/drivers".into(),
                mount_option: "ro,nosuid,nodev,noatime".to_string(),
                mount_root: "/".into(),
                remote: false,
                dummy: false,
            },
            usage: crate::table::FsUsage {
                blocksize: 4096,
                blocks: 244_029_695,
                bfree: 125_085_030,
                bavail: 125_085_030,
                bavail_top_bit_set: false,
                files: 999,
                ffree: 1_000_000,
            },
        };

        let row = Row::from(d);

        assert_eq!(row.inodes_used, 0);
    }

    #[test]
    fn test_table_column_width_computation_include_total_row() {
        init();
        let d1 = crate::Filesystem {
            file: None,
            mount_info: crate::MountInfo {
                dev_id: "28".to_string(),
                dev_name: "none".to_string(),
                fs_type: "9p".to_string(),
                mount_dir: "/usr/lib/wsl/drivers".into(),
                mount_option: "ro,nosuid,nodev,noatime".to_string(),
                mount_root: "/".into(),
                remote: false,
                dummy: false,
            },
            usage: crate::table::FsUsage {
                blocksize: 4096,
                blocks: 244_029_695,
                bfree: 125_085_030,
                bavail: 125_085_030,
                bavail_top_bit_set: false,
                files: 99_999_999_999,
                ffree: 999_999,
            },
        };

        let filesystems = vec![d1.clone(), d1];

        let mut options = Options {
            show_total: true,
            columns: vec![
                Column::Source,
                Column::Itotal,
                Column::Iused,
                Column::Iavail,
            ],
            ..Default::default()
        };

        let table_w_total = Table::new(&options, filesystems.clone());
        let mut data_w_total: Vec<u8> = vec![];
        table_w_total
            .write_to(&mut data_w_total)
            .expect("Write error.");
        assert_eq!(
            String::from_utf8_lossy(&data_w_total),
            "Filesystem           Inodes        IUsed   IFree\n\
             none            99999999999  99999000000  999999\n\
             none            99999999999  99999000000  999999\n\
             total          199999999998 199998000000 1999998\n"
        );

        options.show_total = false;

        let table_w_o_total = Table::new(&options, filesystems);
        let mut data_w_o_total: Vec<u8> = vec![];
        table_w_o_total
            .write_to(&mut data_w_o_total)
            .expect("Write error.");
        assert_eq!(
            String::from_utf8_lossy(&data_w_o_total),
            "Filesystem          Inodes       IUsed  IFree\n\
             none           99999999999 99999000000 999999\n\
             none           99999999999 99999000000 999999\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_table_column_width_non_unicode() {
        init();
        let bad_unicode_os_str = uucore::os_str_from_bytes(b"/usr/lib/w\xf3l/drivers")
            .expect("Only unix platforms can test non-unicode names")
            .to_os_string();
        let d1 = crate::Filesystem {
            file: None,
            mount_info: crate::MountInfo {
                dev_id: "28".to_string(),
                dev_name: "none".to_string(),
                fs_type: "9p".to_string(),
                mount_dir: bad_unicode_os_str,
                mount_option: "ro,nosuid,nodev,noatime".to_string(),
                mount_root: "/".into(),
                remote: false,
                dummy: false,
            },
            usage: crate::table::FsUsage {
                blocksize: 4096,
                blocks: 244_029_695,
                bfree: 125_085_030,
                bavail: 125_085_030,
                bavail_top_bit_set: false,
                files: 99_999_999_999,
                ffree: 999_999,
            },
        };

        let filesystems = vec![d1];

        let options = Options {
            show_total: false,
            columns: vec![Column::Source, Column::Target, Column::Itotal],
            ..Default::default()
        };

        let table = Table::new(&options, filesystems.clone());
        let mut data: Vec<u8> = vec![];
        table.write_to(&mut data).expect("Write error.");
        assert_eq!(
            data,
            b"Filesystem     Mounted on                Inodes\n\
              none           /usr/lib/w\xf3l/drivers 99999999999\n",
            "Comparison failed, lossy data for reference:\n{}\n",
            String::from_utf8_lossy(&data)
        );
    }

    #[test]
    fn test_row_accumulation_u64_overflow() {
        init();
        let total = u64::MAX as u128;
        let used1 = 3000u128;
        let used2 = 50000u128;

        let mut row1 = Row {
            inodes: total,
            inodes_used: used1,
            inodes_free: total - used1,
            ..Default::default()
        };

        let row2 = Row {
            inodes: total,
            inodes_used: used2,
            inodes_free: total - used2,
            ..Default::default()
        };

        row1 += row2;

        assert_eq!(row1.inodes, total * 2);
        assert_eq!(row1.inodes_used, used1 + used2);
        assert_eq!(row1.inodes_free, total * 2 - used1 - used2);
    }
}
