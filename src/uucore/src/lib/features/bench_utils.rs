// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Shared utilities for benchmarking uutils commands
//!
//! This module provides common functionality used across benchmark files
//! to reduce code duplication and standardize benchmark implementations.

#[cfg(feature = "bench-utils")]
pub mod shared {
    pub use divan::{Bencher, black_box};
    pub use std::fs::File;
    pub use std::io::{BufWriter, Write};

    // Re-export TempDir from standard library path
    pub use std::path::PathBuf;

    // Create our own TempDir wrapper to avoid dependency issues
    pub struct TempDir(std::path::PathBuf);

    impl TempDir {
        pub fn new() -> std::io::Result<Self> {
            use std::env;
            let mut path = env::temp_dir();
            path.push(format!("uutils_bench_{}", std::process::id()));
            std::fs::create_dir_all(&path)?;
            Ok(TempDir(path))
        }

        pub fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Create a temporary file with test data
    pub fn create_test_file(data: &[u8], temp_dir: &TempDir, suffix: &str) -> std::path::PathBuf {
        let file_path = temp_dir.path().join(format!("test_data{}.txt", suffix));
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);
        writer.write_all(data).unwrap();
        writer.flush().unwrap();
        file_path
    }

    /// Run a uutils command with given arguments via the multicall binary
    pub fn run_uutils_command(command: &str, args: &[&str]) -> i32 {
        use std::process::{Command, Stdio};

        // Use the multicall binary instead of calling uumain directly to avoid stdout issues
        let output = Command::new("../../../target/release/coreutils")
            .args([command].iter().chain(args.iter()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap_or_else(|_| panic!("Failed to execute {} command", command));

        i32::from(!output.success())
    }

    /// Generate test data with different characteristics for text processing
    pub fn generate_text_data(size_mb: usize, avg_line_length: usize) -> Vec<u8> {
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
    pub fn generate_text_data_by_lines(num_lines: usize, avg_line_length: usize) -> Vec<u8> {
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

    /// Generate pseudo-random binary data for encoding/binary processing benchmarks
    pub fn generate_binary_data(size_mb: usize) -> Vec<u8> {
        let total_size = size_mb * 1024 * 1024;
        let mut data = Vec::with_capacity(total_size);

        // Generate pseudo-random binary data for more realistic base64 encoding
        for i in 0..total_size {
            data.push((i ^ (i >> 8) ^ (i >> 16)) as u8);
        }

        data
    }

    /// Generate specific data patterns for testing different scenarios
    pub fn generate_data_pattern(size_mb: usize, pattern: DataPattern) -> Vec<u8> {
        let total_size = size_mb * 1024 * 1024;

        match pattern {
            DataPattern::Zeros => vec![0u8; total_size],
            DataPattern::RepeatingPattern => {
                let mut data = Vec::with_capacity(total_size);
                let pattern_bytes = b"ABCDEFGHIJKLMNOP";
                for _ in 0..(total_size / pattern_bytes.len()) {
                    data.extend_from_slice(pattern_bytes);
                }
                // Fill remainder
                let remainder = total_size % pattern_bytes.len();
                data.extend_from_slice(&pattern_bytes[..remainder]);
                data
            }
            DataPattern::Random => generate_binary_data(size_mb),
        }
    }

    /// Different data patterns for testing
    #[derive(Clone, Copy, Debug)]
    pub enum DataPattern {
        /// All zeros (highly compressible)
        Zeros,
        /// Repeating 16-byte pattern
        RepeatingPattern,
        /// Pseudo-random data
        Random,
    }

    impl std::fmt::Display for DataPattern {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DataPattern::Zeros => write!(f, "zeros"),
                DataPattern::RepeatingPattern => write!(f, "pattern"),
                DataPattern::Random => write!(f, "random"),
            }
        }
    }

    impl std::str::FromStr for DataPattern {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "zeros" => Ok(DataPattern::Zeros),
                "pattern" => Ok(DataPattern::RepeatingPattern),
                "random" => Ok(DataPattern::Random),
                _ => Err(format!("Unknown pattern: {}", s)),
            }
        }
    }
}
