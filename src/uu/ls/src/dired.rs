// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore dired subdired

/// `dired` Module Documentation
///
/// This module handles the --dired output format, representing file and
/// directory listings.
///
/// Key Mechanisms:
/// 1. **Position Tracking**:
///    - The module tracks byte positions for each file or directory entry.
///    - `BytePosition`: Represents a byte range with start and end positions.
///    - `DiredOutput`: Contains positions for DIRED and SUBDIRED outputs and
///      maintains a padding value.
///
/// 2. **Padding**:
///    - Padding is used when dealing with directory names or the "total" line.
///    - The module adjusts byte positions by adding padding for these cases.
///    - This ensures correct offset for subsequent files or directories.
///
/// 3. **Position Calculation**:
///    - Functions like `calculate_dired`, `calculate_subdired`, and
///      `calculate_and_update_positions` compute byte positions based on output
///      length, previous positions, and padding.
///
/// 4. **Output**:
///    - The module provides functions to print the DIRED output
///      (`print_dired_output`) based on calculated positions and configuration.
///    - Helpers like `print_positions` print positions with specific prefixes.
///
/// Overall, the module ensures each entry in the DIRED output has the correct
/// byte position, considering additional lines or padding affecting positions.
///
use crate::Config;
use std::fmt;
use std::io::{BufWriter, Stdout, Write};
use uucore::error::UResult;

#[derive(Debug, Clone, PartialEq)]
pub struct BytePosition {
    pub start: usize,
    pub end: usize,
}

/// Represents the output structure for DIRED, containing positions for both DIRED and SUBDIRED.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiredOutput {
    pub dired_positions: Vec<BytePosition>,
    pub subdired_positions: Vec<BytePosition>,
    pub padding: usize,
}

impl fmt::Display for BytePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.start, self.end)
    }
}

// When --dired is used, all lines starts with 2 spaces
static DIRED_TRAILING_OFFSET: usize = 2;

fn get_offset_from_previous_line(dired_positions: &[BytePosition]) -> usize {
    if let Some(last_position) = dired_positions.last() {
        last_position.end + 1
    } else {
        0
    }
}

/// Calculates the byte positions for DIRED
pub fn calculate_dired(
    dired_positions: &[BytePosition],
    output_display_len: usize,
    dfn_len: usize,
) -> (usize, usize) {
    let offset_from_previous_line = get_offset_from_previous_line(dired_positions);

    let start = output_display_len + offset_from_previous_line;
    let end = start + dfn_len;
    (start, end)
}

pub fn indent(out: &mut BufWriter<Stdout>) -> UResult<()> {
    write!(out, "  ")?;
    Ok(())
}

pub fn calculate_subdired(dired: &mut DiredOutput, path_len: usize) {
    let offset_from_previous_line = get_offset_from_previous_line(&dired.dired_positions);

    let additional_offset = if dired.subdired_positions.is_empty() {
        0
    } else {
        // if we have several directories: \n\n
        2
    };

    let start = offset_from_previous_line + DIRED_TRAILING_OFFSET + additional_offset;
    let end = start + path_len;
    dired.subdired_positions.push(BytePosition { start, end });
}

/// Prints the dired output based on the given configuration and dired structure.
pub fn print_dired_output(
    config: &Config,
    dired: &DiredOutput,
    out: &mut BufWriter<Stdout>,
) -> UResult<()> {
    out.flush()?;
    if !dired.dired_positions.is_empty() {
        print_positions("//DIRED//", &dired.dired_positions);
    }
    if config.recursive {
        print_positions("//SUBDIRED//", &dired.subdired_positions);
    }
    println!("//DIRED-OPTIONS// --quoting-style={}", config.quoting_style);
    Ok(())
}

/// Helper function to print positions with a given prefix.
fn print_positions(prefix: &str, positions: &Vec<BytePosition>) {
    print!("{}", prefix);
    for c in positions {
        print!(" {}", c);
    }
    println!();
}

pub fn add_total(dired: &mut DiredOutput, total_len: usize) {
    if dired.padding == 0 {
        let offset_from_previous_line = get_offset_from_previous_line(&dired.dired_positions);
        // when dealing with "  total: xx", it isn't part of the //DIRED//
        // so, we just keep the size line to add it to the position of the next file
        dired.padding = total_len + offset_from_previous_line + DIRED_TRAILING_OFFSET;
    } else {
        // += because if we are in -R, we have "  dir:\n  total X". So, we need to take the
        // previous padding too.
        // and we already have the previous position in mind
        dired.padding += total_len + DIRED_TRAILING_OFFSET;
    }
}

// when using -R, we have the dirname. we need to add it to the padding
pub fn add_dir_name(dired: &mut DiredOutput, dir_len: usize) {
    // 1 for the ":" in "  dirname:"
    dired.padding += dir_len + DIRED_TRAILING_OFFSET + 1;
}

/// Calculates byte positions and updates the dired structure.
pub fn calculate_and_update_positions(
    dired: &mut DiredOutput,
    output_display_len: usize,
    dfn_len: usize,
) {
    let offset = dired
        .dired_positions
        .last()
        .map_or(DIRED_TRAILING_OFFSET, |last_position| {
            last_position.start + DIRED_TRAILING_OFFSET
        });
    let start = output_display_len + offset + DIRED_TRAILING_OFFSET;
    let end = start + dfn_len;
    update_positions(dired, start, end);
}

