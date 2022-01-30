// spell-checker:ignore formatteriteminfo blocksize thisblock

use std::cmp;
use std::slice::Iter;

use crate::formatteriteminfo::FormatterItemInfo;
use crate::parse_formats::ParsedFormatterItemInfo;

/// Size in bytes of the max datatype. ie set to 16 for 128-bit numbers.
const MAX_BYTES_PER_UNIT: usize = 8;

/// Contains information to output single output line in human readable form
pub struct SpacedFormatterItemInfo {
    /// Contains a function pointer to output data, and information about the output format.
    pub formatter_item_info: FormatterItemInfo,
    /// Contains the number of spaces to add to align data with other output formats.
    ///
    /// If the corresponding data is a single byte, each entry in this array contains
    /// the number of spaces to insert when outputting each byte. If the corresponding
    /// data is multi-byte, only the fist byte position is used. For example a 32-bit
    /// datatype, could use positions 0, 4, 8, 12, ....
    /// As each block is formatted identically, only the spacing for a single block is set.
    pub spacing: [usize; MAX_BYTES_PER_UNIT],
    /// if set adds a ascii dump at the end of the line
    pub add_ascii_dump: bool,
}

/// Contains information about all output lines.
pub struct OutputInfo {
    /// The number of bytes of a line.
    pub byte_size_line: usize,
    /// The width of a line in human readable format.
    pub print_width_line: usize,

    /// The number of bytes in a block. (This is the size of the largest datatype in `spaced_formatters`.)
    pub byte_size_block: usize,
    /// The width of a block in human readable format. (The size of the largest format.)
    pub print_width_block: usize,
    /// All formats.
    spaced_formatters: Vec<SpacedFormatterItemInfo>,
    /// determines if duplicate output lines should be printed, or
    /// skipped with a "*" showing one or more skipped lines.
    pub output_duplicates: bool,
}

impl OutputInfo {
    /// Returns an iterator over the `SpacedFormatterItemInfo` vector.
    pub fn spaced_formatters_iter(&self) -> Iter<SpacedFormatterItemInfo> {
        self.spaced_formatters.iter()
    }

    /// Creates a new `OutputInfo` based on the parameters
    pub fn new(
        line_bytes: usize,
        formats: &[ParsedFormatterItemInfo],
        output_duplicates: bool,
    ) -> Self {
        let byte_size_block = formats.iter().fold(1, |max, next| {
            cmp::max(max, next.formatter_item_info.byte_size)
        });
        let print_width_block = formats.iter().fold(1, |max, next| {
            cmp::max(
                max,
                next.formatter_item_info.print_width
                    * (byte_size_block / next.formatter_item_info.byte_size),
            )
        });
        let print_width_line = print_width_block * (line_bytes / byte_size_block);

        let spaced_formatters =
            Self::create_spaced_formatter_info(formats, byte_size_block, print_width_block);

        Self {
            byte_size_line: line_bytes,
            print_width_line,
            byte_size_block,
            print_width_block,
            spaced_formatters,
            output_duplicates,
        }
    }

    fn create_spaced_formatter_info(
        formats: &[ParsedFormatterItemInfo],
        byte_size_block: usize,
        print_width_block: usize,
    ) -> Vec<SpacedFormatterItemInfo> {
        formats
            .iter()
            .map(|f| SpacedFormatterItemInfo {
                formatter_item_info: f.formatter_item_info,
                add_ascii_dump: f.add_ascii_dump,
                spacing: Self::calculate_alignment(f, byte_size_block, print_width_block),
            })
            .collect()
    }

