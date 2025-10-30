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
    // Ensure data is fully written to disk before returning
    std::mem::drop(writer);
    File::open(&file_path).unwrap().sync_all().unwrap();
    file_path
}

/// Run a utility function directly with given arguments
/// This calls the uumain function that returns i32 (like the fuzzing approach)
pub fn run_util_function<F>(util_func: F, args: &[&str]) -> i32
where
    F: FnOnce(std::vec::IntoIter<std::ffi::OsString>) -> i32,
{
    // Prepend a dummy program name as argv[0] since clap expects it
    let mut os_args: Vec<std::ffi::OsString> = vec!["benchmark".into()];
    os_args.extend(args.iter().map(|s| (*s).into()));
    util_func(os_args.into_iter())
}

/// Helper function to set up a temporary test file and leak the temporary directory
/// so it persists for the duration of the benchmark
pub fn setup_test_file(data: &[u8]) -> PathBuf {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_test_file(data, temp_dir.path());
    // Keep temp_dir alive by leaking it - the OS will clean it up
    std::mem::forget(temp_dir);
    file_path
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

    /// Helper function to generate test data from a list of words
    pub fn generate_data_from_words(words: &[&str], num_lines: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..num_lines {
            let word = words[i % words.len()];
            let number = i % 1000;
            data.extend_from_slice(format!("{word}_{number:03}\n").as_bytes());
        }
        data
    }

    /// Helper function to generate test data from a list of words without number suffix
    pub fn generate_data_from_words_simple(words: &[&str], num_lines: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..num_lines {
            let word = words[i % words.len()];
            data.extend_from_slice(format!("{word}\n").as_bytes());
        }
        data
    }

    /// Helper function to generate test data from a list of words with counter
    pub fn generate_data_from_words_with_counter(words: &[&str], num_lines: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..num_lines {
            let word = words[i % words.len()];
            let line = format!("{word}{i:04}\n");
            data.extend_from_slice(line.as_bytes());
        }
        data
    }

    /// Generate test data with ASCII-only text
    pub fn generate_ascii_data(num_lines: usize) -> Vec<u8> {
        let words = [
            "apple",
            "banana",
            "cherry",
            "date",
            "elderberry",
            "fig",
            "grape",
            "honeydew",
            "kiwi",
            "lemon",
            "mango",
            "nectarine",
            "orange",
            "papaya",
            "quince",
            "raspberry",
            "strawberry",
            "tangerine",
            "ugli",
            "vanilla",
            "watermelon",
            "xigua",
            "yellow",
            "zucchini",
            "avocado",
        ];

        generate_data_from_words(&words, num_lines)
    }

    /// Generate simple ASCII data with line numbers
    pub fn generate_ascii_data_simple(num_lines: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..num_lines {
            let line = format!("line_{:06}\n", (num_lines - i - 1));
            data.extend_from_slice(line.as_bytes());
        }
        data
    }

    /// Generate test data with accented characters that require locale-aware sorting
    pub fn generate_accented_data(num_lines: usize) -> Vec<u8> {
        let words = [
            // French words with accents
            "café",
            "naïve",
            "résumé",
            "fiancé",
            "crème",
            "déjà",
            "façade",
            "château",
            "élève",
            "côte",
            // German words with umlauts
            "über",
            "Müller",
            "schön",
            "Köln",
            "Düsseldorf",
            "Österreich",
            "Zürich",
            "Mädchen",
            "Bär",
            "größer",
            // Spanish words with tildes and accents
            "niño",
            "señor",
            "año",
            "mañana",
            "español",
            "corazón",
            "María",
            "José",
            "más",
            "también",
        ];

        generate_data_from_words(&words, num_lines)
    }

    /// Generate test data with mixed ASCII and non-ASCII characters
    pub fn generate_mixed_data(num_lines: usize) -> Vec<u8> {
        let words = [
            // Mix of ASCII and accented words
            "apple",
            "café",
            "banana",
            "naïve",
            "cherry",
            "résumé",
            "date",
            "fiancé",
            "elderberry",
            "crème",
            "über",
            "grape",
            "Müller",
            "honeydew",
            "schön",
            "niño",
            "kiwi",
            "señor",
            "lemon",
            "año",
            "mango",
            "María",
            "orange",
            "José",
            "papaya",
        ];

        generate_data_from_words(&words, num_lines)
    }

    /// Generate mixed locale data with counter
    pub fn generate_mixed_locale_data(num_lines: usize) -> Vec<u8> {
        let mixed_strings = [
            "zebra", "äpfel", "banana", "öl", "cat", "über", "dog", "zürich", "elephant", "café",
            "fish", "naïve", "grape", "résumé", "house", "piñata",
        ];
        generate_data_from_words_with_counter(&mixed_strings, num_lines)
    }

    /// Generate German locale-specific data
    pub fn generate_german_locale_data(num_lines: usize) -> Vec<u8> {
        let german_words = [
            "Ärger", "Öffnung", "Über", "Zucker", "Bär", "Föhn", "Größe", "Höhe", "Käse", "Löwe",
            "Mädchen", "Nüsse", "Röntgen", "Schäfer", "Tür", "Würfel", "ä", "ö", "ü", "ß", "a",
            "o", "u", "s",
        ];
        generate_data_from_words_with_counter(&german_words, num_lines)
    }

    /// Generate test data with uppercase/lowercase variations
    pub fn generate_case_sensitive_data(num_lines: usize) -> Vec<u8> {
        let base_words = [
            "apple", "Apple", "APPLE", "banana", "Banana", "BANANA", "café", "Café", "CAFÉ",
            "über", "Über", "ÜBER",
        ];

        generate_data_from_words_simple(&base_words, num_lines)
    }

    /// Generate numeric data for benchmarking (simple sequential numbers)
    pub fn generate_numbers(count: usize) -> String {
        (1..=count)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Filesystem tree generation utilities for benchmarking
pub mod fs_tree {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    /// Create a balanced directory tree for benchmarking
    ///
    /// Creates a tree with specified depth, number of directories per level, and files per directory.
    /// This creates a realistic filesystem structure for testing recursive operations.
    pub fn create_balanced_tree(
        base_dir: &Path,
        depth: usize,
        dirs_per_level: usize,
        files_per_dir: usize,
    ) {
        if depth == 0 {
            return;
        }

        // Create files in current directory
        for file_idx in 0..files_per_dir {
            let file_path = base_dir.join(format!("f{file_idx}"));
            File::create(&file_path).unwrap();
        }

        // Create subdirectories and recurse
        for dir_idx in 0..dirs_per_level {
            let dir_path = base_dir.join(format!("d{dir_idx}"));
            fs::create_dir(&dir_path).unwrap();
            create_balanced_tree(&dir_path, depth - 1, dirs_per_level, files_per_dir);
        }
    }

    /// Create a wide directory tree (many files/dirs at shallow depth)
    ///
    /// This creates a flat structure with many files and directories at a shallow depth,
    /// useful for benchmarking operations that need to traverse many entries quickly.
    pub fn create_wide_tree(base_dir: &Path, total_files: usize, total_dirs: usize) {
        // Create many files in root
        for file_idx in 0..total_files {
            let file_path = base_dir.join(format!("f{file_idx}"));
            File::create(&file_path).unwrap();
        }

        // Create many directories with few files each
        for dir_idx in 0..total_dirs {
            let dir_path = base_dir.join(format!("d{dir_idx}"));
            fs::create_dir(&dir_path).unwrap();
            for file_idx in 0..5 {
                File::create(dir_path.join(format!("f{file_idx}"))).unwrap();
            }
        }
    }

    /// Create a deep directory tree (deep nesting)
    ///
    /// This creates a linear chain of deeply nested directories, useful for testing
    /// recursion depth handling and stack usage.
    pub fn create_deep_tree(base_dir: &Path, depth: usize, files_per_level: usize) {
        let mut current_dir = base_dir.to_path_buf();

        for level in 0..depth {
            // Create files at this level
            for file_idx in 0..files_per_level {
                File::create(current_dir.join(format!("f{file_idx}"))).unwrap();
            }

            // Create next level directory
            if level < depth - 1 {
                let next_dir = current_dir.join("d");
                fs::create_dir(&next_dir).unwrap();
                current_dir = next_dir;
            }
        }
    }

    /// Create a tree with mixed file types and permissions for comprehensive testing
    ///
    /// Creates files with different extensions, sizes, and permissions (on Unix).
    /// Useful for testing file type detection, permission handling, and formatting.
    pub fn create_mixed_tree(base_dir: &Path) {
        let extensions = ["txt", "log", "dat", "tmp", "bak", "cfg"];
        let sizes = [0, 100, 1024, 10240];

        for (i, ext) in extensions.iter().enumerate() {
            for (j, &size) in sizes.iter().enumerate() {
                let file_path = base_dir.join(format!("mixed_file_{i}_{j}.{ext}"));
                let mut file = File::create(&file_path).unwrap();

                if size > 0 {
                    let content = "x".repeat(size);
                    file.write_all(content.as_bytes()).unwrap();
                }

                // Set permissions only on Unix platforms
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = fs::Permissions::from_mode(match (i + j) % 4 {
                        0 => 0o644,
                        1 => 0o755,
                        2 => 0o600,
                        _ => 0o444,
                    });
                    fs::set_permissions(&file_path, perms).unwrap();
                }
            }
        }

        // Create some subdirectories
        for i in 0..5 {
            let dir_path = base_dir.join(format!("mixed_subdir_{i}"));
            fs::create_dir(&dir_path).unwrap();

            for j in 0..3 {
                let file_path = dir_path.join(format!("sub_file_{j}.txt"));
                let mut file = File::create(&file_path).unwrap();
                writeln!(file, "File {j} in subdir {i}").unwrap();
            }
        }
    }
}