/// Updates the dired positions based on the given start and end positions.
/// update when it is the first element in the list (to manage "total X")
/// insert when it isn't the about total
pub fn update_positions(dired: &mut DiredOutput, start: usize, end: usize) {
    // padding can be 0 but as it doesn't matter
    dired.dired_positions.push(BytePosition {
        start: start + dired.padding,
        end: end + dired.padding,
    });
    // Remove the previous padding
    dired.padding = 0;
}

/// Checks if the "--dired" or "-D" argument is present in the command line arguments.
/// we don't use clap here because we need to know if the argument is present
/// as it can be overridden by --hyperlink
pub fn is_dired_arg_present() -> bool {
    let args: Vec<String> = std::env::args().collect();
    args.iter().any(|x| x == "--dired" || x == "-D")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dired() {
        let output_display = "sample_output".to_string();
        let dfn = "sample_file".to_string();
        let dired_positions = vec![BytePosition { start: 5, end: 10 }];
        let (start, end) = calculate_dired(&dired_positions, output_display.len(), dfn.len());

        assert_eq!(start, 24);
        assert_eq!(end, 35);
    }

    #[test]
    fn test_get_offset_from_previous_line() {
        let positions = vec![
            BytePosition { start: 0, end: 3 },
            BytePosition { start: 4, end: 7 },
            BytePosition { start: 8, end: 11 },
        ];
        assert_eq!(get_offset_from_previous_line(&positions), 12);
    }
    #[test]
    fn test_calculate_subdired() {
        let mut dired = DiredOutput {
            dired_positions: vec![
                BytePosition { start: 0, end: 3 },
                BytePosition { start: 4, end: 7 },
                BytePosition { start: 8, end: 11 },
            ],
            subdired_positions: vec![],
            padding: 0,
        };
        let path_len = 5;
        calculate_subdired(&mut dired, path_len);
        assert_eq!(
            dired.subdired_positions,
            vec![BytePosition { start: 14, end: 19 }],
        );
    }

    #[test]
    fn test_add_dir_name() {
        let mut dired = DiredOutput {
            dired_positions: vec![
                BytePosition { start: 0, end: 3 },
                BytePosition { start: 4, end: 7 },
                BytePosition { start: 8, end: 11 },
            ],
            subdired_positions: vec![],
            padding: 0,
        };
        let dir_len = 5;
        add_dir_name(&mut dired, dir_len);
        assert_eq!(
            dired,
            DiredOutput {
                dired_positions: vec![
                    BytePosition { start: 0, end: 3 },
                    BytePosition { start: 4, end: 7 },
                    BytePosition { start: 8, end: 11 },
                ],
                subdired_positions: vec![],
                // 8 = 1 for the \n + 5 for dir_len + 2 for "  " + 1 for :
                padding: 8
            }
        );
    }

    #[test]
    fn test_add_total() {
        let mut dired = DiredOutput {
            dired_positions: vec![
                BytePosition { start: 0, end: 3 },
                BytePosition { start: 4, end: 7 },
                BytePosition { start: 8, end: 11 },
            ],
            subdired_positions: vec![],
            padding: 0,
        };
        // if we have "total: 2"
        let total_len = 8;
        add_total(&mut dired, total_len);
        // 22 = 8 (len) + 2 (padding) + 11 (previous position) + 1 (\n)
        assert_eq!(dired.padding, 22);
    }

    #[test]
    fn test_add_dir_name_and_total() {
        // test when we have
        //   dirname:
        //   total 0
        //   -rw-r--r-- 1 sylvestre sylvestre 0 Sep 30 09:41 ab

        let mut dired = DiredOutput {
            dired_positions: vec![
                BytePosition { start: 0, end: 3 },
                BytePosition { start: 4, end: 7 },
                BytePosition { start: 8, end: 11 },
            ],
            subdired_positions: vec![],
            padding: 0,
        };
        let dir_len = 5;
        add_dir_name(&mut dired, dir_len);
        // 8 = 2 ("  ") + 1 (\n) + 5 + 1 (: of dirname)
        assert_eq!(dired.padding, 8);

        let total_len = 8;
        add_total(&mut dired, total_len);
        assert_eq!(dired.padding, 18);
    }

    #[test]
    fn test_dired_update_positions() {
        let mut dired = DiredOutput {
            dired_positions: vec![BytePosition { start: 5, end: 10 }],
            subdired_positions: vec![],
            padding: 10,
        };

        // Test with adjust = true
        update_positions(&mut dired, 15, 20);
        let last_position = dired.dired_positions.last().unwrap();
        assert_eq!(last_position.start, 25); // 15 + 10 (end of the previous position)
        assert_eq!(last_position.end, 30); // 20 + 10 (end of the previous position)

        // Test with adjust = false
        update_positions(&mut dired, 30, 35);
        let last_position = dired.dired_positions.last().unwrap();
        assert_eq!(last_position.start, 30);
        assert_eq!(last_position.end, 35);
    }

    #[test]
    fn test_calculate_and_update_positions() {
        let mut dired = DiredOutput {
            dired_positions: vec![
                BytePosition { start: 0, end: 3 },
                BytePosition { start: 4, end: 7 },
                BytePosition { start: 8, end: 11 },
            ],
            subdired_positions: vec![],
            padding: 5,
        };
        let output_display_len = 15;
        let dfn_len = 5;
        calculate_and_update_positions(&mut dired, output_display_len, dfn_len);
        assert_eq!(
            dired.dired_positions,
            vec![
                BytePosition { start: 0, end: 3 },
                BytePosition { start: 4, end: 7 },
                BytePosition { start: 8, end: 11 },
                BytePosition { start: 32, end: 37 },
            ]
        );
        assert_eq!(dired.padding, 0);
    }
}