    /// calculates proper alignment for a single line of output
    ///
    /// Multiple representations of the same data, will be right-aligned for easy reading.
    /// For example a 64 bit octal and a 32-bit decimal with a 16-bit hexadecimal looks like this:
    /// ```ignore
    /// 1777777777777777777777 1777777777777777777777
    ///  4294967295 4294967295  4294967295 4294967295
    ///   ffff ffff  ffff ffff   ffff ffff  ffff ffff
    /// ```
    /// In this example is additional spacing before the first and third decimal number,
    /// and there is additional spacing before the 1st, 3rd, 5th and 7th hexadecimal number.
    /// This way both the octal and decimal, as well as the decimal and hexadecimal numbers
    /// left align. Note that the alignment below both octal numbers is identical.
    ///
    /// This function calculates the required spacing for a single line, given the size
    /// of a block, and the width of a block. The size of a block is the largest type
    /// and the width is width of the the type which needs the most space to print that
    /// number of bytes. So both numbers might refer to different types. All widths
    /// include a space at the front. For example the width of a 8-bit hexadecimal,
    /// is 3 characters, for example " FF".
    ///
    /// This algorithm first calculates how many spaces needs to be added, based the
    /// block size and the size of the type, and the widths of the block and the type.
    /// The required spaces are spread across the available positions.
    /// If the blocksize is 8, and the size of the type is 8 too, there will be just
    /// one value in a block, so all spacing will be assigned to position 0.
    /// If the blocksize is 8, and the size of the type is 2, the spacing will be
    /// spread across position 0, 2, 4, 6. All 4 positions will get an additional
    /// space as long as there are more then 4 spaces available. If there are 2
    /// spaces available, they will be assigned to position 0 and 4. If there is
    /// 1 space available, it will be assigned to position 0. This will be combined,
    /// For example 7 spaces will be assigned to position 0, 2, 4, 6 like: 3, 1, 2, 1.
    /// And 7 spaces with 2 positions will be assigned to position 0 and 4 like 4, 3.
    ///
    /// Here is another example showing the alignment of 64-bit unsigned decimal numbers,
    /// 32-bit hexadecimal number, 16-bit octal numbers and 8-bit hexadecimal numbers:
    /// ```ignore
    ///        18446744073709551615        18446744073709551615
    ///      ffffffff      ffffffff      ffffffff      ffffffff
    /// 177777 177777 177777 177777 177777 177777 177777 177777
    ///  ff ff  ff ff  ff ff  ff ff  ff ff  ff ff  ff ff  ff ff
    /// ```
    ///
    /// This algorithm assumes the size of all types is a power of 2 (1, 2, 4, 8, 16, ...)
    /// Increase MAX_BYTES_PER_UNIT to allow larger types.
    fn calculate_alignment(
        sf: &dyn TypeSizeInfo,
        byte_size_block: usize,
        print_width_block: usize,
    ) -> [usize; MAX_BYTES_PER_UNIT] {
        assert!(
            byte_size_block <= MAX_BYTES_PER_UNIT,
            "{}-bits types are unsupported. Current max={}-bits.",
            8 * byte_size_block,
            8 * MAX_BYTES_PER_UNIT
        );
        let mut spacing = [0; MAX_BYTES_PER_UNIT];

        let mut byte_size = sf.byte_size();
        let mut items_in_block = byte_size_block / byte_size;
        let thisblock_width = sf.print_width() * items_in_block;
        let mut missing_spacing = print_width_block - thisblock_width;

        while items_in_block > 0 {
            let avg_spacing: usize = missing_spacing / items_in_block;
            for i in 0..items_in_block {
                spacing[i * byte_size] += avg_spacing;
                missing_spacing -= avg_spacing;
            }

            items_in_block /= 2;
            byte_size *= 2;
        }

        spacing
    }
}

trait TypeSizeInfo {
    fn byte_size(&self) -> usize;
    fn print_width(&self) -> usize;
}

impl TypeSizeInfo for ParsedFormatterItemInfo {
    fn byte_size(&self) -> usize {
        self.formatter_item_info.byte_size
    }
    fn print_width(&self) -> usize {
        self.formatter_item_info.print_width
    }
}

#[cfg(test)]
struct TypeInfo {
    byte_size: usize,
    print_width: usize,
}

