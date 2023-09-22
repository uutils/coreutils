// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore dired subdired

use crate::Config;
use std::fmt;
use std::io::{BufWriter, Stdout, Write};
use uucore::error::UResult;

#[derive(Debug, Clone)]
pub struct BytePosition {
    pub start: usize,
    pub end: usize,
}

/// Represents the output structure for DIRED, containing positions for both DIRED and SUBDIRED.
#[derive(Debug, Clone, Default)]
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

/// Calculates the byte positions for DIRED
pub fn calculate_dired(
    output_display_len: usize,
    dfn_len: usize,
    dired_positions: &[BytePosition],
) -> (usize, usize) {
    let offset_from_previous_line = if let Some(last_position) = dired_positions.last() {
        last_position.end + 1
    } else {
        0
    };

    let start = output_display_len + offset_from_previous_line;
    let end = start + dfn_len;
    (start, end)
}

pub fn indent(out: &mut BufWriter<Stdout>) -> UResult<()> {
    write!(out, "  ")?;
    Ok(())
}

pub fn calculate_subdired(dired: &mut DiredOutput, path_len: usize) {
    let offset = if dired.subdired_positions.is_empty() {
        DIRED_TRAILING_OFFSET
    } else {
        dired.subdired_positions[dired.subdired_positions.len() - 1].start + DIRED_TRAILING_OFFSET
    };
    dired.subdired_positions.push(BytePosition {
        start: offset,
        end: path_len + offset,
    });
}

/// Prints the dired output based on the given configuration and dired structure.
pub fn print_dired_output(
    config: &Config,
    dired: &DiredOutput,
    out: &mut BufWriter<Stdout>,
) -> UResult<()> {
    out.flush()?;
    if config.recursive {
        print_positions("//SUBDIRED//", &dired.subdired_positions);
    } else if dired.padding == 0 {
        print_positions("//DIRED//", &dired.dired_positions);
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

pub fn add_total(total_len: usize, dired: &mut DiredOutput) {
    // when dealing with "  total: xx", it isn't part of the //DIRED//
    // so, we just keep the size line to add it to the position of the next file
    dired.padding = total_len + DIRED_TRAILING_OFFSET;
}

/// Calculates byte positions and updates the dired structure.
pub fn calculate_and_update_positions(
    output_display_len: usize,
    dfn_len: usize,
    dired: &mut DiredOutput,
) {
    let offset = dired
        .dired_positions
        .last()
        .map_or(DIRED_TRAILING_OFFSET, |last_position| {
            last_position.start + DIRED_TRAILING_OFFSET
        });
    let start = output_display_len + offset + DIRED_TRAILING_OFFSET;
    let end = start + dfn_len;
    update_positions(start, end, dired);
}

/// Updates the dired positions based on the given start and end positions.
/// update when it is the first element in the list (to manage "total X")
/// insert when it isn't the about total
pub fn update_positions(start: usize, end: usize, dired: &mut DiredOutput) {
    // padding can be 0 but as it doesn't matter<
    dired.dired_positions.push(BytePosition {
        start: start + dired.padding,
        end: end + dired.padding,
    });
    // Remove the previous padding
    dired.padding = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dired() {
        let output_display = "sample_output".to_string();
        let dfn = "sample_file".to_string();
        let dired_positions = vec![BytePosition { start: 5, end: 10 }];
        let (start, end) = calculate_dired(output_display.len(), dfn.len(), &dired_positions);

        assert_eq!(start, 24);
        assert_eq!(end, 35);
    }

    #[test]
    fn test_dired_update_positions() {
        let mut dired = DiredOutput {
            dired_positions: vec![BytePosition { start: 5, end: 10 }],
            subdired_positions: vec![],
            padding: 10,
        };

        // Test with adjust = true
        update_positions(15, 20, &mut dired);
        let last_position = dired.dired_positions.last().unwrap();
        assert_eq!(last_position.start, 25); // 15 + 10 (end of the previous position)
        assert_eq!(last_position.end, 30); // 20 + 10 (end of the previous position)

        // Test with adjust = false
        update_positions(30, 35, &mut dired);
        let last_position = dired.dired_positions.last().unwrap();
        assert_eq!(last_position.start, 30);
        assert_eq!(last_position.end, 35);
    }
}
