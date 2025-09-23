// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Common benchmark utilities for uutils coreutils
//!
//! This module provides shared functionality for benchmarking utilities,
//! including test data generation and binary execution helpers.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Create a temporary file with test data
pub fn create_test_file(data: &[u8], temp_dir: &Path) -> PathBuf {
    let file_path = temp_dir.join("test_data.txt");
    let file = File::create(&file_path).unwrap();
    let mut writer = BufWriter::new(file);
    writer.write_all(data).unwrap();
    writer.flush().unwrap();
    file_path
}

/// Run a uutils binary with given arguments using the coreutils multicall binary
pub fn run_uutils_binary(util_name: &str, args: &[&str]) -> i32 {
    use std::process::{Command, Stdio};

    // Use the multicall binary
    let output = Command::new("../../../target/release/coreutils")
        .args([util_name].iter().chain(args.iter()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to execute command");

    i32::from(!output.success())
}

/// Generate test data with different characteristics for text processing utilities
pub mod text_data {
    /// Generate test data with a specific size in MB and average line length
    pub fn generate_by_size(size_mb: usize, avg_line_length: usize) -> Vec<u8> {
        let total_size = size_mb * 1024 * 1024;
        let mut data = Vec::with_capacity(total_size);

        let mut current_size = 0;
        let mut line_chars = 0;

        while current_size < total_size {
            if line_chars >= avg_line_length {
                data.push(b'\n');
                line_chars = 0;
            } else {
                // Use various ASCII characters to make it realistic
                data.push(b'a' + (current_size % 26) as u8);
                line_chars += 1;
            }
            current_size += 1;
        }

        // Ensure we end with a newline
        if data.last() != Some(&b'\n') {
            data.push(b'\n');
        }

        data
    }

    /// Generate test data by line count instead of size
    pub fn generate_by_lines(num_lines: usize, avg_line_length: usize) -> Vec<u8> {
        let mut data = Vec::new();

        for line_num in 0..num_lines {
            // Vary line length slightly for realism
            let line_length = avg_line_length + (line_num % 40).saturating_sub(20);

            for char_pos in 0..line_length {
                // Create more realistic text with spaces
                if char_pos > 0 && char_pos % 8 == 0 {
                    data.push(b' '); // Add spaces every 8 characters
                } else {
                    // Cycle through letters with some variation
                    let char_offset = (line_num + char_pos) % 26;
                    data.push(b'a' + char_offset as u8);
                }
            }
            data.push(b'\n');
        }

        data
    }
}