#[cfg(test)]
impl TypeSizeInfo for TypeInfo {
    fn byte_size(&self) -> usize {
        self.byte_size
    }
    fn print_width(&self) -> usize {
        self.print_width
    }
}

#[test]
fn test_calculate_alignment() {
    // For this example `byte_size_block` is 8 and 'print_width_block' is 23:
    // 1777777777777777777777 1777777777777777777777
    //  4294967295 4294967295  4294967295 4294967295
    //   ffff ffff  ffff ffff   ffff ffff  ffff ffff

    // the first line has no additional spacing:
    assert_eq!(
        [0, 0, 0, 0, 0, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 8,
                print_width: 23,
            },
            8,
            23
        )
    );
    // the second line a single space at the start of the block:
    assert_eq!(
        [1, 0, 0, 0, 0, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 4,
                print_width: 11,
            },
            8,
            23
        )
    );
    // the third line two spaces at pos 0, and 1 space at pos 4:
    assert_eq!(
        [2, 0, 0, 0, 1, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 2,
                print_width: 5,
            },
            8,
            23
        )
    );

    // For this example `byte_size_block` is 8 and 'print_width_block' is 28:
    //        18446744073709551615        18446744073709551615
    //      ffffffff      ffffffff      ffffffff      ffffffff
    // 177777 177777 177777 177777 177777 177777 177777 177777
    //  ff ff  ff ff  ff ff  ff ff  ff ff  ff ff  ff ff  ff ff

    assert_eq!(
        [7, 0, 0, 0, 0, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 8,
                print_width: 21,
            },
            8,
            28
        )
    );
    assert_eq!(
        [5, 0, 0, 0, 5, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 4,
                print_width: 9,
            },
            8,
            28
        )
    );
    assert_eq!(
        [0, 0, 0, 0, 0, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 2,
                print_width: 7,
            },
            8,
            28
        )
    );
    assert_eq!(
        [1, 0, 1, 0, 1, 0, 1, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 3,
            },
            8,
            28
        )
    );

    // 9 tests where 8 .. 16 spaces are spread across 8 positions
    assert_eq!(
        [1, 1, 1, 1, 1, 1, 1, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 8
        )
    );
    assert_eq!(
        [2, 1, 1, 1, 1, 1, 1, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 9
        )
    );
    assert_eq!(
        [2, 1, 1, 1, 2, 1, 1, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 10
        )
    );
    assert_eq!(
        [3, 1, 1, 1, 2, 1, 1, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 11
        )
    );
    assert_eq!(
        [2, 1, 2, 1, 2, 1, 2, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 12
        )
    );
    assert_eq!(
        [3, 1, 2, 1, 2, 1, 2, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 13
        )
    );
    assert_eq!(
        [3, 1, 2, 1, 3, 1, 2, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 14
        )
    );
    assert_eq!(
        [4, 1, 2, 1, 3, 1, 2, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 15
        )
    );
    assert_eq!(
        [2, 2, 2, 2, 2, 2, 2, 2],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 16
        )
    );

    // 4 tests where 15 spaces are spread across 8, 4, 2 or 1 position(s)
    assert_eq!(
        [4, 1, 2, 1, 3, 1, 2, 1],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 1,
                print_width: 2,
            },
            8,
            16 + 15
        )
    );
    assert_eq!(
        [5, 0, 3, 0, 4, 0, 3, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 2,
                print_width: 4,
            },
            8,
            16 + 15
        )
    );
    assert_eq!(
        [8, 0, 0, 0, 7, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 4,
                print_width: 8,
            },
            8,
            16 + 15
        )
    );
    assert_eq!(
        [15, 0, 0, 0, 0, 0, 0, 0],
        OutputInfo::calculate_alignment(
            &TypeInfo {
                byte_size: 8,
                print_width: 16,
            },
            8,
            16 + 15
        )
    );
}
